use super::resolve_column_ptr;
use super::{Arity1, Chunk, EntityFetchSpec, QueryComponent, QueryDescriptor};
use crate::ecs::component_type;
use core::slice;
use smallvec::SmallVec;

/// One component access inside a typed query specification.
///
/// # Safety
///
/// Implementations must return the exact component and access mode represented
/// by the type, and may only construct references of that component type from
/// initialized, correctly aligned ranges supplied by the query executor.
pub unsafe trait QueryParam {
    type Slice<'w>;
    type Item<'w>;
    /// Typed component-base representation used by arbitrary-ID lookup.
    type EntityFetch: Copy;

    fn component() -> QueryComponent;

    /// Resolves this parameter's column pointer for one matching chunk.
    ///
    /// Required parameters override this to avoid the optional-component
    /// sentinel check in the chunk-iteration hot path. Custom parameters keep
    /// the conservative default.
    ///
    /// # Safety
    ///
    /// `component_index` must be the cached index produced for this parameter
    /// and `chunk` must belong to the matching archetype.
    #[doc(hidden)]
    #[inline(always)]
    unsafe fn resolve_column(chunk: &Chunk, component_index: u8) -> *mut u8 {
        resolve_column_ptr(chunk, component_index)
    }

    /// # Safety
    ///
    /// `ptr` must address a live, correctly aligned column of this component
    /// type, and `start..start + len` must be initialized and in bounds. The
    /// caller must uphold the access mode represented by this parameter.
    unsafe fn slice_from_raw<'w>(ptr: *mut u8, start: usize, len: usize) -> Self::Slice<'w>;

    /// # Safety
    ///
    /// `ptr` must address a live, correctly aligned column of this component
    /// type and `index` must select an initialized in-bounds row. The caller
    /// must uphold the access mode represented by this parameter.
    unsafe fn item_from_raw<'w>(ptr: *mut u8, index: usize) -> Self::Item<'w>;

    /// Converts a descriptor-matched erased column base into the typed form
    /// retained by an entity-route cache.
    ///
    /// # Safety
    ///
    /// `ptr` must be the corresponding live component column base, or null
    /// only when this parameter is optional and the component is absent.
    #[doc(hidden)]
    unsafe fn prepare_entity_fetch(ptr: *mut u8) -> Self::EntityFetch;

    /// Builds one item from a previously prepared typed component base.
    ///
    /// # Safety
    ///
    /// `fetch` must describe this parameter's matching live chunk and `index`
    /// must select an initialized in-bounds row. The caller must uphold this
    /// parameter's shared/exclusive access mode for `'w`.
    #[doc(hidden)]
    unsafe fn item_from_entity_fetch<'w>(fetch: &Self::EntityFetch, index: usize)
        -> Self::Item<'w>;
}

/// Marker for query parameters that never construct mutable references.
///
/// # Safety
///
/// The underlying [`QueryParam`] implementation must expose shared access only.
pub unsafe trait ReadOnlyQueryParam: QueryParam {}

unsafe impl<T: 'static> QueryParam for &T {
    type Slice<'w> = &'w [T];
    type Item<'w> = &'w T;
    type EntityFetch = *const T;

    #[inline(always)]
    fn component() -> QueryComponent {
        QueryComponent::new(component_type::<T>(), false)
    }

    #[inline(always)]
    unsafe fn resolve_column(chunk: &Chunk, component_index: u8) -> *mut u8 {
        debug_assert_ne!(component_index, u8::MAX);
        chunk.column_ptr(component_index as usize)
    }

    #[inline(always)]
    unsafe fn slice_from_raw<'w>(ptr: *mut u8, start: usize, len: usize) -> Self::Slice<'w> {
        // SAFETY: the QueryParam contract guarantees an initialized, aligned
        // column and an in-bounds `start..start + len` range.
        unsafe { slice::from_raw_parts((ptr as *const T).add(start), len) }
    }

    #[inline(always)]
    unsafe fn item_from_raw<'w>(ptr: *mut u8, index: usize) -> Self::Item<'w> {
        // SAFETY: the QueryParam contract guarantees that `index` selects a
        // live, aligned `T` and that shared access is permitted.
        unsafe { &*((ptr as *const T).add(index)) }
    }

    #[inline(always)]
    unsafe fn prepare_entity_fetch(ptr: *mut u8) -> Self::EntityFetch {
        ptr.cast::<T>().cast_const()
    }

    #[inline(always)]
    unsafe fn item_from_entity_fetch<'w>(
        fetch: &Self::EntityFetch,
        index: usize,
    ) -> Self::Item<'w> {
        // SAFETY: the EntityFetchSpec caller guarantees a live, aligned row
        // and shared access to this typed component base.
        unsafe { &*fetch.add(index) }
    }
}

