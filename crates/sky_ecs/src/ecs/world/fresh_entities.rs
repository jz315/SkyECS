use super::*;

/// Commits metadata for one initialized contiguous row span whose entity slots
/// have never previously been allocated.
///
/// All fallible bounds checks happen before either metadata vector becomes
/// longer. The two raw append operations then contain no user code and cannot
/// unwind, so safe World APIs can never observe only half of the span.
///
/// # Safety
///
/// Every component in `[first_entity_index, first_entity_index + count)` must
/// already be initialized in `chunk`. Both metadata vectors must have reserved
/// `count` slots, and the span must be within the chunk's logical capacity.
#[inline]
pub(super) unsafe fn commit_fresh_entity_span(
    records: &mut Vec<EntityRecord>,
    chunk: &mut Chunk,
    chunk_id: ChunkId,
    first_entity_index: usize,
    count: usize,
) {
    if count == 0 {
        return;
    }

    let count_u32 = u32::try_from(count).expect("entity span limit exhausted");
    let first_row = u32::try_from(first_entity_index).expect("chunk entity index limit exhausted");
    first_row
        .checked_add(count_u32 - 1)
        .expect("chunk entity index limit exhausted");
    let first_entity_id = u32::try_from(records.len()).expect("entity slot limit exhausted");
    first_entity_id
        .checked_add(count_u32 - 1)
        .expect("entity slot limit exhausted");

    // SAFETY: the caller reserved both spans and initialized all component
    // rows. The checked arithmetic above makes the raw loops non-panicking.
    let actual_first_entity_id =
        unsafe { EntityRecord::append_fresh_span(records, chunk_id, first_row, count) };
    debug_assert_eq!(actual_first_entity_id, first_entity_id);
    unsafe {
        chunk.append_fresh_entity_ids(first_entity_id, count);
    }
}
