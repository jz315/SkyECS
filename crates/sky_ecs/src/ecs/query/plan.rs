use super::*;

#[derive(Clone, Copy)]
pub(crate) struct ComponentIndexMap {
    indices: [u8; MAX_QUERY_COMPONENTS],
    len: u8,
}

impl ComponentIndexMap {
    #[inline]
    fn new(len: usize) -> Self {
        debug_assert!(len <= MAX_QUERY_COMPONENTS);
        Self {
            indices: [OPTIONAL_SENTINEL; MAX_QUERY_COMPONENTS],
            len: len as u8,
        }
    }

    #[inline]
    fn push(&mut self, index: u8) {
        let slot = self.len as usize;
        debug_assert!(slot < MAX_QUERY_COMPONENTS);
        self.indices[slot] = index;
        self.len += 1;
    }

    #[inline(always)]
    fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.indices[..self.len as usize]
    }
}

impl Deref for ComponentIndexMap {
    type Target = [u8];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.indices[..self.len as usize]
    }
}

#[derive(Clone, Copy)]
struct FilterTerm {
    ty: ComponentType,
    present: bool,
}

enum FilterPlan {
    Legacy,
    Always,
    Never,
    Terms(SmallVec<[FilterTerm; INLINE_QUERY_COMPONENTS]>),
}

impl FilterPlan {
    fn compile<Flt: QueryFilter>(descriptor: &QueryDescriptor) -> Self {
        let mut raw_terms = SmallVec::<[FilterTerm; MAX_QUERY_COMPONENTS]>::new();
        let complete = Flt::collect_conjunctive_terms(&mut |ty, present| {
            raw_terms.push(FilterTerm { ty, present });
        });
        if !complete {
            return Self::Legacy;
        }
        raw_terms.sort_unstable_by_key(|term| term.ty.id());

        let mut terms = SmallVec::<[FilterTerm; INLINE_QUERY_COMPONENTS]>::new();
        for FilterTerm { ty, present } in raw_terms {
            if descriptor
                .components
                .iter()
                .any(|component| component.ty.id() == ty.id() && !component.optional)
            {
                if present {
                    continue;
                }
                return Self::Never;
            }

            if let Some(previous) = terms.last() {
                if previous.ty.id() == ty.id() {
                    if previous.present != present {
                        return Self::Never;
                    }
                    continue;
                }
            }
            terms.push(FilterTerm { ty, present });
        }

        if terms.is_empty() {
            Self::Always
        } else {
            Self::Terms(terms)
        }
    }

    #[inline]
    fn matches<Flt: QueryFilter>(&self, archetype: &super::super::InternalArchetype) -> bool {
        match self {
            Self::Legacy => Flt::matches_archetype(archetype),
            Self::Always => true,
            Self::Never => false,
            Self::Terms(terms) if terms.len() <= 2 => terms
                .iter()
                .all(|term| archetype.query_component_index(&term.ty).is_some() == term.present),
            Self::Terms(terms) => {
                let binary_search_cost = terms.len().saturating_mul(
                    usize::BITS as usize - archetype.components.len().leading_zeros() as usize,
                );
                let merge_cost = archetype.components.len().saturating_add(terms.len());
                if binary_search_cost <= merge_cost {
                    Self::matches_with_suffix_binary_search(terms, archetype)
                } else {
                    Self::matches_with_merge(terms, archetype)
                }
            }
        }
    }

    #[inline]
    fn matches_with_suffix_binary_search(
        terms: &[FilterTerm],
        archetype: &super::super::InternalArchetype,
    ) -> bool {
        let mut search_start = 0;
        for term in terms {
            let target = term.ty.id();
            match archetype.components[search_start..]
                .binary_search_by_key(&target, |component| component.id())
            {
                Ok(relative_index) => {
                    if !term.present {
                        return false;
                    }
                    search_start += relative_index + 1;
                }
                Err(insertion_index) => {
                    if term.present {
                        return false;
                    }
                    search_start += insertion_index;
                }
            }
        }
        true
    }

