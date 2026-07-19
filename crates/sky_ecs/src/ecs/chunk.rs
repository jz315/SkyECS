#![deny(unsafe_op_in_unsafe_fn)]

use super::{Archetype, EntityId, MAX_COMPONENTS};

mod archetype_storage;
mod entity_chunk;
mod layout;
mod pool;

pub(crate) use archetype_storage::{ArchetypeStorage, ChunkEntityLocation, ChunkRowSpan};
pub use entity_chunk::Chunk;
pub(crate) use layout::ChunkLayout;

pub(crate) const TINY_CHUNK_SIZE: usize = 1024;
pub(crate) const SMALL_CHUNK_SIZE: usize = 4 * 1024;
pub(crate) const MEDIUM_CHUNK_SIZE: usize = 16 * 1024;
pub(crate) const REPEATED_TIER_START: usize = 256 * 1024;
pub(crate) const CHUNK_SIZE: usize = 512 * 1024;
pub(crate) const MAX_CHUNK_SIZE: usize = 4 * 1024 * 1024;
const CHUNK_SIZE_TIERS: [usize; 7] = [
    TINY_CHUNK_SIZE,
    SMALL_CHUNK_SIZE,
    MEDIUM_CHUNK_SIZE,
    64 * 1024,
    REPEATED_TIER_START,
    1024 * 1024,
    MAX_CHUNK_SIZE,
];
const CHUNK_TIER_COUNT: usize = CHUNK_SIZE_TIERS.len();

#[cfg(test)]
#[path = "chunk/tests.rs"]
mod tests;
