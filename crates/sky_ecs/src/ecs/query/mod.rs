macro_rules! for_each_query_arity {
    ($implementation:ident) => {
        $implementation!(Arity1, Args1, (A, a));
        $implementation!(Arity2, Args2, (A, a), (B, b));
        $implementation!(Arity3, Args3, (A, a), (B, b), (C, c));
        $implementation!(Arity4, Args4, (A, a), (B, b), (C, c), (D, d));
        $implementation!(Arity5, Args5, (A, a), (B, b), (C, c), (D, d), (E, e));
        $implementation!(
            Arity6,
            Args6,
            (A, a),
            (B, b),
            (C, c),
            (D, d),
            (E, e),
            (F, f)
        );
        $implementation!(
            Arity7,
            Args7,
            (A, a),
            (B, b),
            (C, c),
            (D, d),
            (E, e),
            (F, f),
            (G, g)
        );
        $implementation!(
            Arity8,
            Args8,
            (A, a),
            (B, b),
            (C, c),
            (D, d),
            (E, e),
            (F, f),
            (G, g),
            (H, h)
        );
        $implementation!(
            Arity9,
            Args9,
            (A, a),
            (B, b),
            (C, c),
            (D, d),
            (E, e),
            (F, f),
            (G, g),
            (H, h),
            (I, i)
        );
        $implementation!(
            Arity10,
            Args10,
            (A, a),
            (B, b),
            (C, c),
            (D, d),
            (E, e),
            (F, f),
            (G, g),
            (H, h),
            (I, i),
            (J, j)
        );
        $implementation!(
            Arity11,
            Args11,
            (A, a),
            (B, b),
            (C, c),
            (D, d),
            (E, e),
            (F, f),
            (G, g),
            (H, h),
            (I, i),
            (J, j),
            (K, k)
        );
        $implementation!(
            Arity12,
            Args12,
            (A, a),
            (B, b),
            (C, c),
            (D, d),
            (E, e),
            (F, f),
            (G, g),
            (H, h),
            (I, i),
            (J, j),
            (K, k),
            (L, l)
        );
        $implementation!(
            Arity13,
            Args13,
            (A, a),
            (B, b),
            (C, c),
            (D, d),
            (E, e),
            (F, f),
            (G, g),
            (H, h),
            (I, i),
            (J, j),
            (K, k),
            (L, l),
            (M, m)
        );
        $implementation!(
            Arity14,
            Args14,
            (A, a),
            (B, b),
            (C, c),
            (D, d),
            (E, e),
            (F, f),
            (G, g),
            (H, h),
            (I, i),
            (J, j),
            (K, k),
            (L, l),
            (M, m),
            (N, n)
        );
        $implementation!(
            Arity15,
            Args15,
            (A, a),
            (B, b),
            (C, c),
            (D, d),
            (E, e),
            (F, f),
            (G, g),
            (H, h),
            (I, i),
            (J, j),
            (K, k),
            (L, l),
            (M, m),
            (N, n),
            (O, o)
        );
        $implementation!(
            Arity16,
            Args16,
            (A, a),
            (B, b),
            (C, c),
            (D, d),
            (E, e),
            (F, f),
            (G, g),
            (H, h),
            (I, i),
            (J, j),
            (K, k),
            (L, l),
            (M, m),
            (N, n),
            (O, o),
            (P, p)
        );
    };
}
pub(crate) use for_each_query_arity;

mod bound;
mod cache;
mod callback;
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
pub(crate) use callback::QueryShapeMarker;
#[doc(hidden)]
pub use callback::{
    Args1, Args10, Args11, Args12, Args13, Args14, Args15, Args16, Args2, Args3, Args4, Args5,
    Args6, Args7, Args8, Args9, Arity1, Arity10, Arity11, Arity12, Arity13, Arity14, Arity15,
    Arity16, Arity2, Arity3, Arity4, Arity5, Arity6, Arity7, Arity8, Arity9,
};
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
