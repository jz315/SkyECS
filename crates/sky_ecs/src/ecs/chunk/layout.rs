use super::{Archetype, MAX_COMPONENTS};
use std::alloc::Layout;

fn align_up(value: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (value + align - 1) & !(align - 1)
}

#[derive(Clone, Copy)]
pub(crate) struct ChunkLayout {
    // Every pooled tier is at most 4 MiB. Oversized batch and one-row layouts
    // are unpooled and intentionally limited to u32 offsets as well, which
    // keeps the layout and every Chunk column table compact on 64-bit targets.
    pub(super) chunk_size: u32,
    pub(super) max_entity_count: u32,
}

impl ChunkLayout {
    pub(super) fn exact_capacity(archetype: Archetype, entity_capacity: usize) -> Option<Self> {
        if entity_capacity == 0 {
            return None;
        }

        let component_bytes = archetype
            .components
            .iter()
            .try_fold(0usize, |bytes, component| bytes.checked_add(component.size))?;
        let chunk_size = if component_bytes == 0 {
            entity_capacity
        } else {
            let mut cursor = 0usize;
            for component in &archetype.components {
                cursor = align_up(cursor, component.align);
                cursor = cursor.checked_add(component.size.checked_mul(entity_capacity)?)?;
            }
            cursor.max(1)
        };

        let _ = Layout::from_size_align(chunk_size, archetype.alignment.max(1)).ok()?;
        Self::compute_column_offsets(archetype, entity_capacity, chunk_size)?;
        Some(Self {
            chunk_size: u32::try_from(chunk_size).ok()?,
            max_entity_count: u32::try_from(entity_capacity).ok()?,
        })
    }

    pub(super) fn exact_one_entity(archetype: Archetype) -> Option<Self> {
        Self::exact_capacity(archetype, 1)
    }

    pub(super) fn compute_column_offsets(
        archetype: Archetype,
        entity_capacity: usize,
        chunk_size: usize,
    ) -> Option<[u32; MAX_COMPONENTS]> {
        let mut offsets = [0_u32; MAX_COMPONENTS];
        let mut cursor = 0usize;

        for (component_index, component) in archetype.components.iter().enumerate() {
            cursor = align_up(cursor, component.align);

            let bytes = component.size.checked_mul(entity_capacity)?;
            let end = cursor.checked_add(bytes)?;
            if end > chunk_size {
                return None;
            }

            offsets[component_index] = u32::try_from(cursor).ok()?;
            cursor = end;
        }

        Some(offsets)
    }

    fn try_for_archetype(archetype: Archetype, chunk_size: usize) -> Option<Self> {
        let component_bytes: usize = archetype
            .components
            .iter()
            .map(|component| component.size)
            .sum();

        Self::try_for_archetype_with_component_bytes(archetype, chunk_size, component_bytes)
    }

    pub(super) fn try_for_archetype_with_component_bytes(
        archetype: Archetype,
        chunk_size: usize,
        component_bytes: usize,
    ) -> Option<Self> {
        if component_bytes == 0 {
            Self::compute_column_offsets(archetype, chunk_size, chunk_size)
                .expect("zero-sized archetypes should always fit in a chunk");
            return Some(Self {
                chunk_size: u32::try_from(chunk_size).ok()?,
                max_entity_count: u32::try_from(chunk_size).ok()?,
            });
        }

        let mut entity_capacity = (chunk_size / component_bytes).max(1);

        while entity_capacity > 0 {
            if let Some(_column_offsets) =
                Self::compute_column_offsets(archetype, entity_capacity, chunk_size)
            {
                return Some(Self {
                    chunk_size: u32::try_from(chunk_size).ok()?,
                    max_entity_count: u32::try_from(entity_capacity).ok()?,
                });
            }
            entity_capacity -= 1;
        }

        None
    }

    pub(super) fn for_archetype(archetype: Archetype, chunk_size: usize) -> Self {
        Self::try_for_archetype(archetype, chunk_size)
            .or_else(|| Self::exact_one_entity(archetype))
            .unwrap_or_else(|| {
                panic!("archetype is too large to represent with 32-bit chunk offsets")
            })
    }

    #[inline(always)]
    pub(crate) fn chunk_size(&self) -> usize {
        self.chunk_size as usize
    }

    #[inline(always)]
    pub(super) fn max_entity_count(&self) -> usize {
        self.max_entity_count as usize
    }
}