unsafe impl<T: 'static> ReadOnlyQueryParam for &T {}

unsafe impl<T: 'static> QueryParam for &mut T {
    type Slice<'w> = &'w mut [T];
    type Item<'w> = &'w mut T;
    type EntityFetch = *mut T;

    #[inline(always)]
    fn component() -> QueryComponent {
        QueryComponent::new(component_type::<T>(), true)
    }

    #[inline(always)]
    unsafe fn resolve_column(chunk: &Chunk, component_index: u8) -> *mut u8 {
        debug_assert_ne!(component_index, u8::MAX);
        chunk.column_ptr(component_index as usize)
    }

    #[inline(always)]
    unsafe fn slice_from_raw<'w>(ptr: *mut u8, start: usize, len: usize) -> Self::Slice<'w> {
        // SAFETY: the QueryParam contract guarantees an initialized, aligned,
        // exclusively borrowed range for the returned mutable slice.
        unsafe { slice::from_raw_parts_mut((ptr as *mut T).add(start), len) }
    }

    #[inline(always)]
    unsafe fn item_from_raw<'w>(ptr: *mut u8, index: usize) -> Self::Item<'w> {
        // SAFETY: the QueryParam contract guarantees a live, aligned `T` at
        // `index` and exclusive access for the yielded item.
        unsafe { &mut *((ptr as *mut T).add(index)) }
    }

    #[inline(always)]
    unsafe fn prepare_entity_fetch(ptr: *mut u8) -> Self::EntityFetch {
        ptr.cast::<T>()
    }

    #[inline(always)]
    unsafe fn item_from_entity_fetch<'w>(
        fetch: &Self::EntityFetch,
        index: usize,
    ) -> Self::Item<'w> {
        // SAFETY: the EntityFetchSpec caller guarantees a live, aligned row
        // and exclusive access to this typed component base.
        unsafe { &mut *fetch.add(index) }
    }
}

unsafe impl<T: 'static> QueryParam for Option<&T> {
    type Slice<'w> = Option<&'w [T]>;
    type Item<'w> = Option<&'w T>;
    type EntityFetch = *const T;

    #[inline(always)]
    fn component() -> QueryComponent {
        QueryComponent::optional(component_type::<T>(), false)
    }

    #[inline(always)]
    unsafe fn slice_from_raw<'w>(ptr: *mut u8, start: usize, len: usize) -> Self::Slice<'w> {
        if ptr.is_null() {
            None
        } else {
            // SAFETY: a non-null optional pointer obeys the same initialized,
            // aligned and in-bounds contract as a required shared column.
            Some(unsafe { slice::from_raw_parts((ptr as *const T).add(start), len) })
        }
    }

    #[inline(always)]
    unsafe fn item_from_raw<'w>(ptr: *mut u8, index: usize) -> Self::Item<'w> {
        if ptr.is_null() {
            None
        } else {
            // SAFETY: a non-null optional pointer identifies a live shared
            // component row at the caller-validated index.
            Some(unsafe { &*((ptr as *const T).add(index)) })
        }
    }

    #[inline(always)]
    unsafe fn prepare_entity_fetch(ptr: *mut u8) -> Self::EntityFetch {
        ptr.cast::<T>().cast_const()
    }

    #[inline(always)]
    unsafe fn item_from_entity_fetch<'w>(
        fetch: &Self::EntityFetch,
        index: usize,
    ) -> Self::Item<'w> {
        if fetch.is_null() {
            None
        } else {
            // SAFETY: a non-null optional fetch obeys the live shared-row
            // contract established while the route cache was prepared.
            Some(unsafe { &*fetch.add(index) })
        }
    }
}

unsafe impl<T: 'static> ReadOnlyQueryParam for Option<&T> {}