    #[inline]
    fn matches_with_merge(
        terms: &[FilterTerm],
        archetype: &super::super::InternalArchetype,
    ) -> bool {
        let mut archetype_index = 0;
        for term in terms {
            let target = term.ty.id();
            while archetype_index < archetype.components.len()
                && archetype.components[archetype_index].id() < target
            {
                archetype_index += 1;
            }
            let found = archetype_index < archetype.components.len()
                && archetype.components[archetype_index].id() == target;
            if found != term.present {
                return false;
            }
            if found {
                archetype_index += 1;
            }
        }
        true
    }
}

#[derive(Clone, Copy)]
pub struct QueryComponent {
    pub(crate) ty: ComponentType,
    pub(crate) mutable: bool,
    pub(crate) optional: bool,
}

impl QueryComponent {
    pub(crate) fn new(ty: ComponentType, mutable: bool) -> Self {
        Self {
            ty,
            mutable,
            optional: false,
        }
    }

    pub(crate) fn optional(ty: ComponentType, mutable: bool) -> Self {
        Self {
            ty,
            mutable,
            optional: true,
        }
    }
}

pub struct QueryDescriptor {
    pub(crate) components: SmallVec<[QueryComponent; INLINE_QUERY_COMPONENTS]>,
    match_order: SmallVec<[MatchComponent; INLINE_QUERY_COMPONENTS]>,
}

#[derive(Clone, Copy)]
struct MatchComponent {
    component: QueryComponent,
    query_slot: u8,
}

impl QueryDescriptor {
    pub(crate) fn new(components: SmallVec<[QueryComponent; INLINE_QUERY_COMPONENTS]>) -> Self {
        assert!(
            components.len() <= MAX_QUERY_COMPONENTS,
            "typed queries support at most {MAX_QUERY_COMPONENTS} component parameters"
        );
        for (index, component) in components.iter().enumerate() {
            for other in &components[(index + 1)..] {
                if component.ty.id() == other.ty.id() {
                    let access_mode = if component.mutable || other.mutable {
                        "mutable"
                    } else {
                        "shared"
                    };
                    panic!(
                        "duplicate component type `{}` is not supported in {} queries",
                        component.ty.name, access_mode
                    );
                }
            }
        }

        let mut match_order = SmallVec::<[MatchComponent; INLINE_QUERY_COMPONENTS]>::new();
        if components.len() > 2 {
            match_order.extend(components.iter().copied().enumerate().map(
                |(query_slot, component)| MatchComponent {
                    component,
                    query_slot: query_slot as u8,
                },
            ));
            match_order.sort_unstable_by_key(|entry| entry.component.ty.id());
        }

        Self {
            components,
            match_order,
        }
    }

    #[inline]
    fn match_archetype(
        &self,
        archetype: &super::super::InternalArchetype,
    ) -> Option<ComponentIndexMap> {
        let query_len = self.match_order.len();
        debug_assert!(query_len > 2);
        let archetype_len = archetype.components.len();
        let mut indices = ComponentIndexMap::new(query_len);

        let binary_search_cost =
            query_len.saturating_mul(usize::BITS as usize - archetype_len.leading_zeros() as usize);
        let merge_cost = archetype_len.saturating_add(query_len);

        let matches = if binary_search_cost <= merge_cost {
            self.match_with_suffix_binary_search(archetype, indices.as_mut_slice())
        } else {
            self.match_with_merge(archetype, indices.as_mut_slice())
        };

        matches.then_some(indices)
    }

