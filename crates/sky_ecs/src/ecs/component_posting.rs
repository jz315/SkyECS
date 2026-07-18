use super::{ComponentType, InternalArchetype, MAX_COMPONENTS};
use rustc_hash::FxHashMap;

const BITMAP_WORD_BITS: usize = u64::BITS as usize;
const BITMAP_MIN_ARCHETYPE_COUNT: usize = 256;
// A promoted bitmap adds at most 10% of its posting payload at 25% density
// (one bit per archetype versus one u32 index plus one u8 column per hit).
// Retaining it down to 12.5% caps that ratio at 20% and avoids threshold churn.
const BITMAP_PROMOTION_DENSITY_DIVISOR: usize = 4;
const BITMAP_RETENTION_DENSITY_DIVISOR: usize = 8;

/// One component occurrence in one World-local archetype storage.
///
/// Archetype-storage indices are appended in World creation order, so every posting list
/// remains sorted without a separate sort pass. The column index is stored
/// alongside it so query preparation does not need to rediscover the column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ComponentPostingEntry {
    data_index: u32,
    column_index: u8,
}

impl ComponentPostingEntry {
    fn new(data_index: usize, column_index: usize) -> Self {
        let data_index = u32::try_from(data_index)
            .expect("a World cannot contain more than u32::MAX archetype storages");
        debug_assert!(column_index < MAX_COMPONENTS);
        Self {
            data_index,
            column_index: column_index as u8,
        }
    }

    #[inline(always)]
    pub(crate) fn data_index(self) -> usize {
        self.data_index as usize
    }

    #[inline(always)]
    pub(crate) fn column_index(self) -> u8 {
        self.column_index
    }
}

/// Sorted occurrences of one component across a World's archetype storages.
///
/// The two columns avoid a padded per-entry struct while keeping their
/// relationship explicit behind this type's API.
#[derive(Default)]
pub(crate) struct ComponentPostingList {
    data_indices: Vec<u32>,
    column_indices: Vec<u8>,
    archetype_bitmap: Option<Vec<u64>>,
}

impl ComponentPostingList {
    fn push(&mut self, entry: ComponentPostingEntry, archetype_count: usize) {
        debug_assert_eq!(self.data_indices.len(), self.column_indices.len());
        debug_assert!(self
            .data_indices
            .last()
            .is_none_or(|previous| *previous < entry.data_index));
        self.data_indices.push(entry.data_index);
        self.column_indices.push(entry.column_index);
        self.update_archetype_bitmap(entry.data_index(), archetype_count);
    }

    fn update_archetype_bitmap(&mut self, data_index: usize, archetype_count: usize) {
        debug_assert_eq!(data_index.checked_add(1), Some(archetype_count));
        let posting_count = self.data_indices.len();

        if self.archetype_bitmap.is_some() {
            let minimum_count = archetype_count.div_ceil(BITMAP_RETENTION_DENSITY_DIVISOR);
            if posting_count < minimum_count {
                self.archetype_bitmap = None;
                return;
            }

            let words = self
                .archetype_bitmap
                .as_mut()
                .expect("the bitmap presence check must remain valid");
            Self::set_bitmap_bit(words, data_index);
            return;
        }

        if archetype_count < BITMAP_MIN_ARCHETYPE_COUNT
            || posting_count < archetype_count.div_ceil(BITMAP_PROMOTION_DENSITY_DIVISOR)
        {
            return;
        }

        let mut words = vec![0_u64; data_index / BITMAP_WORD_BITS + 1];
        for &posting_data_index in &self.data_indices {
            Self::set_bitmap_bit(&mut words, posting_data_index as usize);
        }
        self.archetype_bitmap = Some(words);
    }

    #[inline]
    fn set_bitmap_bit(words: &mut Vec<u64>, data_index: usize) {
        let word_index = data_index / BITMAP_WORD_BITS;
        if word_index >= words.len() {
            words.resize(word_index + 1, 0);
        }
        words[word_index] |= 1_u64 << (data_index % BITMAP_WORD_BITS);
    }

    #[inline(always)]
    pub(crate) fn len(&self) -> usize {
        debug_assert_eq!(self.data_indices.len(), self.column_indices.len());
        self.data_indices.len()
    }

    #[inline(always)]
    pub(crate) fn entry(&self, index: usize) -> Option<ComponentPostingEntry> {
        let data_index = *self.data_indices.get(index)?;
        let column_index = *self
            .column_indices
            .get(index)
            .expect("posting columns must stay aligned");
        Some(ComponentPostingEntry {
            data_index,
            column_index,
        })
    }

    #[inline]
    pub(crate) fn first_at_or_after(&self, data_index: usize) -> usize {
        let Ok(data_index) = u32::try_from(data_index) else {
            return self.len();
        };
        self.data_indices
            .partition_point(|candidate| *candidate < data_index)
    }

