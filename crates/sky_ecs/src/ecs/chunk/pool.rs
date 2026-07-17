use super::{
    EntityId, CHUNK_SIZE, MAX_CHUNK_SIZE, MEDIUM_CHUNK_SIZE, REPEATED_TIER_START, SMALL_CHUNK_SIZE,
    TINY_CHUNK_SIZE,
};
use std::alloc::{alloc, dealloc, handle_alloc_error, Layout};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::mem;
use std::ptr::NonNull;

pub(super) const BACKING_POOL_BUDGET_BYTES: usize = 4 * 1024 * 1024;
pub(super) const ENTITY_BUFFER_POOL_BUDGET_BYTES: usize = 1024 * 1024;
pub(super) const ENTITY_BUFFER_POOL_MAX_ENTRIES: usize = 64;
pub(super) const ENTITY_BUFFER_NON_ZST_MAX_OVERSIZE: usize = 2;
pub(super) const ENTITY_BUFFER_ZST_MAX_OVERSIZE: usize = 4;
// Archetype storage uses the seven adaptive tiers above, while the public expert
// `Chunk::new` constructor retains its historical 512 KiB allocation. Pool
// blocks must match the requested size exactly, so the expert size gets its
// own bucket rather than being folded into a neighboring adaptive tier.
pub(super) const BACKING_POOL_SIZE_TIERS: [usize; 8] = [
    TINY_CHUNK_SIZE,
    SMALL_CHUNK_SIZE,
    MEDIUM_CHUNK_SIZE,
    64 * 1024,
    REPEATED_TIER_START,
    CHUNK_SIZE,
    1024 * 1024,
    MAX_CHUNK_SIZE,
];

thread_local! {
    pub(super) static CHUNK_BLOCK_POOL: RefCell<ChunkBlockPool> =
        RefCell::new(ChunkBlockPool::default());
}

pub(super) struct ChunkBlock {
    pub(super) data: NonNull<u8>,
    pub(super) alloc_layout: Layout,
}

impl ChunkBlock {
    pub(super) fn fresh(size: usize, alignment: usize) -> Self {
        let alloc_layout = Layout::from_size_align(size, alignment.max(1)).unwrap();
        let data = unsafe {
            // Chunk storage is intentionally uninitialised. Occupied rows are
            // created only through the unsafe slot-allocation path, whose
            // caller must initialise every component before the row can be
            // observed or dropped. Reused pool blocks already have the same
            // property, so fresh blocks do not need a separate zeroing pass.
            let raw = alloc(alloc_layout);
            NonNull::new(raw).unwrap_or_else(|| handle_alloc_error(alloc_layout))
        };

        Self { data, alloc_layout }
    }

    #[inline(always)]
    pub(super) fn retained_size(&self) -> usize {
        self.alloc_layout.size()
    }

    #[inline(always)]
    pub(super) fn size(&self) -> usize {
        self.alloc_layout.size()
    }

    #[inline(always)]
    pub(super) fn alignment(&self) -> usize {
        self.alloc_layout.align()
    }

    pub(super) fn into_raw_parts(self) -> (NonNull<u8>, Layout) {
        let data = self.data;
        let alloc_layout = self.alloc_layout;
        mem::forget(self);
        (data, alloc_layout)
    }
}

impl Drop for ChunkBlock {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.data.as_ptr(), self.alloc_layout);
        }
    }
}

pub(super) struct PooledBlock {
    pub(super) block: ChunkBlock,
    pub(super) last_used: u64,
}

pub(super) struct PooledEntityBuffer {
    pub(super) buffer: Vec<EntityId>,
    pub(super) last_used: u64,
}

#[derive(Default)]
pub(super) struct ChunkBlockPool {
    pub(super) block_buckets: [VecDeque<PooledBlock>; BACKING_POOL_SIZE_TIERS.len()],
    pub(super) retained_bytes: usize,
    pub(super) entity_buffers: Vec<PooledEntityBuffer>,
    pub(super) retained_entity_bytes: usize,
    pub(super) clock: u64,
}

impl ChunkBlockPool {
    #[inline(always)]
    fn next_tick(&mut self) -> u64 {
        let tick = self.clock;
        self.clock = self.clock.wrapping_add(1);
        tick
    }

    #[inline(always)]
    fn block_bucket_index(size: usize) -> Option<usize> {
        BACKING_POOL_SIZE_TIERS.binary_search(&size).ok()
    }

    pub(super) fn acquire(&mut self, size: usize, alignment: usize) -> Option<ChunkBlock> {
        let bucket_index = Self::block_bucket_index(size)?;
        let bucket = &mut self.block_buckets[bucket_index];

        // Exact-alignment reuse is already the best possible fit. Keeping this
        // common tail path avoids a scan during steady same-archetype churn.
        if bucket
            .back()
            .is_some_and(|entry| entry.block.alignment() == alignment)
        {
            let entry = bucket.pop_back().unwrap();
            self.retained_bytes -= entry.block.retained_size();
            return Some(entry.block);
        }

        let mut best: Option<(usize, usize)> = None;

        // Buckets preserve release order. Search newest-to-oldest so ties use
        // the most recently released block while the oldest remaining entry
        // stays at the front for exact global-LRU eviction.
        for (index, entry) in bucket.iter().enumerate().rev() {
            let entry_alignment = entry.block.alignment();
            if entry_alignment < alignment {
                continue;
            }

            let replace = best.is_none_or(|(_, best_alignment)| entry_alignment < best_alignment);
            if replace {
                best = Some((index, entry_alignment));
                if entry_alignment == alignment {
                    break;
                }
            }
        }

        let index = best.map(|(index, _)| index)?;
        let entry = bucket.remove(index).unwrap();
        self.retained_bytes -= entry.block.retained_size();
        Some(entry.block)
    }