    #[inline]
    fn component_indices(
        &self,
        archetype: &super::super::InternalArchetype,
    ) -> Option<ComponentIndexMap> {
        if self.components.len() > 2 {
            return self.match_archetype(archetype);
        }

        let mut component_indices = ComponentIndexMap::new(0);
        for component in &self.components {
            if let Some(index) = archetype.query_component_index(&component.ty) {
                debug_assert!(index < OPTIONAL_SENTINEL as usize);
                component_indices.push(index as u8);
            } else if component.optional {
                component_indices.push(OPTIONAL_SENTINEL);
            } else {
                return None;
            }
        }
        Some(component_indices)
    }

    #[inline]
    fn match_with_suffix_binary_search(
        &self,
        archetype: &super::super::InternalArchetype,
        indices: &mut [u8],
    ) -> bool {
        let mut search_start = 0;
        for entry in &self.match_order {
            let target = entry.component.ty.id();
            match archetype.components[search_start..]
                .binary_search_by_key(&target, |component| component.id())
            {
                Ok(relative_index) => {
                    let index = search_start + relative_index;
                    debug_assert!(index < OPTIONAL_SENTINEL as usize);
                    indices[entry.query_slot as usize] = index as u8;
                    search_start = index + 1;
                }
                Err(insertion_index) => {
                    if !entry.component.optional {
                        return false;
                    }
                    search_start += insertion_index;
                }
            }
        }
        true
    }

    #[inline]
    fn match_with_merge(
        &self,
        archetype: &super::super::InternalArchetype,
        indices: &mut [u8],
    ) -> bool {
        let mut archetype_index = 0;
        for entry in &self.match_order {
            let target = entry.component.ty.id();
            while archetype_index < archetype.components.len()
                && archetype.components[archetype_index].id() < target
            {
                archetype_index += 1;
            }

            if archetype_index < archetype.components.len()
                && archetype.components[archetype_index].id() == target
            {
                debug_assert!(archetype_index < OPTIONAL_SENTINEL as usize);
                indices[entry.query_slot as usize] = archetype_index as u8;
                archetype_index += 1;
            } else if !entry.component.optional {
                return false;
            }
        }
        true
    }
}

#[derive(Clone)]
pub(crate) struct CachedArchetype {
    pub data_index: usize,
    pub component_indices: ComponentIndexMap,
}

struct PostingCursor<'w> {
    query_slot: Option<u8>,
    list: &'w ComponentPostingList,
    next: usize,
}

#[derive(Clone, Copy)]
struct PostingSeed {
    ty: ComponentType,
    query_slot: Option<u8>,
}

const BITMAP_WORD_BITS: usize = u64::BITS as usize;
// This conservative margin absorbs bitmap setup and column-map reconstruction;
// dense random-fragmentation workloads measure close to a 32x work advantage.
const BITMAP_MIN_WORK_ADVANTAGE: usize = 8;

impl PostingCursor<'_> {
    #[inline(always)]
    fn current(&self) -> Option<super::super::component_posting::ComponentPostingEntry> {
        self.list.entry(self.next)
    }

    #[inline(always)]
    fn remaining(&self) -> usize {
        self.list.len() - self.next
    }
}

#[inline(always)]
fn filter_matches<Flt: QueryFilter>(
    filter_plan: Option<&FilterPlan>,
    archetype: &super::super::InternalArchetype,
) -> bool {
    filter_plan.map_or_else(
        || Flt::matches_archetype(archetype),
        |plan| plan.matches::<Flt>(archetype),
    )
}

#[inline]
fn append_candidate<Flt: QueryFilter>(
    world: &World,
    descriptor: &QueryDescriptor,
    filter_plan: Option<&FilterPlan>,
    candidate_data_index: usize,
    mut component_indices: ComponentIndexMap,
    matches: &mut Vec<CachedArchetype>,
) {
    let archetype = world.data[candidate_data_index].archetype;
    if !filter_matches::<Flt>(filter_plan, &archetype) {
        return;
    }

    for (query_slot, component) in descriptor.components.iter().enumerate() {
        if component_indices.indices[query_slot] != OPTIONAL_SENTINEL {
            continue;
        }
        if let Some(index) = archetype.query_component_index(&component.ty) {
            debug_assert!(index < OPTIONAL_SENTINEL as usize);
            component_indices.indices[query_slot] = index as u8;
        } else if !component.optional {
            return;
        }
    }

    matches.push(CachedArchetype {
        data_index: candidate_data_index,
        component_indices,
    });
}

