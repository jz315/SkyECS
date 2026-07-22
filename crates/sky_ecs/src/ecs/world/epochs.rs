use super::*;

#[derive(Default)]
pub(super) struct StorageEpochs {
    chunk_set: u64,
    row_layout: u64,
}

/// Couples one ArchetypeStorage mutation to the World-wide chunk-set epoch.
/// The storage-local version is marked before any potentially invalidating
/// mutation, so Drop also propagates changes when user iteration unwinds.
pub(super) struct ChunkSetEpochGuard<'a> {
    storage: &'a mut ArchetypeStorage,
    epochs: &'a mut StorageEpochs,
    initial_version: u64,
    next_chunk_set_epoch: u64,
}

impl<'a> ChunkSetEpochGuard<'a> {
    pub(super) fn new(storage: &'a mut ArchetypeStorage, epochs: &'a mut StorageEpochs) -> Self {
        let initial_version = storage.chunk_set_version();
        let next_chunk_set_epoch = epochs
            .chunk_set
            .checked_add(1)
            .expect("world chunk-set epoch exhausted");
        Self {
            storage,
            epochs,
            initial_version,
            next_chunk_set_epoch,
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
    pub(crate) fn chunk_set_epoch(&self) -> u64 {
        self.storage_epochs.chunk_set
    }

    #[inline(always)]
    pub(crate) fn row_layout_epoch(&self) -> u64 {
        self.storage_epochs.row_layout
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