    fn entity_buffer_bytes(buffer: &Vec<EntityId>) -> usize {
        buffer.capacity().saturating_mul(mem::size_of::<EntityId>())
    }

    /// Takes the closest reusable EntityId allocation without coupling it to
    /// a component backing block. Non-ZST chunks require enough capacity up
    /// front; blockless ZST chunks may reuse a smaller allocation and keep
    /// growing lazily from the occupancy they actually reach.
    pub(super) fn acquire_entities(
        &mut self,
        preferred_capacity: usize,
        allow_undersized: bool,
        max_oversize_factor: usize,
    ) -> Option<Vec<EntityId>> {
        debug_assert!(preferred_capacity > 0);
        debug_assert!(max_oversize_factor > 0);

        let max_capacity = preferred_capacity.saturating_mul(max_oversize_factor);
        let mut best_sufficient: Option<(usize, usize)> = None;
        let mut best_undersized: Option<(usize, usize)> = None;

        for (index, entry) in self.entity_buffers.iter().enumerate() {
            let capacity = entry.buffer.capacity();
            if capacity >= preferred_capacity && capacity <= max_capacity {
                if best_sufficient.is_none_or(|(_, best_capacity)| capacity < best_capacity) {
                    best_sufficient = Some((index, capacity));
                }
            } else if allow_undersized
                && capacity < preferred_capacity
                && best_undersized.is_none_or(|(_, best_capacity)| capacity > best_capacity)
            {
                best_undersized = Some((index, capacity));
            }
        }

        let index = best_sufficient.or(best_undersized)?.0;
        let entry = self.entity_buffers.swap_remove(index);
        self.retained_entity_bytes -= Self::entity_buffer_bytes(&entry.buffer);
        debug_assert!(entry.buffer.is_empty());
        Some(entry.buffer)
    }

    fn evict_oldest(&mut self) -> bool {
        let Some((oldest_bucket_index, _)) = self
            .block_buckets
            .iter()
            .enumerate()
            .filter_map(|(bucket_index, bucket)| {
                bucket.front().map(|entry| (bucket_index, entry.last_used))
            })
            .min_by_key(|(_, last_used)| *last_used)
        else {
            return false;
        };

        let entry = self.block_buckets[oldest_bucket_index].pop_front().unwrap();
        self.retained_bytes -= entry.block.retained_size();
        true
    }

    fn evict_oldest_entity_buffer(&mut self) -> bool {
        let Some((oldest_index, _)) = self
            .entity_buffers
            .iter()
            .enumerate()
            .min_by_key(|(_, entry)| entry.last_used)
        else {
            return false;
        };

        let entry = self.entity_buffers.swap_remove(oldest_index);
        self.retained_entity_bytes -= Self::entity_buffer_bytes(&entry.buffer);
        true
    }

    pub(super) fn release(&mut self, block: ChunkBlock) {
        let retained_size = block.retained_size();
        if retained_size > BACKING_POOL_BUDGET_BYTES {
            return;
        }
        let Some(bucket_index) = Self::block_bucket_index(block.size()) else {
            return;
        };

        while self.retained_bytes + retained_size > BACKING_POOL_BUDGET_BYTES {
            if !self.evict_oldest() {
                return;
            }
        }

        let last_used = self.next_tick();
        self.retained_bytes += retained_size;
        self.block_buckets[bucket_index].push_back(PooledBlock { block, last_used });
    }

    pub(super) fn release_entities(&mut self, mut buffer: Vec<EntityId>) {
        buffer.clear();
        let retained_size = Self::entity_buffer_bytes(&buffer);
        if retained_size == 0 || retained_size > ENTITY_BUFFER_POOL_BUDGET_BYTES {
            return;
        }

        while self.entity_buffers.len() >= ENTITY_BUFFER_POOL_MAX_ENTRIES
            || self.retained_entity_bytes.saturating_add(retained_size)
                > ENTITY_BUFFER_POOL_BUDGET_BYTES
        {
            if !self.evict_oldest_entity_buffer() {
                return;
            }
        }

        let last_used = self.next_tick();
        self.retained_entity_bytes += retained_size;
        self.entity_buffers
            .push(PooledEntityBuffer { buffer, last_used });
    }

    #[cfg(test)]
    pub(super) fn clear(&mut self) {
        for bucket in &mut self.block_buckets {
            bucket.clear();
        }
        self.retained_bytes = 0;
        self.entity_buffers.clear();
        self.retained_entity_bytes = 0;
        self.clock = 0;
    }
}