unsafe impl<T: 'static> QueryParam for Option<&mut T> {
    type Slice<'w> = Option<&'w mut [T]>;
    type Item<'w> = Option<&'w mut T>;
    type EntityFetch = *mut T;

    #[inline(always)]
    fn component() -> QueryComponent {
        QueryComponent::optional(component_type::<T>(), true)
    }

    #[inline(always)]
    unsafe fn slice_from_raw<'w>(ptr: *mut u8, start: usize, len: usize) -> Self::Slice<'w> {
        if ptr.is_null() {
            None
        } else {
            // SAFETY: a non-null optional pointer obeys the caller-provided
            // exclusive, initialized and in-bounds range contract.
            Some(unsafe { slice::from_raw_parts_mut((ptr as *mut T).add(start), len) })
        }
    }

    #[inline(always)]
    unsafe fn item_from_raw<'w>(ptr: *mut u8, index: usize) -> Self::Item<'w> {
        if ptr.is_null() {
            None
        } else {
            // SAFETY: a non-null optional pointer identifies a live component
            // row for which the query executor holds exclusive access.
            Some(unsafe { &mut *((ptr as *mut T).add(index)) })
        }
    }

    #[inline(always)]
    unsafe fn prepare_entity_fetch(ptr: *mut u8) -> Self::EntityFetch {
        ptr.cast::<T>()
    }

    #[inline(always)]
    unsafe fn item_from_entity_fetch<'w>(
        fetch: &Self::EntityFetch,
        index: usize,
    ) -> Self::Item<'w> {
        if fetch.is_null() {
            None
        } else {
            // SAFETY: a non-null optional fetch obeys the live exclusive-row
            // contract established while the route cache was prepared.
            Some(unsafe { &mut *fetch.add(index) })
        }
    }
}

// ---------------------------------------------------------------------------
// QuerySpec
// ---------------------------------------------------------------------------

/// Type-level description of a typed query.
///
/// This is an unsafe implementation contract. Use the built-in query
/// parameter forms or `#[derive(QueryData)]` rather than implementing it
/// manually.
///
/// # Safety
///
/// Implementations must describe every component access accurately, preserve
/// the aliasing mode of each parameter, and only construct references within
/// the storage and lifetime represented by the supplied chunk.
///
/// [`Arity`](Self::Arity), [`ChunkArgs`](Self::ChunkArgs), and
/// [`ItemArgs`](Self::ItemArgs) must describe the same query parameters, in the
/// same order, as the descriptor. The two `into_*_args` methods must only
/// unpack their input: they must neither duplicate nor discard component
/// references, especially mutable references.
///
/// Implementations of
/// [`for_each_entity_raw_parts`](Self::for_each_entity_raw_parts) must invoke
/// the callback exactly once for every row in the requested range and must not
/// invoke it for rows outside that range.
pub unsafe trait QuerySpec {
    /// Number of independently typed arguments exposed by iteration callbacks.
    #[doc(hidden)]
    type Arity;

    type Chunk<'w>;
    type Item<'w>;

    /// Flattened callback arguments corresponding to [`Chunk`](Self::Chunk).
    #[doc(hidden)]
    type ChunkArgs<'w>;

    /// Flattened callback arguments corresponding to [`Item`](Self::Item).
    #[doc(hidden)]
    type ItemArgs<'w>;

    fn descriptor() -> QueryDescriptor;

