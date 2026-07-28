use super::{Chunk, QuerySpec};

/// Typed route-local component bases for one entity-query specification.
///
/// This contract is separate from [`QuerySpec`] because chunk iteration and
/// arbitrary-ID lookup have different cache lifecycles. Built-in query
/// parameters, tuples, and `#[derive(QueryData)]` implement it directly.
///
/// # Safety
///
/// `prepare_fetch` must resolve exactly the accesses declared by `QuerySpec`
/// and `fetch_item` must construct only the corresponding live row while
/// preserving every shared/exclusive access mode.
#[doc(hidden)]
pub unsafe trait EntityFetchSpec: QuerySpec {
    /// Monomorphized component-base representation cached for one chunk route.
    type Fetch: Copy;

    /// Resolves this query's typed component bases for one matching chunk.
    ///
    /// # Safety
    ///
    /// `component_indices` must be the descriptor-matched column map for
    /// `chunk`. Optional-component sentinels must be preserved in `Fetch`.
    unsafe fn prepare_fetch(chunk: &Chunk, component_indices: &[u8]) -> Self::Fetch;

    /// Constructs one query item from a prepared typed fetch.
    ///
    /// # Safety
    ///
    /// `fetch` must have been prepared from the matching live chunk route,
    /// `entity_index` must select an initialized row in that chunk, and the
    /// caller must uphold the query's complete aliasing contract for `'w`.
    unsafe fn fetch_item<'w>(fetch: &Self::Fetch, entity_index: usize) -> Self::Item<'w>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Position;
    struct Velocity;

    #[test]
    fn tuple_fetch_is_the_typed_component_base_tuple() {
        type Fetch = <(&'static Position, &'static mut Velocity) as EntityFetchSpec>::Fetch;

        assert_eq!(
            core::mem::size_of::<Fetch>(),
            2 * core::mem::size_of::<usize>()
        );
        assert_eq!(
            core::mem::align_of::<Fetch>(),
            core::mem::align_of::<usize>()
        );
    }
}
