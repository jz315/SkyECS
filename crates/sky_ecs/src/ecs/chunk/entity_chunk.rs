use super::pool::{
    ChunkBlock, CHUNK_BLOCK_POOL, ENTITY_BUFFER_NON_ZST_MAX_OVERSIZE,
    ENTITY_BUFFER_ZST_MAX_OVERSIZE,
};
use super::{
    Archetype, ChunkLayout, EntityId, CHUNK_SIZE, MAX_COMPONENTS, SMALL_CHUNK_SIZE, TINY_CHUNK_SIZE,
};
use std::alloc::Layout;
use std::mem;
use std::ptr::{self, NonNull};

pub struct Chunk {
    pub entity_count: usize,
    pub max_entity_count: usize,
    pub archetype: Archetype,
    data: NonNull<u8>,
    alloc_layout: Layout,
    column_offsets: [u32; MAX_COMPONENTS],
    pub(super) entities: Vec<EntityId>,
}

impl Chunk {
    #[inline]
    fn is_zero_sized_archetype(archetype: Archetype) -> bool {
        archetype
            .components
            .iter()
            .all(|component| component.size == 0)
    }

    fn aligned_dangling(alignment: usize) -> NonNull<u8> {
        debug_assert!(alignment != 0 && alignment.is_power_of_two());
        // ZST slices and references still require a non-null, aligned pointer,
        // but they never dereference storage. Use the alignment itself as a
        // provenance-free dangling address, as Vec does for zero-sized values.
        unsafe { NonNull::new_unchecked(ptr::without_provenance_mut(alignment)) }
    }

    fn take_block(size: usize, alignment: usize) -> ChunkBlock {
        CHUNK_BLOCK_POOL
            .try_with(|pool| pool.try_borrow_mut().ok()?.acquire(size, alignment))
            .ok()
            .flatten()
            .unwrap_or_else(|| ChunkBlock::fresh(size, alignment))
    }

    fn take_entities(preferred_capacity: usize, zero_sized_archetype: bool) -> Vec<EntityId> {
        let max_oversize_factor = if zero_sized_archetype {
            ENTITY_BUFFER_ZST_MAX_OVERSIZE
        } else {
            ENTITY_BUFFER_NON_ZST_MAX_OVERSIZE
        };
        let pooled = CHUNK_BLOCK_POOL
            .try_with(|pool| {
                pool.try_borrow_mut().ok()?.acquire_entities(
                    preferred_capacity,
                    zero_sized_archetype,
                    max_oversize_factor,
                )
            })
            .ok()
            .flatten();

        pooled.unwrap_or_else(|| {
            if zero_sized_archetype {
                // ZST chunks have no component backing allocation, so their
                // EntityId column grows only as far as actual occupancy needs.
                Vec::new()
            } else {
                Vec::with_capacity(preferred_capacity)
            }
        })
    }

    pub fn new(archetype: Archetype) -> Self {
        let layout = ChunkLayout::for_archetype(archetype, CHUNK_SIZE);
        Self::new_with_layout(archetype, &layout)
    }

    pub(crate) fn new_with_layout(archetype: Archetype, layout: &ChunkLayout) -> Self {
        let alignment = archetype.alignment.max(1);
        let zero_sized_archetype = Self::is_zero_sized_archetype(archetype);
        let (data, alloc_layout) = if zero_sized_archetype {
            (
                Self::aligned_dangling(alignment),
                Layout::from_size_align(layout.chunk_size(), alignment).unwrap(),
            )
        } else {
            Self::take_block(layout.chunk_size(), alignment).into_raw_parts()
        };
        let column_offsets = ChunkLayout::compute_column_offsets(
            archetype,
            layout.max_entity_count(),
            layout.chunk_size(),
        )
        .expect("cached chunk layout must fit its archetype");
        let entities = Self::take_entities(layout.max_entity_count(), zero_sized_archetype);

        Self {
            entity_count: 0,
            max_entity_count: layout.max_entity_count(),
            archetype,
            data,
            alloc_layout,
            column_offsets,
            entities,
        }
    }