    /// Converts a chunk value into the arguments passed to a chunk callback.
    #[doc(hidden)]
    fn into_chunk_args<'w>(chunk: Self::Chunk<'w>) -> Self::ChunkArgs<'w>;

    /// Converts an item value into the arguments passed to an entity callback.
    #[doc(hidden)]
    fn into_item_args<'w>(item: Self::Item<'w>) -> Self::ItemArgs<'w>;

    /// Builds the typed slice view for one matching chunk.
    ///
    /// # Safety
    ///
    /// `component_indices` must come from this specification's descriptor for
    /// `chunk`, and the caller must uphold every declared shared/exclusive
    /// component access for `'w`.
    unsafe fn chunk_from_raw<'w>(chunk: &'w Chunk, component_indices: &[u8]) -> Self::Chunk<'w>;

    /// Builds a typed slice view over a prevalidated subrange of raw columns.
    ///
    /// # Safety
    ///
    /// Every pointer must address the corresponding descriptor column, the
    /// range must be initialized and in bounds, and accesses must be disjoint
    /// wherever the specification declares mutable references.
    #[doc(hidden)]
    unsafe fn chunk_from_raw_parts<'w>(
        component_ptrs: &[*mut u8],
        start: usize,
        len: usize,
    ) -> Self::Chunk<'w>;

    /// Visits a prevalidated subrange of raw component columns entity by entity.
    ///
    /// This is the entity-level counterpart to [`chunk_from_raw_parts`](Self::chunk_from_raw_parts)
    /// used by the parallel stripe runner.
    ///
    /// # Safety
    ///
    /// Every pointer must address the corresponding descriptor column, the
    /// range must be initialized and in bounds, and concurrent calls must use
    /// disjoint ranges for every mutable component access.
    #[doc(hidden)]
    unsafe fn for_each_entity_raw_parts<'w, Func>(
        component_ptrs: &[*mut u8],
        start: usize,
        len: usize,
        f: &mut Func,
    ) where
        Func: FnMut(Self::Item<'w>);

    /// Visits every initialized entity row in one matching chunk.
    ///
    /// # Safety
    ///
    /// `component_indices` must match this specification and `chunk`; the
    /// caller must also uphold the aliasing contract for all yielded items
    /// until each closure invocation returns.
    unsafe fn for_each_entity<'w, Func>(chunk: &'w Chunk, component_indices: &[u8], f: &mut Func)
    where
        Func: FnMut(Self::Item<'w>);
}

/// Marker for query specifications that never yield mutable references.
///
/// # Safety
///
/// Every access declared by the implementing [`QuerySpec`] must be read-only.
pub unsafe trait ReadOnlyQuerySpec: QuerySpec {}

unsafe impl<P: QueryParam> QuerySpec for P {
    type Arity = Arity1;
    type Chunk<'w> = P::Slice<'w>;
    type Item<'w> = P::Item<'w>;
    type ChunkArgs<'w> = P::Slice<'w>;
    type ItemArgs<'w> = P::Item<'w>;

    #[inline(always)]
    fn into_chunk_args<'w>(chunk: Self::Chunk<'w>) -> Self::ChunkArgs<'w> {
        chunk
    }

    #[inline(always)]
    fn into_item_args<'w>(item: Self::Item<'w>) -> Self::ItemArgs<'w> {
        item
    }

    #[inline(always)]
    fn descriptor() -> QueryDescriptor {
        let mut components = SmallVec::new();
        components.push(P::component());
        QueryDescriptor::new(components)
    }

    #[inline(always)]
    unsafe fn chunk_from_raw<'w>(chunk: &'w Chunk, component_indices: &[u8]) -> Self::Chunk<'w> {
        // SAFETY: QuerySpec callers provide the descriptor-matched column map;
        // the cached index and full live chunk range therefore satisfy P.
        unsafe {
            P::slice_from_raw(
                P::resolve_column(chunk, *component_indices.get_unchecked(0)),
                0,
                chunk.entity_count,
            )
        }
    }

    #[inline(always)]
    unsafe fn chunk_from_raw_parts<'w>(
        component_ptrs: &[*mut u8],
        start: usize,
        len: usize,
    ) -> Self::Chunk<'w> {
        // SAFETY: the caller guarantees that slot zero belongs to P and that
        // the requested range is initialized, in bounds, and correctly aliased.
        unsafe { P::slice_from_raw(component_ptrs[0], start, len) }
    }

    #[inline(always)]
    unsafe fn for_each_entity<'w, Func>(chunk: &'w Chunk, component_indices: &[u8], f: &mut Func)
    where
        Func: FnMut(Self::Item<'w>),
    {
        // SAFETY: the matched column map resolves P's live column, and every
        // loop index is within the chunk's initialized entity range.
        unsafe {
            let base = P::resolve_column(chunk, *component_indices.get_unchecked(0));
            for entity_index in 0..chunk.entity_count {
                f(P::item_from_raw(base, entity_index));
            }
        }
    }

    #[inline(always)]
    unsafe fn for_each_entity_raw_parts<'w, Func>(
        component_ptrs: &[*mut u8],
        start: usize,
        len: usize,
        f: &mut Func,
    ) where
        Func: FnMut(Self::Item<'w>),
    {
        // SAFETY: the caller supplies P's column pointer and an initialized,
        // in-bounds range while upholding P's aliasing mode.
        unsafe {
            let base = component_ptrs[0];
            for entity_index in start..start + len {
                f(P::item_from_raw(base, entity_index));
            }
        }
    }
}

