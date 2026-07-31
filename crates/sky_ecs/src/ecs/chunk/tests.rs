use super::pool::{
    ChunkBlock, ChunkBlockPool, BACKING_POOL_BUDGET_BYTES, BACKING_POOL_SIZE_TIERS,
    CHUNK_BLOCK_POOL, ENTITY_BUFFER_NON_ZST_MAX_OVERSIZE, ENTITY_BUFFER_POOL_BUDGET_BYTES,
    ENTITY_BUFFER_POOL_MAX_ENTRIES, ENTITY_BUFFER_ZST_MAX_OVERSIZE,
};
use super::{
    Chunk, CHUNK_SIZE, MAX_CHUNK_SIZE, MEDIUM_CHUNK_SIZE, REPEATED_TIER_START, SMALL_CHUNK_SIZE,
    TINY_CHUNK_SIZE,
};
use crate::ecs::{create_archetype, register_component_type};
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering as AtomicOrdering};

#[repr(align(16))]
#[allow(dead_code)]
struct Aligned16([u8; 16]);

#[repr(align(8))]
#[allow(dead_code)]
struct Aligned8([u8; 8]);

#[repr(align(32))]
#[allow(dead_code)]
struct Aligned32([u8; 32]);

#[allow(dead_code)]
#[derive(Clone, Copy)]
struct Tiny(u32);

#[derive(Clone, Copy)]
struct Marker;

#[repr(align(64))]
#[derive(Clone, Copy)]
struct AlignedMarker;

static DROPPABLE_MARKER_DROPS: AtomicUsize = AtomicUsize::new(0);

struct DroppableMarker;

impl Drop for DroppableMarker {
    fn drop(&mut self) {
        DROPPABLE_MARKER_DROPS.fetch_add(1, AtomicOrdering::Relaxed);
    }
}

static PANIC_DROPPABLE_MARKER: AtomicBool = AtomicBool::new(false);

struct PanicDroppableMarker;