#[inline]
fn append_bitmap_candidate<Flt: QueryFilter>(
    world: &World,
    descriptor: &QueryDescriptor,
    filter_plan: Option<&FilterPlan>,
    candidate_data_index: usize,
    matches: &mut Vec<CachedArchetype>,
) {
    let archetype = world.data[candidate_data_index].archetype;
    if !filter_matches::<Flt>(filter_plan, &archetype) {
        return;
    }
    let Some(component_indices) = descriptor.component_indices(&archetype) else {
        return;
    };
    matches.push(CachedArchetype {
        data_index: candidate_data_index,
        component_indices,
    });
}

/// Uses dense component bitmaps only when their estimated input work is at
/// least eight times smaller than walking the remaining posting entries.
/// Returns `true` when the bitmap path handled the suffix, including when the
/// intersection is known to be empty.
fn try_append_bitmap_matches<Flt: QueryFilter>(
    world: &World,
    descriptor: &QueryDescriptor,
    filter_plan: Option<&FilterPlan>,
    scan_start: usize,
    cursors: &[PostingCursor<'_>],
    matches: &mut Vec<CachedArchetype>,
) -> bool {
    if cursors.len() < 2 {
        return false;
    }

    let mut bitmaps = SmallVec::<[&[u64]; MAX_QUERY_COMPONENTS]>::new();
    for cursor in cursors {
        let Some(bitmap) = cursor.list.archetype_bitmap() else {
            return false;
        };
        bitmaps.push(bitmap);
    }

    let word_start = scan_start / BITMAP_WORD_BITS;
    let word_end = bitmaps
        .iter()
        .map(|bitmap| bitmap.len())
        .min()
        .expect("multiple posting cursors must produce multiple bitmaps");
    if word_start >= word_end {
        return true;
    }

    let posting_work = cursors.iter().fold(0_usize, |work, cursor| {
        work.saturating_add(cursor.remaining())
    });
    let bitmap_work = (word_end - word_start).saturating_mul(bitmaps.len());
    if posting_work < bitmap_work.saturating_mul(BITMAP_MIN_WORK_ADVANTAGE) {
        return false;
    }

    let mut intersection = bitmaps[0][word_start..word_end].to_vec();
    let first_word_offset = scan_start % BITMAP_WORD_BITS;
    if first_word_offset != 0 {
        intersection[0] &= u64::MAX << first_word_offset;
    }
    for bitmap in &bitmaps[1..] {
        for (candidates, &component_word) in
            intersection.iter_mut().zip(&bitmap[word_start..word_end])
        {
            *candidates &= component_word;
        }
    }

    matches.reserve(
        intersection
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum(),
    );

    for (relative_word, mut candidates) in intersection.into_iter().enumerate() {
        while candidates != 0 {
            let bit = candidates.trailing_zeros() as usize;
            let candidate_data_index = (word_start + relative_word) * BITMAP_WORD_BITS + bit;
            append_bitmap_candidate::<Flt>(
                world,
                descriptor,
                filter_plan,
                candidate_data_index,
                matches,
            );
            candidates &= candidates - 1;
        }
    }
    true
}

/// Appends candidates from an intersection of required query components and
/// positive conjunctive filter terms. Returns `false` only when no positive
/// term exists and a full archetype scan is still required.
fn append_posting_matches<Flt: QueryFilter>(
    world: &World,
    descriptor: &QueryDescriptor,
    filter_plan: Option<&FilterPlan>,
    scan_start: usize,
    matches: &mut Vec<CachedArchetype>,
) -> bool {
    let mut seeds = SmallVec::<[PostingSeed; INLINE_QUERY_COMPONENTS]>::new();
    seeds.extend(
        descriptor
            .components
            .iter()
            .enumerate()
            .filter(|(_, component)| !component.optional)
            .map(|(query_slot, component)| PostingSeed {
                ty: component.ty,
                query_slot: Some(query_slot as u8),
            }),
    );
    if let Some(FilterPlan::Terms(terms)) = filter_plan {
        for term in terms.iter().filter(|term| term.present) {
            if !seeds.iter().any(|seed| seed.ty.id() == term.ty.id()) {
                let query_slot = descriptor
                    .components
                    .iter()
                    .position(|component| component.ty.id() == term.ty.id())
                    .map(|slot| slot as u8);
                seeds.push(PostingSeed {
                    ty: term.ty,
                    query_slot,
                });
            }
        }
    }

    if matches!(filter_plan, Some(FilterPlan::Never)) {
        return true;
    }
    if seeds.is_empty() {
        return false;
    }

    let mut cursors = SmallVec::<[PostingCursor<'_>; INLINE_QUERY_COMPONENTS]>::new();
    for seed in seeds {
        let Some(list) = world.component_posting(&seed.ty) else {
            return true;
        };
        cursors.push(PostingCursor {
            query_slot: seed.query_slot,
            list,
            next: list.first_at_or_after(scan_start),
        });
    }

    // Starting from the shortest remaining list makes the intersection jump
    // to selective candidates sooner without changing declaration-order maps.
    cursors.sort_unstable_by_key(PostingCursor::remaining);

    if try_append_bitmap_matches::<Flt>(
        world,
        descriptor,
        filter_plan,
        scan_start,
        &cursors,
        matches,
    ) {
        return true;
    }

    'intersection: loop {
        let mut candidate_data_index = 0usize;
        for cursor in &cursors {
            let Some(entry) = cursor.current() else {
                break 'intersection;
            };
            candidate_data_index = candidate_data_index.max(entry.data_index());
        }

        loop {
            let mut candidate_changed = false;
            for cursor in &mut cursors {
                while cursor
                    .current()
                    .is_some_and(|entry| entry.data_index() < candidate_data_index)
                {
                    cursor.next += 1;
                }

                let Some(entry) = cursor.current() else {
                    break 'intersection;
                };
                if entry.data_index() > candidate_data_index {
                    candidate_data_index = entry.data_index();
                    candidate_changed = true;
                }
            }
            if !candidate_changed {
                break;
            }
        }

        let mut component_indices = ComponentIndexMap::new(descriptor.components.len());
        for cursor in &mut cursors {
            let entry = cursor
                .current()
                .expect("posting intersection cursor must remain valid");
            debug_assert_eq!(entry.data_index(), candidate_data_index);
            if let Some(query_slot) = cursor.query_slot {
                component_indices.indices[query_slot as usize] = entry.column_index();
            }
            cursor.next += 1;
        }

        append_candidate::<Flt>(
            world,
            descriptor,
            filter_plan,
            candidate_data_index,
            component_indices,
            matches,
        );
    }

    true
}

#[derive(Default)]
pub(crate) struct PreparedCache {
    cached_world: Option<Arc<()>>,
    cached_archetype_epoch: Option<usize>,
    cached_active_storage_epoch: Option<u64>,
    scanned_data_len: usize,
    signature_matches: Vec<CachedArchetype>,
    pub archetypes: Arc<Vec<CachedArchetype>>,
    #[cfg(test)]
    active_refresh_count: u64,
}

impl PreparedCache {
    #[inline(always)]
    pub fn prepare<Flt: QueryFilter>(&mut self, world: &World, descriptor: &QueryDescriptor) {
        let same_world = self
            .cached_world
            .as_ref()
            .is_some_and(|cached| Arc::ptr_eq(cached, world.cache_token()));
        let current_epoch = world.archetype_epoch();
        let active_storage_epoch = world.active_storage_epoch();
        let signatures_changed = !same_world || self.cached_archetype_epoch != Some(current_epoch);

        if signatures_changed {
            let data_len = world.data.len();
            let filter_plan = (!Flt::IS_TRIVIAL && Flt::IS_CONJUNCTIVE)
                .then(|| FilterPlan::compile::<Flt>(descriptor));
            // A new archetype increments the epoch and appends exactly one
            // storage. Matching deltas permit an incremental suffix scan;
            // clear or world replacement forces a complete signature rebuild.
            let scan_start = match self.cached_archetype_epoch {
                Some(cached_epoch)
                    if same_world
                        && data_len >= self.scanned_data_len
                        && current_epoch.wrapping_sub(cached_epoch)
                            == data_len - self.scanned_data_len =>
                {
                    self.scanned_data_len
                }
                _ => 0,
            };

            if scan_start == 0 {
                self.signature_matches.clear();
            }

            if !append_posting_matches::<Flt>(
                world,
                descriptor,
                filter_plan.as_ref(),
                scan_start,
                &mut self.signature_matches,
            ) {
                for (data_index, data) in world.data.iter().enumerate().skip(scan_start) {
                    let archetype = data.archetype;
                    if !filter_matches::<Flt>(filter_plan.as_ref(), &archetype) {
                        continue;
                    }

                    if let Some(component_indices) = descriptor.component_indices(&archetype) {
                        self.signature_matches.push(CachedArchetype {
                            data_index,
                            component_indices,
                        });
                    }
                }
            }

            self.scanned_data_len = data_len;
            self.cached_archetype_epoch = Some(current_epoch);
        }

        if signatures_changed || self.cached_active_storage_epoch != Some(active_storage_epoch) {
            let active = Arc::make_mut(&mut self.archetypes);
            active.clear();
            active.extend(
                self.signature_matches
                    .iter()
                    .filter(|cached| !world.data[cached.data_index].chunks.is_empty())
                    .cloned(),
            );
            self.cached_active_storage_epoch = Some(active_storage_epoch);
            #[cfg(test)]
            {
                self.active_refresh_count = self.active_refresh_count.saturating_add(1);
            }
        }

        if !same_world {
            self.cached_world = Some(Arc::clone(world.cache_token()));
        }
    }

    #[inline(always)]
    pub fn visit_chunks<'w, F>(&self, world: &'w World, mut f: F)
    where
        F: FnMut(&CachedArchetype, &'w Chunk),
    {
        for cached in self.archetypes.iter() {
            let data = &world.data[cached.data_index];

            for chunk in &data.chunks {
                debug_assert!(chunk.entity_count != 0);
                f(cached, chunk);
            }
        }
    }

    pub fn cached_archetype_count(&self) -> usize {
        self.archetypes.len()
    }

    #[cfg(test)]
    pub(crate) fn active_refresh_count(&self) -> u64 {
        self.active_refresh_count
    }
}

#[inline(always)]
pub(crate) fn resolve_column_ptr(chunk: &Chunk, index: u8) -> *mut u8 {
    if index == OPTIONAL_SENTINEL {
        ptr::null_mut()
    } else {
        chunk.column_ptr(index as usize)
    }
}

#[cfg(test)]
mod layout_tests {
    use super::{ComponentIndexMap, MAX_QUERY_COMPONENTS};

    #[test]
    fn component_index_map_is_inline_fixed_capacity_storage() {
        assert_eq!(
            std::mem::size_of::<ComponentIndexMap>(),
            MAX_QUERY_COMPONENTS + 1
        );
        assert_eq!(std::mem::align_of::<ComponentIndexMap>(), 1);
        assert!(!std::mem::needs_drop::<ComponentIndexMap>());
    }
}