unsafe impl<P: ReadOnlyQueryParam> ReadOnlyQuerySpec for P {}

unsafe impl<P: QueryParam> EntityFetchSpec for P {
    type Fetch = P::EntityFetch;

    #[inline(always)]
    unsafe fn prepare_fetch(chunk: &Chunk, component_indices: &[u8]) -> Self::Fetch {
        // SAFETY: the caller supplies P's descriptor-matched component index
        // for this live chunk, including the optional sentinel when absent.
        unsafe {
            P::prepare_entity_fetch(P::resolve_column(
                chunk,
                *component_indices.get_unchecked(0),
            ))
        }
    }

    #[inline(always)]
    unsafe fn fetch_item<'w>(fetch: &Self::Fetch, entity_index: usize) -> Self::Item<'w> {
        // SAFETY: the caller guarantees this typed base belongs to P's live
        // matching chunk and that entity_index selects an initialized row.
        unsafe { P::item_from_entity_fetch(fetch, entity_index) }
    }
}

macro_rules! query_arity {
    ($A:ident) => {
        super::Arity1
    };
    ($A:ident, $B:ident) => {
        super::Arity2
    };
    ($A:ident, $B:ident, $C:ident) => {
        super::Arity3
    };
    ($A:ident, $B:ident, $C:ident, $D:ident) => {
        super::Arity4
    };
    ($A:ident, $B:ident, $C:ident, $D:ident, $E:ident) => {
        super::Arity5
    };
    ($A:ident, $B:ident, $C:ident, $D:ident, $E:ident, $F:ident) => {
        super::Arity6
    };
    ($A:ident, $B:ident, $C:ident, $D:ident, $E:ident, $F:ident, $G:ident) => {
        super::Arity7
    };
    ($A:ident, $B:ident, $C:ident, $D:ident, $E:ident, $F:ident, $G:ident, $H:ident) => {
        super::Arity8
    };
    ($A:ident, $B:ident, $C:ident, $D:ident, $E:ident, $F:ident, $G:ident, $H:ident, $I:ident) => {
        super::Arity9
    };
    ($A:ident, $B:ident, $C:ident, $D:ident, $E:ident, $F:ident, $G:ident, $H:ident, $I:ident, $J:ident) => {
        super::Arity10
    };
    ($A:ident, $B:ident, $C:ident, $D:ident, $E:ident, $F:ident, $G:ident, $H:ident, $I:ident, $J:ident, $K:ident) => {
        super::Arity11
    };
    ($A:ident, $B:ident, $C:ident, $D:ident, $E:ident, $F:ident, $G:ident, $H:ident, $I:ident, $J:ident, $K:ident, $L:ident) => {
        super::Arity12
    };
    ($A:ident, $B:ident, $C:ident, $D:ident, $E:ident, $F:ident, $G:ident, $H:ident, $I:ident, $J:ident, $K:ident, $L:ident, $M:ident) => {
        super::Arity13
    };
    ($A:ident, $B:ident, $C:ident, $D:ident, $E:ident, $F:ident, $G:ident, $H:ident, $I:ident, $J:ident, $K:ident, $L:ident, $M:ident, $N:ident) => {
        super::Arity14
    };
    ($A:ident, $B:ident, $C:ident, $D:ident, $E:ident, $F:ident, $G:ident, $H:ident, $I:ident, $J:ident, $K:ident, $L:ident, $M:ident, $N:ident, $O:ident) => {
        super::Arity15
    };
    ($A:ident, $B:ident, $C:ident, $D:ident, $E:ident, $F:ident, $G:ident, $H:ident, $I:ident, $J:ident, $K:ident, $L:ident, $M:ident, $N:ident, $O:ident, $P:ident) => {
        super::Arity16
    };
}