impl Drop for PanicDroppableMarker {
    fn drop(&mut self) {
        if PANIC_DROPPABLE_MARKER.swap(false, AtomicOrdering::Relaxed) {
            panic!("PanicDroppableMarker");
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
struct Large([u8; 1024]);

#[repr(align(16))]
#[allow(dead_code)]
#[derive(Clone, Copy)]
struct AlignedTiny(u32);

#[repr(align(16))]
#[allow(dead_code)]
#[derive(Clone, Copy)]
struct AlignedLarge([u8; 1024]);

#[allow(dead_code)]
#[derive(Clone, Copy)]
struct Huge([u8; 16 * 1024]);

#[allow(dead_code)]
struct Oversized([u8; MAX_CHUNK_SIZE + 4096]);

struct PoolGuard;

static POOL_WAS_UNAVAILABLE_DURING_THREAD_EXIT_STORAGE_DROP: AtomicBool = AtomicBool::new(false);

struct ThreadExitStorage(Option<super::ArchetypeStorage>);

impl Drop for ThreadExitStorage {
    fn drop(&mut self) {
        let pool_was_unavailable = CHUNK_BLOCK_POOL.try_with(|_| ()).is_err();
        POOL_WAS_UNAVAILABLE_DURING_THREAD_EXIT_STORAGE_DROP
            .store(pool_was_unavailable, AtomicOrdering::SeqCst);

        // Exercise Chunk's owned fallback after the pool TLS destructor has
        // already run. This must release both allocations without touching TLS.
        drop(self.0.take());
    }
}

thread_local! {
    static THREAD_EXIT_STORAGE: RefCell<ThreadExitStorage> =
        const { RefCell::new(ThreadExitStorage(None)) };
}

impl PoolGuard {
    fn new() -> Self {
        CHUNK_BLOCK_POOL.with(|pool| pool.borrow_mut().clear());
        Self
    }
}

impl Drop for PoolGuard {
    fn drop(&mut self) {
        CHUNK_BLOCK_POOL.with(|pool| pool.borrow_mut().clear());
    }
}

fn pool_stats() -> (usize, usize) {
    CHUNK_BLOCK_POOL.with(|pool| {
        let pool = pool.borrow();
        (
            pool.block_buckets.iter().map(|bucket| bucket.len()).sum(),
            pool.retained_bytes,
        )
    })
}

fn entity_pool_stats() -> (usize, usize) {
    CHUNK_BLOCK_POOL.with(|pool| {
        let pool = pool.borrow();
        (pool.entity_buffers.len(), pool.retained_entity_bytes)
    })
}

fn pooled_ptrs() -> Vec<usize> {
    CHUNK_BLOCK_POOL.with(|pool| {
        pool.borrow()
            .block_buckets
            .iter()
            .flatten()
            .map(|entry| entry.block.data.as_ptr() as usize)
            .collect()
    })
}

fn test_entity(index: usize) -> crate::ecs::EntityId {
    crate::ecs::EntityId::new(index as u32, 0)
}

fn backing_entry_count(pool: &ChunkBlockPool) -> usize {
    pool.block_buckets.iter().map(|bucket| bucket.len()).sum()
}

#[test]
fn backing_pool_reuses_all_eight_exact_size_tiers() {
    for size in BACKING_POOL_SIZE_TIERS {
        let mut pool = ChunkBlockPool::default();
        let block = ChunkBlock::fresh(size, 8);
        let ptr = block.data.as_ptr();

        pool.release(block);
        assert_eq!(backing_entry_count(&pool), 1);
        assert_eq!(pool.retained_bytes, size);

        let acquired = pool.acquire(size, 8).unwrap();
        assert_eq!(acquired.data.as_ptr(), ptr);
        assert_eq!(acquired.size(), size);
        assert_eq!(backing_entry_count(&pool), 0);
        assert_eq!(pool.retained_bytes, 0);
    }

    assert!(BACKING_POOL_SIZE_TIERS.contains(&CHUNK_SIZE));
}

#[test]
fn backing_pool_never_reuses_a_different_size_tier() {
    let mut pool = ChunkBlockPool::default();
    let block = ChunkBlock::fresh(CHUNK_SIZE, 8);
    let ptr = block.data.as_ptr();
    pool.release(block);

    assert!(pool.acquire(SMALL_CHUNK_SIZE, 8).is_none());
    assert_eq!(backing_entry_count(&pool), 1);

    let acquired = pool.acquire(CHUNK_SIZE, 8).unwrap();
    assert_eq!(acquired.data.as_ptr(), ptr);
}

#[test]
fn backing_pool_uses_best_fit_alignment_and_preserves_compatibility() {
    let mut pool = ChunkBlockPool::default();
    let low = ChunkBlock::fresh(TINY_CHUNK_SIZE, 8);
    let low_ptr = low.data.as_ptr();
    let high = ChunkBlock::fresh(TINY_CHUNK_SIZE, 32);
    let high_ptr = high.data.as_ptr();

    // Put the high-alignment block at the tail so the fast path cannot
    // accidentally consume it while an exact lower-alignment fit exists.
    pool.release(low);
    pool.release(high);

    let exact = pool.acquire(TINY_CHUNK_SIZE, 8).unwrap();
    assert_eq!(exact.data.as_ptr(), low_ptr);
    assert_eq!(exact.alignment(), 8);

    let compatible = pool.acquire(TINY_CHUNK_SIZE, 16).unwrap();
    assert_eq!(compatible.data.as_ptr(), high_ptr);
    assert_eq!(compatible.alignment(), 32);

    pool.release(exact);
    assert!(pool.acquire(TINY_CHUNK_SIZE, 16).is_none());
    assert_eq!(backing_entry_count(&pool), 1);
}

#[test]
fn backing_pool_lru_compares_oldest_heads_across_size_tiers() {
    let mut pool = ChunkBlockPool::default();
    let sizes = [
        REPEATED_TIER_START,
        1024 * 1024,
        CHUNK_SIZE,
        1024 * 1024,
        1024 * 1024,
        REPEATED_TIER_START,
    ];
    assert_eq!(sizes.iter().sum::<usize>(), BACKING_POOL_BUDGET_BYTES);

    let mut ptrs = Vec::new();
    for size in sizes {
        let block = ChunkBlock::fresh(size, 8);
        ptrs.push(block.data.as_ptr() as usize);
        pool.release(block);
    }

    let newcomer = ChunkBlock::fresh(TINY_CHUNK_SIZE, 8);
    let newcomer_ptr = newcomer.data.as_ptr() as usize;
    pool.release(newcomer);

    let pooled: Vec<_> = pool
        .block_buckets
        .iter()
        .flatten()
        .map(|entry| entry.block.data.as_ptr() as usize)
        .collect();
    assert!(!pooled.contains(&ptrs[0]));
    assert!(ptrs[1..].iter().all(|ptr| pooled.contains(ptr)));
    assert!(pooled.contains(&newcomer_ptr));
    assert!(pool.retained_bytes <= BACKING_POOL_BUDGET_BYTES);
}

#[test]
fn backing_pool_large_release_evicts_4096_tiny_blocks_within_budget() {
    let mut pool = ChunkBlockPool::default();
    let tiny_count = BACKING_POOL_BUDGET_BYTES / TINY_CHUNK_SIZE;
    for _ in 0..tiny_count {
        pool.release(ChunkBlock::fresh(TINY_CHUNK_SIZE, 8));
    }
    assert_eq!(backing_entry_count(&pool), tiny_count);
    assert_eq!(pool.retained_bytes, BACKING_POOL_BUDGET_BYTES);

    let large = ChunkBlock::fresh(MAX_CHUNK_SIZE, 8);
    let large_ptr = large.data.as_ptr();
    pool.release(large);

    assert_eq!(backing_entry_count(&pool), 1);
    assert_eq!(pool.retained_bytes, BACKING_POOL_BUDGET_BYTES);
    let retained = pool.acquire(MAX_CHUNK_SIZE, 8).unwrap();
    assert_eq!(retained.data.as_ptr(), large_ptr);
    assert!(pool.acquire(TINY_CHUNK_SIZE, 8).is_none());
}

#[test]
fn backing_pool_clear_resets_every_size_bucket() {
    let mut pool = ChunkBlockPool::default();
    for size in BACKING_POOL_SIZE_TIERS {
        pool.release(ChunkBlock::fresh(size, 8));
    }
    assert!(backing_entry_count(&pool) > 0);

    pool.clear();

    assert!(pool.block_buckets.iter().all(|bucket| bucket.is_empty()));
    assert_eq!(backing_entry_count(&pool), 0);
    assert_eq!(pool.retained_bytes, 0);
    assert_eq!(pool.clock, 0);
}

#[test]
fn entity_id_buffer_is_reused_independently_from_backing_blocks() {
    let _guard = PoolGuard::new();

    let archetype = create_archetype().add_rust_component::<Tiny>().build();
    let (first_ptr, first_capacity) = {
        let mut data = super::ArchetypeStorage::new(archetype);
        let location = unsafe { data.add_entity(test_entity(0)) };
        let ptr = data.chunks[0].entities.as_ptr();
        let capacity = data.chunks[0].entities.capacity();
        data.remove_entity(location);
        (ptr, capacity)
    };

    assert_eq!(pool_stats(), (1, TINY_CHUNK_SIZE));
    assert_eq!(
        entity_pool_stats(),
        (
            1,
            first_capacity * std::mem::size_of::<crate::ecs::EntityId>()
        )
    );

    let mut data = super::ArchetypeStorage::new(archetype);
    let location = unsafe { data.add_entity(test_entity(1)) };
    assert_eq!(data.chunks[0].entities.as_ptr(), first_ptr);
    assert_eq!(data.chunks[0].entities.capacity(), first_capacity);
    assert_eq!(
        data.chunks[0].entity_id(location.entity_index),
        Some(test_entity(1))
    );
    assert_eq!(entity_pool_stats(), (0, 0));
}

#[test]
fn entity_buffer_pool_uses_best_fit_and_enforces_oversize_limits() {
    let _guard = PoolGuard::new();

    let smaller = Vec::with_capacity(64);
    let smaller_capacity = smaller.capacity();
    let larger = Vec::with_capacity(128);
    CHUNK_BLOCK_POOL.with(|pool| {
        let mut pool = pool.borrow_mut();
        pool.release_entities(larger);
        pool.release_entities(smaller);
        let acquired = pool
            .acquire_entities(48, false, ENTITY_BUFFER_NON_ZST_MAX_OVERSIZE)
            .unwrap();
        assert_eq!(acquired.capacity(), smaller_capacity);
    });

    CHUNK_BLOCK_POOL.with(|pool| pool.borrow_mut().clear());
    let too_large = Vec::with_capacity(256);
    let too_large_capacity = too_large.capacity();
    CHUNK_BLOCK_POOL.with(|pool| {
        let mut pool = pool.borrow_mut();
        pool.release_entities(too_large);
        assert!(pool
            .acquire_entities(100, false, ENTITY_BUFFER_NON_ZST_MAX_OVERSIZE)
            .is_none());
        let acquired = pool
            .acquire_entities(64, true, ENTITY_BUFFER_ZST_MAX_OVERSIZE)
            .unwrap();
        assert_eq!(acquired.capacity(), too_large_capacity);
    });

    CHUNK_BLOCK_POOL.with(|pool| pool.borrow_mut().clear());
    let undersized = Vec::with_capacity(32);
    let undersized_capacity = undersized.capacity();
    CHUNK_BLOCK_POOL.with(|pool| {
        let mut pool = pool.borrow_mut();
        pool.release_entities(undersized);
        let acquired = pool
            .acquire_entities(64, true, ENTITY_BUFFER_ZST_MAX_OVERSIZE)
            .unwrap();
        assert_eq!(acquired.capacity(), undersized_capacity);
    });
}

#[test]
fn entity_buffer_pool_has_independent_byte_and_entry_budgets() {
    let _guard = PoolGuard::new();

    let oversized_capacity =
        ENTITY_BUFFER_POOL_BUDGET_BYTES / std::mem::size_of::<crate::ecs::EntityId>() + 1;
    CHUNK_BLOCK_POOL.with(|pool| {
        pool.borrow_mut()
            .release_entities(Vec::with_capacity(oversized_capacity));
    });
    assert_eq!(entity_pool_stats(), (0, 0));

    CHUNK_BLOCK_POOL.with(|pool| {
        let mut pool = pool.borrow_mut();
        for _ in 0..=ENTITY_BUFFER_POOL_MAX_ENTRIES {
            pool.release_entities(Vec::with_capacity(4));
        }
    });
    let (entry_count, retained_bytes) = entity_pool_stats();
    assert_eq!(entry_count, ENTITY_BUFFER_POOL_MAX_ENTRIES);
    assert!(retained_bytes <= ENTITY_BUFFER_POOL_BUDGET_BYTES);
    assert_eq!(pool_stats(), (0, 0));
}

#[test]
fn blockless_zst_reuses_an_existing_entity_buffer_without_eager_allocation() {
    let _guard = PoolGuard::new();

    let archetype = create_archetype().add_rust_component::<Marker>().build();
    let (first_ptr, first_capacity) = {
        let mut data = super::ArchetypeStorage::new(archetype);
        for index in 0..32 {
            unsafe {
                data.add_entity(test_entity(index));
            }
        }
        (
            data.chunks[0].entities.as_ptr(),
            data.chunks[0].entities.capacity(),
        )
    };

    assert_eq!(pool_stats(), (0, 0));
    assert_eq!(
        entity_pool_stats(),
        (
            1,
            first_capacity * std::mem::size_of::<crate::ecs::EntityId>()
        )
    );

    let mut data = super::ArchetypeStorage::new(archetype);
    let location = unsafe { data.add_entity(test_entity(99)) };
    assert_eq!(data.chunks[0].entities.as_ptr(), first_ptr);
    assert_eq!(data.chunks[0].entities.capacity(), first_capacity);
    assert_eq!(
        data.chunks[0].entity_id(location.entity_index),
        Some(test_entity(99))
    );
    assert_eq!(pool_stats(), (0, 0));
}

#[test]
fn panicking_component_drop_releases_storage_before_resuming() {
    let _guard = PoolGuard::new();
    PANIC_DROPPABLE_MARKER.store(true, AtomicOrdering::Relaxed);

    let archetype = create_archetype()
        .add_rust_component::<PanicDroppableMarker>()
        .build();
    let component_index = archetype
        .query_component_index(&crate::ecs::component_type::<PanicDroppableMarker>())
        .unwrap();
    let mut data = super::ArchetypeStorage::new(archetype);
    let location = unsafe { data.add_entity(test_entity(0)) };
    unsafe {
        std::ptr::write(
            data.chunks[location.chunk_index]
                .component_ptr(component_index, location.entity_index)
                .cast::<PanicDroppableMarker>(),
            PanicDroppableMarker,
        );
    }

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(data)));
    assert!(result.is_err());
    assert_eq!(entity_pool_stats().0, 1);
    assert_eq!(pool_stats(), (0, 0));
}

#[test]
fn oversized_archetype_uses_an_exact_unpooled_one_row_layout() {
    let _guard = PoolGuard::new();
    let archetype = create_archetype().add_rust_component::<Oversized>().build();

    let data = super::ArchetypeStorage::new(archetype);
    assert_eq!(data.layouts.len(), 1);
    assert_eq!(data.layouts[0].max_entity_count(), 1);
    assert_eq!(
        data.layouts[0].chunk_size(),
        std::mem::size_of::<Oversized>()
    );

    let chunk = Chunk::new(archetype);
    assert_eq!(chunk.block_size(), std::mem::size_of::<Oversized>());
    drop(chunk);
    assert_eq!(pool_stats(), (0, 0));
}

#[test]
fn chunk_columns_respect_component_alignment() {
    let _guard = PoolGuard::new();

    let ty_a = register_component_type(
        "chunk_test_aligned16",
        core::mem::size_of::<Aligned16>(),
        core::mem::align_of::<Aligned16>(),
    );
    let ty_b = register_component_type(
        "chunk_test_aligned8",
        core::mem::size_of::<Aligned8>(),
        core::mem::align_of::<Aligned8>(),
    );

    let archetype = create_archetype()
        .add_component(ty_a)
        .add_component(ty_b)
        .build();
    let mut chunk = Chunk::new(archetype);
    let index_a = archetype.query_component_index(&ty_a).unwrap();
    let index_b = archetype.query_component_index(&ty_b).unwrap();

    let entity_a = crate::ecs::EntityId::new(0, 0);
    let entity_b = crate::ecs::EntityId::new(1, 0);

    assert!(unsafe { chunk.add_entity(entity_a) }.is_some());
    assert!(unsafe { chunk.add_entity(entity_b) }.is_some());

    assert!(chunk.max_entity_count > 0);
    assert_eq!(
        chunk.column_ptr(index_a) as usize % core::mem::align_of::<Aligned16>(),
        0
    );
    assert_eq!(
        chunk.column_ptr(index_b) as usize % core::mem::align_of::<Aligned8>(),
        0
    );
    assert_eq!(
        unsafe {
            chunk
                .component_ptr(index_a, 1)
                .offset_from(chunk.component_ptr(index_a, 0)) as usize
        },
        core::mem::size_of::<Aligned16>()
    );
}

#[test]
fn zero_sized_chunks_grow_entity_ids_from_actual_occupancy() {
    let _guard = PoolGuard::new();

    let archetype = create_archetype().add_rust_component::<Marker>().build();
    let mut data = super::ArchetypeStorage::new(archetype);
    data.prepare_batch_capacity(1);

    assert_eq!(data.chunks[0].max_entity_count, TINY_CHUNK_SIZE);
    assert_eq!(data.chunks[0].entities.capacity(), 0);

    unsafe {
        data.add_entity(test_entity(0));
    }
    assert!(data.chunks[0].entities.capacity() < data.chunks[0].max_entity_count);
    assert_eq!(pool_stats(), (0, 0));

    drop(data);
    assert_eq!(pool_stats(), (0, 0));
}

#[test]
fn zero_sized_chunk_uses_an_aligned_dangling_pointer() {
    let _guard = PoolGuard::new();

    let archetype = create_archetype()
        .add_rust_component::<AlignedMarker>()
        .build();
    let chunk = Chunk::new(archetype);
    let component_index = archetype
        .query_component_index(&crate::ecs::component_type::<AlignedMarker>())
        .unwrap();

    assert_eq!(std::mem::size_of::<AlignedMarker>(), 0);
    assert_eq!(
        chunk.column_ptr(component_index) as usize % std::mem::align_of::<AlignedMarker>(),
        0
    );
    assert_eq!(pool_stats(), (0, 0));

    drop(chunk);
    assert_eq!(pool_stats(), (0, 0));
}

#[test]
fn zero_sized_promotion_keeps_entity_ids_in_place() {
    let _guard = PoolGuard::new();

    let archetype = create_archetype().add_rust_component::<Marker>().build();
    let mut data = super::ArchetypeStorage::new(archetype);
    let location = unsafe { data.add_entity(test_entity(0)) };
    assert_eq!(location.entity_index, 0);

    let entity_ptr = data.chunks[0].entities.as_ptr();
    let entity_capacity = data.chunks[0].entities.capacity();
    let data_ptr = data.chunks[0].data_ptr();
    let layout = data.layouts[data.layout_index_after(TINY_CHUNK_SIZE)];
    data.chunks[0].promote_tiny(&layout);

    assert_eq!(data.chunks[0].block_size(), SMALL_CHUNK_SIZE);
    assert_eq!(data.chunks[0].entity_id(0), Some(test_entity(0)));
    assert_eq!(data.chunks[0].entities.as_ptr(), entity_ptr);
    assert_eq!(data.chunks[0].entities.capacity(), entity_capacity);
    assert_eq!(data.chunks[0].data_ptr(), data_ptr);
    assert_eq!(pool_stats(), (0, 0));
}

#[test]
fn droppable_zero_sized_components_survive_promotion_and_drop_once() {
    let _guard = PoolGuard::new();
    DROPPABLE_MARKER_DROPS.store(0, AtomicOrdering::Relaxed);

    let archetype = create_archetype()
        .add_rust_component::<DroppableMarker>()
        .build();
    let component_index = archetype
        .query_component_index(&crate::ecs::component_type::<DroppableMarker>())
        .unwrap();
    let mut data = super::ArchetypeStorage::new(archetype);

    for index in 0..=TINY_CHUNK_SIZE {
        let location = unsafe { data.add_entity(test_entity(index)) };
        unsafe {
            std::ptr::write(
                data.chunks[location.chunk_index]
                    .component_ptr(component_index, location.entity_index)
                    .cast::<DroppableMarker>(),
                DroppableMarker,
            );
        }
    }

    assert_eq!(data.chunks.len(), 1);
    assert_eq!(data.chunks[0].block_size(), SMALL_CHUNK_SIZE);
    assert_eq!(DROPPABLE_MARKER_DROPS.load(AtomicOrdering::Relaxed), 0);
    assert_eq!(pool_stats(), (0, 0));

    drop(data);
    assert_eq!(
        DROPPABLE_MARKER_DROPS.load(AtomicOrdering::Relaxed),
        TINY_CHUNK_SIZE + 1
    );
    assert_eq!(pool_stats(), (0, 0));
}

#[test]
fn mixed_zero_and_nonzero_components_still_use_backing_storage() {
    let _guard = PoolGuard::new();

    let archetype = create_archetype()
        .add_rust_component::<Marker>()
        .add_rust_component::<Tiny>()
        .build();
    let marker_index = archetype
        .query_component_index(&crate::ecs::component_type::<Marker>())
        .unwrap();
    let tiny_index = archetype
        .query_component_index(&crate::ecs::component_type::<Tiny>())
        .unwrap();
    let mut data = super::ArchetypeStorage::new(archetype);
    let location = unsafe { data.add_entity(test_entity(0)) };
    unsafe {
        std::ptr::write(
            data.chunks[location.chunk_index]
                .component_ptr(marker_index, location.entity_index)
                .cast::<Marker>(),
            Marker,
        );
        std::ptr::write(
            data.chunks[location.chunk_index]
                .component_ptr(tiny_index, location.entity_index)
                .cast::<Tiny>(),
            Tiny(7),
        );
    }

    assert_eq!(pool_stats(), (0, 0));
    drop(data);
    assert_eq!(pool_stats(), (1, TINY_CHUNK_SIZE));
}

#[test]
fn dropped_block_is_reused_on_same_thread() {
    let _guard = PoolGuard::new();

    let archetype = create_archetype().add_rust_component::<Tiny>().build();
    let first_ptr = {
        let chunk = Chunk::new(archetype);
        chunk.data_ptr() as usize
    };

    let (entry_count, _) = pool_stats();
    assert_eq!(entry_count, 1);

    let second_chunk = Chunk::new(archetype);
    assert_eq!(second_chunk.data_ptr() as usize, first_ptr);
}

#[test]
fn incremental_growth_reserves_one_initial_chunk_slot() {
    let _guard = PoolGuard::new();

    let archetype = create_archetype().add_rust_component::<Large>().build();
    let mut data = super::ArchetypeStorage::new(archetype);

    unsafe {
        data.add_entity(test_entity(0));
    }

    assert_eq!(data.chunks.len(), 1);
    assert_eq!(data.chunks.capacity(), 1);
}

#[test]
fn batch_chunk_count_accounts_for_promotion_and_repeated_tiers() {
    let _guard = PoolGuard::new();

    let archetype = create_archetype().add_rust_component::<Large>().build();
    let mut data = super::ArchetypeStorage::new(archetype);

    assert_eq!(data.batch_chunk_count(2), 1);
    assert_eq!(data.batch_chunk_count(257), 2);

    data.prepare_batch_capacity(257);
    let entities: Vec<_> = (0..257).map(test_entity).collect();
    for (index, &entity) in entities.iter().enumerate() {
        unsafe {
            data.add_entity_with_batch_hint(entity, entities.len() - index);
        }
    }

    assert_eq!(data.chunks.len(), 2);
    assert_eq!(data.chunks.capacity(), 2);
}

#[test]
fn batch_append_respects_the_existing_incremental_class_floor() {
    let _guard = PoolGuard::new();
    let archetype = create_archetype().add_rust_component::<Huge>().build();
    let mut data = super::ArchetypeStorage::new(archetype);
    let first = unsafe { data.add_entity(test_entity(0)) };
    unsafe {
        std::ptr::write(
            data.chunks[first.chunk_index]
                .component_ptr(0, first.entity_index)
                .cast::<Huge>(),
            Huge([0; 16 * 1024]),
        );
    }
    assert_eq!(data.chunks.capacity(), 1);

    data.prepare_batch_capacity(40);

    assert_eq!(data.chunks.capacity(), 2);
    for index in 1..=40 {
        let location = unsafe { data.add_entity_with_batch_hint(test_entity(index), 41 - index) };
        unsafe {
            std::ptr::write(
                data.chunks[location.chunk_index]
                    .component_ptr(0, location.entity_index)
                    .cast::<Huge>(),
                Huge([index as u8; 16 * 1024]),
            );
        }
    }
    assert_eq!(data.chunks.len(), 2);
    assert_eq!(data.chunks.capacity(), 2);
}

#[test]
fn batch_selects_smallest_fitting_class_and_preserves_order() {
    let _guard = PoolGuard::new();

    let archetype = create_archetype().add_rust_component::<Huge>().build();
    let mut data = super::ArchetypeStorage::new(archetype);
    assert_eq!(data.layouts.len(), 5);
    assert_eq!(data.layouts[0].chunk_size(), MEDIUM_CHUNK_SIZE);
    assert_eq!(data.layouts[0].max_entity_count(), 1);
    assert_eq!(data.layouts[3].chunk_size(), 1024 * 1024);
    assert_eq!(data.layouts[3].max_entity_count(), 64);
    assert_eq!(data.layouts[4].chunk_size(), MAX_CHUNK_SIZE);
    assert_eq!(data.layouts[4].max_entity_count(), 256);

    let entities: Vec<_> = (0..40).map(test_entity).collect();
    let locations = entities
        .iter()
        .enumerate()
        .map(|(index, &entity)| unsafe {
            data.add_entity_with_batch_hint(entity, entities.len() - index)
        })
        .collect::<Vec<_>>();

    assert_eq!(locations.len(), entities.len());
    assert_eq!(locations[0].chunk_index, 0);
    assert_eq!(locations[39].chunk_index, 0);
    assert_eq!(locations[39].entity_index, 39);
    assert_eq!(data.chunks.len(), 1);
    assert_eq!(data.chunks[0].block_size(), 1024 * 1024);
    assert_eq!(data.chunks[0].entity_count, 40);
    assert_eq!(data.chunks[0].entity_id(0), Some(entities[0]));
    assert_eq!(data.chunks[0].entity_id(39), Some(entities[39]));
}

#[test]
fn known_batch_uses_the_bounded_largest_first_remainder_plan() {
    let _guard = PoolGuard::new();

    #[allow(dead_code)]
    #[derive(Clone, Copy)]
    struct HundredBytes([u8; 100]);

    let archetype = create_archetype()
        .add_rust_component::<HundredBytes>()
        .build();
    let mut data = super::ArchetypeStorage::new(archetype);

    let mut plan = data.prepare_batch_capacity(100_000);

    assert_eq!(data.chunks.len(), 1);
    assert_eq!(data.chunks.capacity(), 3);
    assert_eq!(data.chunks[0].block_size(), MAX_CHUNK_SIZE);
    assert_eq!(
        plan.take_next_layout().unwrap().chunk_size(),
        MAX_CHUNK_SIZE
    );
    assert_eq!(
        plan.take_next_layout().unwrap().chunk_size(),
        MAX_CHUNK_SIZE
    );
    assert!(plan.take_next_layout().is_none());
}

#[test]
fn oversized_batch_plan_keeps_the_memory_minimal_three_chunk_layout() {
    let _guard = PoolGuard::new();

    #[allow(dead_code)]
    #[derive(Clone, Copy)]
    struct HundredBytes([u8; 100]);

    let archetype = create_archetype()
        .add_rust_component::<HundredBytes>()
        .build();
    let mut data = super::ArchetypeStorage::new(archetype);

    let mut plan = data.prepare_batch_capacity(1_000_000);

    assert_eq!(plan.remaining_chunk_count(), 2);
    assert_eq!(data.chunks.len(), 1);
    assert!(data.chunks.capacity() >= 3);
    assert_eq!(data.chunks[0].max_entity_count, 671_088);
    assert_eq!(data.chunks[0].block_size(), 64 * 1024 * 1024);
    assert_eq!(
        plan.take_next_layout().unwrap().chunk_size(),
        16 * 1024 * 1024
    );
    assert_eq!(
        plan.take_next_layout().unwrap().chunk_size(),
        16 * 1024 * 1024
    );
    assert!(plan.take_next_layout().is_none());

    drop(data);
    assert_eq!(pool_stats(), (0, 0));
}

#[test]
fn growth_after_an_oversized_tail_reenters_static_batch_tiers() {
    let _guard = PoolGuard::new();

    #[allow(dead_code)]
    #[derive(Clone, Copy)]
    struct HundredBytes([u8; 100]);

    let archetype = create_archetype()
        .add_rust_component::<HundredBytes>()
        .build();
    let mut data = super::ArchetypeStorage::new(archetype);
    data.prepare_batch_capacity(1_000_000);

    data.grow_tail(10_000);

    assert_eq!(data.chunks.len(), 2);
    assert_eq!(data.chunks[1].block_size(), REPEATED_TIER_START);
}

#[test]
fn incremental_growth_promotes_only_tiny_chunk_then_keeps_later_chunks_stable() {
    let _guard = PoolGuard::new();

    let archetype = create_archetype().add_rust_component::<Large>().build();
    let mut data = super::ArchetypeStorage::new(archetype);
    assert_eq!(
        data.layouts
            .iter()
            .map(|layout| layout.chunk_size())
            .collect::<Vec<_>>(),
        vec![
            TINY_CHUNK_SIZE,
            SMALL_CHUNK_SIZE,
            MEDIUM_CHUNK_SIZE,
            64 * 1024,
            REPEATED_TIER_START,
            1024 * 1024,
            MAX_CHUNK_SIZE,
        ]
    );

    let entities: Vec<_> = (0..257).map(test_entity).collect();
    let mut locations = Vec::with_capacity(entities.len());
    locations.push(unsafe { data.add_entity(entities[0]) });
    let tiny_chunk_ptr = data.chunks[0].data_ptr();
    locations.push(unsafe { data.add_entity(entities[1]) });
    let promoted_chunk_ptr = data.chunks[0].data_ptr();
    locations.extend(
        entities[2..]
            .iter()
            .map(|&entity| unsafe { data.add_entity(entity) }),
    );

    assert_eq!(locations[0].chunk_index, 0);
    assert_eq!(locations[0].entity_index, 0);
    assert_eq!(locations[1].chunk_index, 0);
    assert_eq!(locations[1].entity_index, 1);
    assert_eq!(locations[4].chunk_index, 1);
    assert_eq!(locations[20].chunk_index, 2);
    assert_eq!(locations[84].chunk_index, 3);
    assert_eq!(locations[256].chunk_index, 3);
    assert_eq!(locations[256].entity_index, 172);
    assert_eq!(data.chunks.len(), 4);
    assert_eq!(data.chunks[0].block_size(), SMALL_CHUNK_SIZE);
    assert_eq!(data.chunks[1].block_size(), MEDIUM_CHUNK_SIZE);
    assert_eq!(data.chunks[2].block_size(), 64 * 1024);
    assert_eq!(data.chunks[3].block_size(), REPEATED_TIER_START);
    assert_eq!(data.chunks[3].entity_count, 173);
    assert_ne!(tiny_chunk_ptr, promoted_chunk_ptr);
    assert_eq!(data.chunks[0].data_ptr(), promoted_chunk_ptr);
    assert_eq!(data.chunks[0].entity_id(0), Some(entities[0]));
    assert_eq!(data.chunks[3].entity_id(172), Some(entities[256]));
}

#[test]
fn empty_storage_warm_starts_from_the_highest_bounded_retired_tier() {
    let _guard = PoolGuard::new();

    let archetype = create_archetype().add_rust_component::<Large>().build();
    let mut data = super::ArchetypeStorage::new(archetype);
    let mut locations = Vec::new();

    // This reaches the 256 KiB tier. The warm-start policy deliberately caps
    // the remembered tier at 64 KiB.
    for index in 0..90 {
        let location = unsafe { data.add_entity(test_entity(index)) };
        unsafe {
            data.chunks[location.chunk_index]
                .component_ptr(0, location.entity_index)
                .cast::<Large>()
                .write(Large([index as u8; 1024]));
        }
        locations.push(location);
    }

    let warm_chunk_ptr = data
        .chunks
        .iter()
        .find(|chunk| chunk.block_size() == 64 * 1024)
        .unwrap()
        .data_ptr();
    assert_eq!(
        data.chunks.last().unwrap().block_size(),
        REPEATED_TIER_START
    );

    for location in locations.into_iter().rev() {
        data.remove_entity(location);
    }

    assert!(data.chunks.is_empty());
    assert_eq!(data.warm_start_chunk_size(), 64 * 1024);

    let location = unsafe { data.add_entity(test_entity(100)) };
    assert_eq!(location.chunk_index, 0);
    assert_eq!(location.entity_index, 0);
    assert_eq!(data.chunks[0].block_size(), 64 * 1024);
    assert_eq!(data.chunks[0].data_ptr(), warm_chunk_ptr);
    unsafe {
        data.chunks[0]
            .component_ptr(0, 0)
            .cast::<Large>()
            .write(Large([100; 1024]));
    }
}

#[test]
fn empty_storage_warm_start_is_correct_when_the_tls_pool_misses() {
    let _guard = PoolGuard::new();

    let archetype = create_archetype().add_rust_component::<Large>().build();
    let mut data = super::ArchetypeStorage::new(archetype);
    let mut locations = Vec::new();
    for index in 0..90 {
        let location = unsafe { data.add_entity(test_entity(index)) };
        unsafe {
            data.chunks[location.chunk_index]
                .component_ptr(0, location.entity_index)
                .cast::<Large>()
                .write(Large([index as u8; 1024]));
        }
        locations.push(location);
    }
    for location in locations.into_iter().rev() {
        data.remove_entity(location);
    }

    assert!(data.chunks.is_empty());
    assert_eq!(data.warm_start_chunk_size(), 64 * 1024);
    CHUNK_BLOCK_POOL.with(|pool| pool.borrow_mut().clear());
    assert_eq!(pool_stats(), (0, 0));

    let location = unsafe { data.add_entity(test_entity(100)) };
    assert_eq!(data.chunks[0].block_size(), 64 * 1024);
    unsafe {
        data.chunks[0]
            .component_ptr(0, location.entity_index)
            .cast::<Large>()
            .write(Large([100; 1024]));
    }
    assert_eq!(data.chunks[0].entity_id(0), Some(test_entity(100)));
}

#[test]
fn retiring_only_a_large_chunk_clamps_the_warm_start_hint() {
    let _guard = PoolGuard::new();

    let archetype = create_archetype().add_rust_component::<Tiny>().build();
    let mut data = super::ArchetypeStorage::new(archetype);
    let large_layout_index = data
        .layouts
        .iter()
        .position(|layout| layout.chunk_size() == REPEATED_TIER_START)
        .unwrap();
    data.add_chunk_with_layout(large_layout_index);

    let location = unsafe { data.add_entity(test_entity(0)) };
    unsafe {
        data.chunks[0]
            .component_ptr(0, location.entity_index)
            .cast::<Tiny>()
            .write(Tiny(7));
    }
    data.remove_entity(location);

    assert!(data.chunks.is_empty());
    assert_eq!(data.warm_start_chunk_size(), 64 * 1024);
}

#[test]
fn repeated_tail_tier_check_stops_after_four_matching_chunks() {
    let _guard = PoolGuard::new();

    let archetype = create_archetype().add_rust_component::<Large>().build();
    let mut data = super::ArchetypeStorage::new(archetype);
    let repeated_layout_index = data
        .layouts
        .iter()
        .position(|layout| layout.chunk_size() == REPEATED_TIER_START)
        .unwrap();

    for _ in 0..8 {
        data.add_chunk_with_layout(repeated_layout_index);
    }
    assert!(data
        .chunks
        .iter()
        .all(|chunk| chunk.block_size() == REPEATED_TIER_START));

    data.grow_tail(1);

    assert_eq!(data.chunks.len(), 9);
    assert_eq!(data.chunks.last().unwrap().block_size(), 1024 * 1024);
}

#[test]
fn emptied_data_chunk_returns_to_shared_pool() {
    let _guard = PoolGuard::new();

    let archetype = create_archetype().add_rust_component::<Tiny>().build();
    let mut data = super::ArchetypeStorage::new(archetype);
    let location = unsafe { data.add_entity(test_entity(0)) };
    assert_eq!(pool_stats(), (0, 0));

    data.remove_entity(location);

    assert!(data.chunks.is_empty());
    assert_eq!(pool_stats(), (1, TINY_CHUNK_SIZE));
}

#[test]
fn higher_alignment_block_satisfies_lower_alignment_request() {
    let _guard = PoolGuard::new();

    let high = create_archetype().add_rust_component::<Aligned32>().build();
    let low = create_archetype().add_rust_component::<Aligned8>().build();
    let high_ptr = {
        let chunk = Chunk::new(high);
        chunk.data_ptr() as usize
    };

    let low_chunk = Chunk::new(low);
    assert_eq!(low_chunk.data_ptr() as usize, high_ptr);
}

#[test]
fn different_archetypes_can_reuse_same_raw_block() {
    let _guard = PoolGuard::new();

    let large = create_archetype()
        .add_rust_component::<AlignedLarge>()
        .build();
    let small = create_archetype()
        .add_rust_component::<AlignedTiny>()
        .build();
    let large_ptr = {
        let chunk = Chunk::new(large);
        chunk.data_ptr() as usize
    };

    let small_chunk = Chunk::new(small);
    assert_eq!(small_chunk.data_ptr() as usize, large_ptr);
}

#[test]
fn retained_bytes_stay_within_budget() {
    let _guard = PoolGuard::new();

    let archetype = create_archetype().add_rust_component::<Large>().build();
    let mut chunks = Vec::new();
    for _ in 0..8 {
        chunks.push(Chunk::new(archetype));
    }
    drop(chunks);

    let (_, retained_bytes) = pool_stats();
    assert!(retained_bytes <= BACKING_POOL_BUDGET_BYTES);
}

#[test]
fn lru_eviction_removes_oldest_block_when_budget_overflows() {
    let _guard = PoolGuard::new();

    let archetype = create_archetype().add_rust_component::<Large>().build();
    let chunks: Vec<_> = (0..9).map(|_| Chunk::new(archetype)).collect();
    let ptrs: Vec<_> = chunks
        .iter()
        .map(|chunk| chunk.data_ptr() as usize)
        .collect();
    for chunk in chunks {
        drop(chunk);
    }

    let pooled = pooled_ptrs();
    assert!(!pooled.contains(&ptrs[0]));
    assert_eq!(pooled.len(), 8);
}

#[test]
fn pool_reset_helper_clears_thread_local_state() {
    let _guard = PoolGuard::new();

    let archetype = create_archetype().add_rust_component::<Tiny>().build();
    drop(Chunk::new(archetype));
    let (entry_count, _) = pool_stats();
    assert_eq!(entry_count, 1);

    CHUNK_BLOCK_POOL.with(|pool| pool.borrow_mut().clear());
    let (entry_count, retained_bytes) = pool_stats();
    assert_eq!(entry_count, 0);
    assert_eq!(retained_bytes, 0);
}

#[test]
fn thread_exit_storage_drop_survives_pool_tls_destruction() {
    POOL_WAS_UNAVAILABLE_DURING_THREAD_EXIT_STORAGE_DROP.store(false, AtomicOrdering::SeqCst);

    std::thread::spawn(|| {
        // Initializing the holder before constructing storage makes the pool
        // the later TLS value. Thread locals are then destroyed in reverse
        // initialization order: pool first, storage holder second.
        THREAD_EXIT_STORAGE.with(|holder| {
            let archetype = create_archetype().add_rust_component::<Tiny>().build();
            let mut storage = super::ArchetypeStorage::new(archetype);
            let location = unsafe { storage.add_entity(test_entity(0)) };
            unsafe {
                storage.chunks[location.chunk_index]
                    .component_ptr(0, location.entity_index)
                    .cast::<Tiny>()
                    .write(Tiny(7));
            }
            holder.borrow_mut().0 = Some(storage);
        });
    })
    .join()
    .expect("thread-local storage destruction must not panic");

    assert!(POOL_WAS_UNAVAILABLE_DURING_THREAD_EXIT_STORAGE_DROP.load(AtomicOrdering::SeqCst));
}
