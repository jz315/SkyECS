use super::*;

#[derive(Default)]
pub(super) struct StorageEpochs {
    chunk_set: u64,
    column_base: u64,
    row_layout: u64,
    active_storage: u64,
}

/// Couples one ArchetypeStorage mutation to the World-wide chunk-set epoch.
/// The storage-local version is marked before any potentially invalidating
/// mutation, so Drop also propagates changes when user iteration unwinds.
pub(super) struct ChunkSetEpochGuard<'a> {
    storage: &'a mut ArchetypeStorage,
    epochs: &'a mut StorageEpochs,
    initial_version: u64,
    initial_column_base_version: u64,
    initial_active: bool,
    next_chunk_set_epoch: u64,
    next_column_base_epoch: u64,
    next_active_storage_epoch: u64,
}

impl<'a> ChunkSetEpochGuard<'a> {
    pub(super) fn new(storage: &'a mut ArchetypeStorage, epochs: &'a mut StorageEpochs) -> Self {
        let initial_version = storage.chunk_set_version();
        let initial_column_base_version = storage.column_base_version();
        let initial_active = !storage.chunks.is_empty();
        let next_chunk_set_epoch = epochs
            .chunk_set
            .checked_add(1)
            .expect("world chunk-set epoch exhausted");
        let next_column_base_epoch = epochs
            .column_base
            .checked_add(1)
            .expect("world column-base epoch exhausted");
        let next_active_storage_epoch = epochs
            .active_storage
            .checked_add(1)
            .expect("world active-storage epoch exhausted");
        Self {
            storage,
            epochs,
            initial_version,
            initial_column_base_version,
            initial_active,
            next_chunk_set_epoch,
            next_column_base_epoch,
            next_active_storage_epoch,
        }
    }

    #[inline(always)]
    pub(super) fn storage_mut(&mut self) -> &mut ArchetypeStorage {
        self.storage
    }
}

impl Drop for ChunkSetEpochGuard<'_> {
    fn drop(&mut self) {
        if self.storage.chunk_set_version() != self.initial_version {
            self.epochs.chunk_set = self.next_chunk_set_epoch;
        }
        if self.storage.column_base_version() != self.initial_column_base_version {
            self.epochs.column_base = self.next_column_base_epoch;
        }
        let active_now = !self.storage.chunks.is_empty();
        if active_now != self.initial_active {
            self.epochs.active_storage = self.next_active_storage_epoch;
        }
    }
}

impl World {
    #[inline(always)]
    pub(super) fn bump_row_layout_epoch(&mut self) {
        self.storage_epochs.row_layout = self
            .storage_epochs
            .row_layout
            .checked_add(1)
            .expect("world row-layout epoch exhausted");
    }

    #[inline(always)]
    pub(super) fn bump_chunk_set_epoch(&mut self) {
        self.storage_epochs.chunk_set = self
            .storage_epochs
            .chunk_set
            .checked_add(1)
            .expect("world chunk-set epoch exhausted");
    }

    #[inline(always)]
    pub(super) fn bump_column_base_epoch(&mut self) {
        self.storage_epochs.column_base = self
            .storage_epochs
            .column_base
            .checked_add(1)
            .expect("world column-base epoch exhausted");
    }

    #[inline(always)]
    pub(super) fn bump_active_storage_epoch(&mut self) {
        self.storage_epochs.active_storage = self
            .storage_epochs
            .active_storage
            .checked_add(1)
            .expect("world active-storage epoch exhausted");
    }

    #[inline(always)]
    pub(crate) fn chunk_set_epoch(&self) -> u64 {
        self.storage_epochs.chunk_set
    }

    #[inline(always)]
    pub(crate) fn row_layout_epoch(&self) -> u64 {
        self.storage_epochs.row_layout
    }

    #[inline(always)]
    pub(crate) fn column_base_epoch(&self) -> u64 {
        self.storage_epochs.column_base
    }

    #[inline(always)]
    pub(crate) fn active_storage_epoch(&self) -> u64 {
        self.storage_epochs.active_storage
    }
}

#[cfg(test)]
mod tests {
    use super::World;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    #[derive(Clone, Copy)]
    struct Position;

    #[test]
    fn chunk_set_epoch_ignores_in_chunk_row_churn() {
        let mut world = World::new();
        assert_eq!(world.chunk_set_epoch(), 0);

        let first = world.spawn((Position,));
        assert_eq!(world.chunk_set_epoch(), 1);

        let second = world.spawn((Position,));
        assert_eq!(world.chunk_set_epoch(), 1);

        assert!(world.despawn(second));
        assert_eq!(world.chunk_set_epoch(), 1);

        assert!(world.despawn(first));
        assert_eq!(world.chunk_set_epoch(), 2);

        world.clear();
        assert_eq!(world.chunk_set_epoch(), 3);
    }

    #[test]
    fn column_base_epoch_ignores_rows_but_tracks_backing_changes() {
        let mut world = World::new();
        assert_eq!(world.column_base_epoch(), 0);
        let first = world.spawn((Position,));
        let after_first = world.column_base_epoch();
        assert_eq!(after_first, 1);
        let second = world.spawn((Position,));
        assert_eq!(world.column_base_epoch(), after_first);
        assert!(world.despawn(second));
        assert_eq!(world.column_base_epoch(), after_first);
        assert!(world.despawn(first));
        assert_eq!(world.column_base_epoch(), after_first + 1);
        world.clear();
        assert_eq!(world.column_base_epoch(), after_first + 2);
    }

    #[test]
    fn active_storage_epoch_tracks_only_empty_to_nonempty_transitions() {
        let mut world = World::new();
        assert_eq!(world.active_storage_epoch(), 0);

        let first = world.spawn((Position,));
        assert_eq!(world.active_storage_epoch(), 1);
        let second = world.spawn((Position,));
        assert_eq!(world.active_storage_epoch(), 1);

        assert!(world.despawn(second));
        assert_eq!(world.active_storage_epoch(), 1);
        assert!(world.despawn(first));
        assert_eq!(world.active_storage_epoch(), 2);

        let reactivated = world.spawn((Position,));
        assert_eq!(world.active_storage_epoch(), 3);
        assert!(world.despawn(reactivated));
        assert_eq!(world.active_storage_epoch(), 4);

        world.clear();
        assert_eq!(world.active_storage_epoch(), 5);
    }

    #[test]
    fn chunk_set_epoch_is_committed_when_batch_iteration_panics() {
        let mut world = World::new();
        let mut iteration = 0;

        let panic = catch_unwind(AssertUnwindSafe(|| {
            world.spawn_batch(std::iter::from_fn(|| {
                iteration += 1;
                match iteration {
                    1 => Some((Position,)),
                    2 => panic!("batch iterator panic"),
                    _ => None,
                }
            }));
        }));

        assert!(panic.is_err());
        assert_eq!(world.entity_count(), 1);
        assert_eq!(world.chunk_set_epoch(), 1);
    }
}