macro_rules! impl_query_spec_tuple {
    ($(($Param:ident, $base:ident, $index:tt)),+ $(,)?) => {
        unsafe impl<$($Param: QueryParam),+> QuerySpec for ($($Param,)+) {
            type Arity = query_arity!($($Param),+);
            type Chunk<'w> = ($($Param::Slice<'w>,)+);
            type Item<'w> = ($($Param::Item<'w>,)+);
            type ChunkArgs<'w> = Self::Chunk<'w>;
            type ItemArgs<'w> = Self::Item<'w>;

            #[inline(always)]
            fn into_chunk_args<'w>(chunk: Self::Chunk<'w>) -> Self::ChunkArgs<'w> {
                chunk
            }

            #[inline(always)]
            fn into_item_args<'w>(item: Self::Item<'w>) -> Self::ItemArgs<'w> {
                item
            }

            #[inline(always)]
            fn descriptor() -> QueryDescriptor {
                let mut components = SmallVec::new();
                $(components.push($Param::component());)+
                QueryDescriptor::new(components)
            }

            #[inline(always)]
            unsafe fn chunk_from_raw<'w>(
                chunk: &'w Chunk,
                component_indices: &[u8],
            ) -> Self::Chunk<'w> {
                // SAFETY: the descriptor-matched map has one valid slot per
                // parameter, and the live chunk range satisfies each parameter.
                unsafe {
                    (
                        $(
                            $Param::slice_from_raw(
                                $Param::resolve_column(
                                    chunk,
                                    *component_indices.get_unchecked($index),
                                ),
                                0,
                                chunk.entity_count,
                            ),
                        )+
                    )
                }
            }

            #[inline(always)]
            unsafe fn chunk_from_raw_parts<'w>(
                component_ptrs: &[*mut u8],
                start: usize,
                len: usize,
            ) -> Self::Chunk<'w> {
                // SAFETY: pointer slots correspond to their tuple parameters;
                // the caller validates the range and combined aliasing contract.
                unsafe {
                    (
                        $(
                            $Param::slice_from_raw(component_ptrs[$index], start, len),
                        )+
                    )
                }
            }

            #[inline(always)]
            unsafe fn for_each_entity<'w, Func>(
                chunk: &'w Chunk,
                component_indices: &[u8],
                f: &mut Func,
            )
            where
                Func: FnMut(Self::Item<'w>),
            {
                // SAFETY: cached indices match their tuple parameters, every
                // loop index is live, and QuerySpec prevents alias conflicts.
                unsafe {
                    $(let $base = $Param::resolve_column(
                        chunk,
                        *component_indices.get_unchecked($index),
                    );)+

                    for entity_index in 0..chunk.entity_count {
                        f((
                            $(
                                $Param::item_from_raw($base, entity_index),
                            )+
                        ));
                    }
                }
            }

            #[inline(always)]
            unsafe fn for_each_entity_raw_parts<'w, Func>(
                component_ptrs: &[*mut u8],
                start: usize,
                len: usize,
                f: &mut Func,
            )
            where
                Func: FnMut(Self::Item<'w>),
            {
                // SAFETY: pointer slots match their tuple parameters, and the
                // caller provides a live in-bounds range with valid aliasing.
                unsafe {
                    $(let $base = component_ptrs[$index];)+

                    for entity_index in start..start + len {
                        f((
                            $(
                                $Param::item_from_raw($base, entity_index),
                            )+
                        ));
                    }
                }
            }
        }

        unsafe impl<$($Param: ReadOnlyQueryParam),+> ReadOnlyQuerySpec for ($($Param,)+) {}

        unsafe impl<$($Param: QueryParam),+> EntityFetchSpec for ($($Param,)+) {
            type Fetch = ($($Param::EntityFetch,)+);

            #[inline(always)]
            unsafe fn prepare_fetch(
                chunk: &Chunk,
                component_indices: &[u8],
            ) -> Self::Fetch {
                // SAFETY: every cached index belongs to its tuple parameter
                // for this matching live chunk. Optional sentinels are
                // converted into nullable typed bases.
                unsafe {
                    (
                        $(
                            $Param::prepare_entity_fetch(
                                $Param::resolve_column(
                                    chunk,
                                    *component_indices.get_unchecked($index),
                                ),
                            ),
                        )+
                    )
                }
            }

            #[inline(always)]
            unsafe fn fetch_item<'w>(
                fetch: &Self::Fetch,
                entity_index: usize,
            ) -> Self::Item<'w> {
                // SAFETY: fetch contains the descriptor-matched typed bases
                // for this live row and the caller upholds combined aliasing.
                unsafe {
                    (
                        $(
                            $Param::item_from_entity_fetch(
                                &fetch.$index,
                                entity_index,
                            ),
                        )+
                    )
                }
            }
        }
    };
}