    #[inline(always)]
    pub(crate) fn archetype_bitmap(&self) -> Option<&[u64]> {
        self.archetype_bitmap.as_deref()
    }
}

/// World-local inverted index from component type to archetype storage.
#[derive(Default)]
pub(crate) struct ComponentPostingIndex {
    lists: FxHashMap<usize, ComponentPostingList>,
}

impl ComponentPostingIndex {
    pub(crate) fn append_archetype(&mut self, data_index: usize, archetype: &InternalArchetype) {
        // Check the representable limit before mutating any posting list.
        let _ = u32::try_from(data_index)
            .expect("a World cannot contain more than u32::MAX archetype storages");
        let archetype_count = data_index
            .checked_add(1)
            .expect("world archetype count exhausted");
        for (column_index, component) in archetype.components.iter().enumerate() {
            self.lists.entry(component.id()).or_default().push(
                ComponentPostingEntry::new(data_index, column_index),
                archetype_count,
            );
        }
    }

    #[inline(always)]
    pub(crate) fn list(&self, component: &ComponentType) -> Option<&ComponentPostingList> {
        self.lists.get(&component.id())
    }

    pub(crate) fn clear(&mut self) {
        self.lists.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::{component_type, create_archetype};

    #[derive(Clone, Copy)]
    struct A;
    #[derive(Clone, Copy)]
    struct B;

    #[test]
    fn postings_preserve_data_order_and_archetype_columns() {
        let only_a = create_archetype().add_rust_component::<A>().build();
        let a_and_b = create_archetype()
            .add_rust_component::<A>()
            .add_rust_component::<B>()
            .build();
        let mut postings = ComponentPostingIndex::default();
        postings.append_archetype(0, &only_a);
        postings.append_archetype(1, &a_and_b);

        let a = component_type::<A>();
        let a_list = postings.list(&a).unwrap();
        assert_eq!(a_list.first_at_or_after(0), 0);
        assert_eq!(a_list.first_at_or_after(1), 1);
        assert_eq!(a_list.len(), 2);
        for posting_index in 0..a_list.len() {
            let entry = a_list.entry(posting_index).unwrap();
            let archetype = [only_a, a_and_b][entry.data_index()];
            assert_eq!(
                archetype.query_component_index(&a),
                Some(entry.column_index() as usize)
            );
        }

        let b = component_type::<B>();
        let b_entry = postings.list(&b).unwrap().entry(0).unwrap();
        assert_eq!(b_entry.data_index(), 1);
        assert_eq!(
            a_and_b.query_component_index(&b),
            Some(b_entry.column_index() as usize)
        );
    }

    #[test]
    fn clearing_postings_removes_every_component_list() {
        let archetype = create_archetype().add_rust_component::<A>().build();
        let mut postings = ComponentPostingIndex::default();
        postings.append_archetype(0, &archetype);
        postings.clear();

        assert!(postings.list(&component_type::<A>()).is_none());
    }

    #[test]
    fn dense_posting_promotes_to_an_exact_archetype_bitmap() {
        let empty = create_archetype().build();
        let only_a = create_archetype().add_rust_component::<A>().build();
        let mut postings = ComponentPostingIndex::default();

        for data_index in 0..192 {
            postings.append_archetype(data_index, &empty);
        }
        for data_index in 192..256 {
            postings.append_archetype(data_index, &only_a);
        }

        let bitmap = postings
            .list(&component_type::<A>())
            .unwrap()
            .archetype_bitmap()
            .expect("a 25%-dense posting should have a bitmap");
        assert_eq!(bitmap, &[0, 0, 0, u64::MAX]);
    }

    #[test]
    fn sparse_posting_does_not_allocate_a_full_bitmap() {
        let empty = create_archetype().build();
        let only_a = create_archetype().add_rust_component::<A>().build();
        let mut postings = ComponentPostingIndex::default();

        postings.append_archetype(0, &only_a);
        for data_index in 1..256 {
            postings.append_archetype(data_index, &empty);
        }
        postings.append_archetype(256, &only_a);

        assert!(postings
            .list(&component_type::<A>())
            .unwrap()
            .archetype_bitmap()
            .is_none());
    }

    #[test]
    fn bitmap_is_dropped_before_a_sparse_late_occurrence_extends_it() {
        let empty = create_archetype().build();
        let only_a = create_archetype().add_rust_component::<A>().build();
        let mut postings = ComponentPostingIndex::default();

        for data_index in 0..256 {
            postings.append_archetype(data_index, &only_a);
        }
        assert_eq!(
            postings
                .list(&component_type::<A>())
                .unwrap()
                .archetype_bitmap()
                .unwrap()
                .len(),
            4
        );

        for data_index in 256..2_056 {
            postings.append_archetype(data_index, &empty);
        }
        postings.append_archetype(2_056, &only_a);

        let list = postings.list(&component_type::<A>()).unwrap();
        assert_eq!(list.len(), 257);
        assert!(list.archetype_bitmap().is_none());
    }
}