    pub fn is_full(&self) -> bool {
        self.entity_count == self.max_entity_count
    }

    pub fn is_empty(&self) -> bool {
        self.entity_count == 0
    }

    #[inline(always)]
    pub(crate) fn block_size(&self) -> usize {
        self.alloc_layout.size()
    }

    /// Replaces the tiny starter block with the next layout while preserving
    /// row order. Growth beyond the starter tier appends stable chunks instead.
    pub(super) fn promote_tiny(&mut self, layout: &ChunkLayout) {
        debug_assert_eq!(self.block_size(), TINY_CHUNK_SIZE);
        debug_assert_eq!(layout.chunk_size(), SMALL_CHUNK_SIZE);
        debug_assert!(layout.max_entity_count() >= self.entity_count);

        if Self::is_zero_sized_archetype(self.archetype) {
            // There are no component bytes or pointers to relocate. Keep the
            // existing EntityId allocation and only advance the logical tier.
            self.max_entity_count = layout.max_entity_count();
            self.alloc_layout =
                Layout::from_size_align(layout.chunk_size(), self.archetype.alignment.max(1))
                    .unwrap();
            self.column_offsets = ChunkLayout::compute_column_offsets(
                self.archetype,
                layout.max_entity_count(),
                layout.chunk_size(),
            )
            .expect("cached zero-sized chunk layout must fit its archetype");
            return;
        }

        let entity_count = self.entity_count;
        let mut replacement = Self::new_with_layout(self.archetype, layout);
        for (component_index, component) in self.archetype.components.iter().enumerate() {
            if component.size == 0 {
                continue;
            }
            unsafe {
                ptr::copy_nonoverlapping(
                    self.column_ptr(component_index),
                    replacement.column_ptr(component_index),
                    component.size * entity_count,
                );
            }
        }

        replacement.entities.extend_from_slice(self.entities());
        replacement.entity_count = entity_count;
        self.entity_count = 0;
        self.entities.clear();
        *self = replacement;
    }

    /// Adds a logical entity slot without initializing component columns.
    ///
    /// # Safety
    ///
    /// The caller must initialize every component column for the returned
    /// entity index before the chunk can be queried, migrated, removed, or
    /// dropped. Otherwise component destructors may run on uninitialized data.
    #[inline(always)]
    pub unsafe fn add_entity(&mut self, entity: EntityId) -> Option<usize> {
        if self.entity_count == self.max_entity_count {
            return None;
        }

        Some(unsafe { self.add_entity_unchecked(entity) })
    }

    /// Adds an entity when the caller has already reserved a row in this
    /// chunk's current capacity span.
    ///
    /// # Safety
    ///
    /// `self.entity_count` must be strictly less than
    /// `self.max_entity_count`, and the caller must initialize every component
    /// in the returned row before the chunk can be observed or dropped.
    #[inline(always)]
    pub(crate) unsafe fn add_entity_unchecked(&mut self, entity: EntityId) -> usize {
        debug_assert!(self.entity_count < self.max_entity_count);

        let entity_index = self.entity_count;
        self.entity_count += 1;
        self.entities.push(entity);
        entity_index
    }

    #[inline]
    pub(crate) fn reserve_entity_slots(&mut self, additional: usize) {
        self.entities.reserve(additional);
    }

    /// Adds an entity after the caller has reserved the EntityId vector.
    ///
    /// # Safety
    ///
    /// The chunk must have a free logical row and `self.entities` must have
    /// spare capacity for one more `EntityId`. The caller must initialize all
    /// component columns for the returned row before the chunk is observed.
    #[inline(always)]
    pub(crate) unsafe fn add_entity_reserved_unchecked(&mut self, entity: EntityId) -> usize {
        debug_assert!(self.entity_count < self.max_entity_count);
        debug_assert!(self.entities.len() < self.entities.capacity());

        let entity_index = self.entity_count;
        let entity_len = self.entities.len();
        unsafe {
            self.entities.as_mut_ptr().add(entity_len).write(entity);
            self.entities.set_len(entity_len + 1);
        }
        self.entity_count += 1;
        entity_index
    }