impl_query_spec_tuple!((A, a, 0), (B, b, 1));
impl_query_spec_tuple!((A, a, 0), (B, b, 1), (C, c, 2));
impl_query_spec_tuple!((A, a, 0), (B, b, 1), (C, c, 2), (D, d, 3));
impl_query_spec_tuple!((A, a, 0), (B, b, 1), (C, c, 2), (D, d, 3), (E, e, 4));
impl_query_spec_tuple!(
    (A, a, 0),
    (B, b, 1),
    (C, c, 2),
    (D, d, 3),
    (E, e, 4),
    (F, f, 5)
);
impl_query_spec_tuple!(
    (A, a, 0),
    (B, b, 1),
    (C, c, 2),
    (D, d, 3),
    (E, e, 4),
    (F, f, 5),
    (G, g, 6)
);
impl_query_spec_tuple!(
    (A, a, 0),
    (B, b, 1),
    (C, c, 2),
    (D, d, 3),
    (E, e, 4),
    (F, f, 5),
    (G, g, 6),
    (H, h, 7)
);
impl_query_spec_tuple!(
    (A, a, 0),
    (B, b, 1),
    (C, c, 2),
    (D, d, 3),
    (E, e, 4),
    (F, f, 5),
    (G, g, 6),
    (H, h, 7),
    (I, i, 8)
);
impl_query_spec_tuple!(
    (A, a, 0),
    (B, b, 1),
    (C, c, 2),
    (D, d, 3),
    (E, e, 4),
    (F, f, 5),
    (G, g, 6),
    (H, h, 7),
    (I, i, 8),
    (J, j, 9)
);
impl_query_spec_tuple!(
    (A, a, 0),
    (B, b, 1),
    (C, c, 2),
    (D, d, 3),
    (E, e, 4),
    (F, f, 5),
    (G, g, 6),
    (H, h, 7),
    (I, i, 8),
    (J, j, 9),
    (K, k, 10)
);
impl_query_spec_tuple!(
    (A, a, 0),
    (B, b, 1),
    (C, c, 2),
    (D, d, 3),
    (E, e, 4),
    (F, f, 5),
    (G, g, 6),
    (H, h, 7),
    (I, i, 8),
    (J, j, 9),
    (K, k, 10),
    (L, l, 11)
);
impl_query_spec_tuple!(
    (A, a, 0),
    (B, b, 1),
    (C, c, 2),
    (D, d, 3),
    (E, e, 4),
    (F, f, 5),
    (G, g, 6),
    (H, h, 7),
    (I, i, 8),
    (J, j, 9),
    (K, k, 10),
    (L, l, 11),
    (M, m, 12)
);
impl_query_spec_tuple!(
    (A, a, 0),
    (B, b, 1),
    (C, c, 2),
    (D, d, 3),
    (E, e, 4),
    (F, f, 5),
    (G, g, 6),
    (H, h, 7),
    (I, i, 8),
    (J, j, 9),
    (K, k, 10),
    (L, l, 11),
    (M, m, 12),
    (N, n, 13)
);
impl_query_spec_tuple!(
    (A, a, 0),
    (B, b, 1),
    (C, c, 2),
    (D, d, 3),
    (E, e, 4),
    (F, f, 5),
    (G, g, 6),
    (H, h, 7),
    (I, i, 8),
    (J, j, 9),
    (K, k, 10),
    (L, l, 11),
    (M, m, 12),
    (N, n, 13),
    (O, o, 14)
);
impl_query_spec_tuple!(
    (A, a, 0),
    (B, b, 1),
    (C, c, 2),
    (D, d, 3),
    (E, e, 4),
    (F, f, 5),
    (G, g, 6),
    (H, h, 7),
    (I, i, 8),
    (J, j, 9),
    (K, k, 10),
    (L, l, 11),
    (M, m, 12),
    (N, n, 13),
    (O, o, 14),
    (P, p, 15)
);
