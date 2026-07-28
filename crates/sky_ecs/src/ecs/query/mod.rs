mod bound;
mod cache;
mod entity_fetch;
mod filter;
mod parallel;
mod param;
mod plan;
mod prepared;
mod sequential;

use super::component_posting::ComponentPostingList;
use super::{Chunk, EntityId, World};
use crate::ecs::ComponentType;
use core::ops::Deref;
use core::ptr;
use smallvec::SmallVec;
use std::sync::Arc;

pub(crate) use bound::{
    count_matches, matches_nothing, run_for_each, run_for_each_chunk,
    run_for_each_chunk_with_entities, run_for_each_with_entity,
};
pub use bound::{Query, QueryMut};
pub(crate) use cache::QueryCacheStore;
#[doc(hidden)]
pub use entity_fetch::EntityFetchSpec;
pub use filter::{Any, QueryFilter, With, Without};
pub(crate) use parallel::{
    par_for_each, par_for_each_chunk, par_for_each_chunk_with_entities, par_for_each_with_entity,
    prepare_job_cache, prepared_job_snapshot, ParallelJobCache, ParallelJobSnapshot,
};
pub use param::{QueryParam, QuerySpec, ReadOnlyQuerySpec};
pub(crate) use plan::{resolve_column_ptr, CachedArchetype, ComponentIndexMap, PreparedCache};
pub use plan::{QueryComponent, QueryDescriptor};
pub use prepared::PreparedQuery;
pub(crate) use sequential::{
    run_for_each as run_cached_for_each, run_for_each_chunk as run_cached_for_each_chunk,
    run_for_each_chunk_with_entities as run_cached_for_each_chunk_with_entities,
    run_for_each_with_entity as run_cached_for_each_with_entity, SequentialChunk,
    SequentialChunkCache,
};

pub trait QueryFilterSealed {}

const INLINE_QUERY_COMPONENTS: usize = 8;
pub(crate) const MAX_QUERY_COMPONENTS: usize = 16;
const OPTIONAL_SENTINEL: u8 = u8::MAX;

#[doc(hidden)]
pub trait QueryWorld<Q: QuerySpec>: sealed::QueryWorldSealed {
    fn as_world(&self) -> &World;
}

mod sealed {
    use super::World;

    pub trait QueryWorldSealed {}

    impl QueryWorldSealed for &World {}
    impl QueryWorldSealed for &mut World {}
}

impl<Q: QuerySpec> QueryWorld<Q> for &mut World {
    #[inline(always)]
    fn as_world(&self) -> &World {
        self
    }
}

impl<Q: ReadOnlyQuerySpec> QueryWorld<Q> for &World {
    #[inline(always)]
    fn as_world(&self) -> &World {
        self
    }
}