    #[inline(always)]
    pub fn column_ptr(&self, component_index: usize) -> *mut u8 {
        unsafe {
            self.data
                .as_ptr()
                .add(self.column_offsets[component_index] as usize)
        }
    }

    pub fn data_ptr(&self) -> *mut u8 {
        self.data.as_ptr()
    }

    #[inline(always)]
    pub(super) unsafe fn component_ptr_unchecked(
        &self,
        component_index: usize,
        entity_index: usize,
    ) -> *mut u8 {
        let component = &self.archetype.components[component_index];
        let column = self.column_ptr(component_index);

        // SAFETY: The caller guarantees that `component_index` identifies a
        // component column in this chunk and that `entity_index` is a valid row.
        unsafe { column.add(entity_index * component.size) }
    }

    #[inline(always)]
    pub fn component_ptr(&self, component_index: usize, entity_index: usize) -> *mut u8 {
        if entity_index >= self.entity_count {
            return ptr::null_mut();
        }

        unsafe { self.component_ptr_unchecked(component_index, entity_index) }
    }

    pub(crate) fn entities(&self) -> &[EntityId] {
        &self.entities[..self.entity_count]
    }

    pub fn get_entity_as_ptr(&self, index: usize) -> *const u8 {
        if self.archetype.components.is_empty() {
            return ptr::null();
        }

        self.component_ptr(0, index) as *const u8
    }

    pub fn entity_id(&self, entity_index: usize) -> Option<EntityId> {
        self.entities.get(entity_index).copied()
    }

    #[inline(always)]
    pub(crate) fn remove_last_entity(&mut self) -> Option<EntityId> {
        if self.entity_count == 0 {
            return None;
        }

        self.entity_count -= 1;
        self.entities.pop()
    }

    // -----------------------------------------------------------------------
    // Drop helpers
    // -----------------------------------------------------------------------

    /// Drops every live component while retaining at most the first panic.
    /// The caller can then release the backing allocation before resuming it.
    unsafe fn drop_all_entities_catching(&self) -> Option<Box<dyn std::any::Any + Send + 'static>> {
        let mut first_panic = None;
        for entity_index in 0..self.entity_count {
            for &component_index in &self.archetype.drop_component_indices {
                let component_index = component_index as usize;
                let component = &self.archetype.components[component_index];
                let drop_fn = component
                    .drop_fn()
                    .expect("drop component index must point to a droppable component");
                let ptr = unsafe { self.component_ptr_unchecked(component_index, entity_index) };
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
                    drop_fn(ptr);
                }));
                if let Err(payload) = result {
                    if first_panic.is_none() {
                        first_panic = Some(payload);
                    } else {
                        // Dropping a later payload could itself panic while we
                        // are repairing storage after the first destructor.
                        mem::forget(payload);
                    }
                }
            }
        }
        first_panic
    }
}

impl Drop for Chunk {
    fn drop(&mut self) {
        // Safety: we are the sole owner and this chunk is being destroyed.
        // Catch destructor panics long enough to release the allocation and
        // invalidate the initialized prefix before resuming the first one.
        let drop_panic = unsafe { self.drop_all_entities_catching() };
        self.entity_count = 0;

        let mut block = (!Self::is_zero_sized_archetype(self.archetype)).then(|| ChunkBlock {
            data: self.data,
            alloc_layout: self.alloc_layout,
        });
        self.entities.clear();
        let mut entities = Some(mem::take(&mut self.entities));
        let _ = CHUNK_BLOCK_POOL.try_with(|pool| {
            let Ok(mut pool) = pool.try_borrow_mut() else {
                return;
            };
            if let Some(block) = block.take() {
                pool.release(block);
            }
            pool.release_entities(entities.take().unwrap());
        });
        // If TLS is already being destroyed or currently borrowed, the owned
        // fallback values are dropped here and deallocate normally.
        drop(block);
        drop(entities);

        if let Some(payload) = drop_panic {
            std::panic::resume_unwind(payload);
        }
    }
}
