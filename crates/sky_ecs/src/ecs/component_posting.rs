use super::{ComponentType, InternalArchetype, MAX_COMPONENTS};
use rustc_hash::FxHashMap;

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
}

impl ComponentPostingList {
    fn push(&mut self, entry: ComponentPostingEntry) {
        debug_assert_eq!(self.data_indices.len(), self.column_indices.len());
        debug_assert!(self
            .data_indices
            .last()
            .is_none_or(|previous| *previous < entry.data_index));
        self.data_indices.push(entry.data_index);
        self.column_indices.push(entry.column_index);
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
        for (column_index, component) in archetype.components.iter().enumerate() {
            self.lists
                .entry(component.id())
                .or_default()
                .push(ComponentPostingEntry::new(data_index, column_index));
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
}
