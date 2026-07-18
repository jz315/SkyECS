#![deny(unsafe_op_in_unsafe_fn)]

use super::{create_archetype, Archetype, Chunk, MAX_COMPONENTS};
use crate::ecs::{component_type, ComponentType};
use rustc_hash::FxHashMap;
use smallvec::{smallvec, SmallVec};
use std::{
    any::TypeId,
    cell::{Cell, RefCell},
    ptr,
    sync::RwLock,
};

lazy_static::lazy_static! {
    static ref BUNDLE_META: RwLock<FxHashMap<TypeId, &'static BundleMeta>> = RwLock::new(FxHashMap::default());
}

thread_local! {
    static LAST_BUNDLE_META: Cell<Option<(TypeId, &'static BundleMeta)>> = const { Cell::new(None) };
    static LOCAL_BUNDLE_META: RefCell<FxHashMap<TypeId, &'static BundleMeta>> = RefCell::new(FxHashMap::default());
}

pub(crate) struct BundleMeta {
    pub archetype: Archetype,
    pub columns: SmallVec<[(usize, usize); MAX_COMPONENTS]>,
}

mod sealed {
    pub trait BundleSealed {
        /// Writes one bundle through pointers that already identify its next
        /// row, then advances every pointer to the following row.
        ///
        /// # Safety
        ///
        /// `cursors` and `columns` must be in bundle tuple order. Every cursor
        /// must identify a valid, properly aligned, uninitialized component
        /// slot, and the corresponding size in `columns` must match that
        /// component type.
        unsafe fn write_fast_cursor(self, cursors: &mut [*mut u8], columns: &[(usize, usize)]);
    }
}

fn assert_unique_types(types: &[ComponentType]) {
    for (index, ty) in types.iter().enumerate() {
        for other in &types[(index + 1)..] {
            if ty.id() == other.id() {
                panic!(
                    "duplicate component type `{}` is not supported in bundles",
                    ty.name
                );
            }
        }
    }
}

fn bundle_meta<B: 'static>(
    make_types: impl FnOnce() -> SmallVec<[ComponentType; MAX_COMPONENTS]>,
) -> &'static BundleMeta {
    let type_id = TypeId::of::<B>();

    if let Some((cached_type_id, meta)) = LAST_BUNDLE_META.get() {
        if cached_type_id == type_id {
            return meta;
        }
    }

    if let Some(meta) = LOCAL_BUNDLE_META.with(|cache| cache.borrow().get(&type_id).copied()) {
        LAST_BUNDLE_META.set(Some((type_id, meta)));
        return meta;
    }

    if let Some(meta) = BUNDLE_META.read().unwrap().get(&type_id).copied() {
        LOCAL_BUNDLE_META.with(|cache| {
            cache.borrow_mut().insert(type_id, meta);
        });
        LAST_BUNDLE_META.set(Some((type_id, meta)));
        return meta;
    }

    let mut cache = BUNDLE_META.write().unwrap();
    if let Some(meta) = cache.get(&type_id).copied() {
        LOCAL_BUNDLE_META.with(|cache| {
            cache.borrow_mut().insert(type_id, meta);
        });
        LAST_BUNDLE_META.set(Some((type_id, meta)));
        return meta;
    }

    let types = make_types();
    assert_unique_types(&types);
    let mut builder = create_archetype();
    for ty in &types {
        builder = builder.add_component(*ty);
    }
    let archetype = builder.build();
    let columns = types
        .iter()
        .map(|ty| {
            let component_index = archetype.query_component_index(ty).unwrap();
            (component_index, ty.size)
        })
        .collect();
    let meta = Box::leak(Box::new(BundleMeta { archetype, columns }));

    let meta = *cache.entry(type_id).or_insert(meta);
    LOCAL_BUNDLE_META.with(|cache| {
        cache.borrow_mut().insert(type_id, meta);
    });
    LAST_BUNDLE_META.set(Some((type_id, meta)));
    meta
}

/// Trait implemented by component tuples for spawning entities.
///
/// You do not need to implement this manually — it is auto-implemented
/// for tuples of up to 16 `'static` types. Both `Copy` and non-`Copy`
/// types are supported.
///
/// This trait is sealed so Sky ECS can keep the storage invariants behind
/// [`World::spawn`](crate::ecs::World::spawn) sound.
pub trait Bundle: sealed::BundleSealed + 'static {
    fn cached_meta() -> (Archetype, &'static [(usize, usize)]);

    fn archetype() -> Archetype;
    /// # Safety
    ///
    /// `entity_index` must be a valid slot within `chunk` and the
    /// chunk's archetype must match the bundle's archetype.
    unsafe fn write(self, chunk: &mut Chunk, entity_index: usize);

    /// Fast write using pre-computed column indices. Skips binary search.
    ///
    /// # Safety
    ///
    /// `entity_index` must be a valid uninitialized slot within `chunk`.
    /// `columns` must come from this bundle's [`cached_meta`](Self::cached_meta)
    /// result and therefore match the chunk archetype and tuple element order.
    unsafe fn write_fast(self, chunk: &mut Chunk, entity_index: usize, columns: &[(usize, usize)]);
}

macro_rules! impl_bundle_tuple {
    ($(($Type:ident, $value:ident, $idx:tt)),+ $(,)?) => {
        impl<$($Type: 'static),+> sealed::BundleSealed for ($($Type,)+) {
            #[inline(always)]
            unsafe fn write_fast_cursor(
                self,
                cursors: &mut [*mut u8],
                columns: &[(usize, usize)],
            ) {
                let ($($value,)+) = self;

                $(
                    let (_, comp_size) = columns[$idx];
                    let destination = cursors[$idx];
                    // SAFETY: the method contract guarantees that every
                    // cursor identifies the next uninitialized slot for its
                    // tuple component. Advancing by the cached component size
                    // preserves that invariant for the following row.
                    unsafe {
                        ptr::write(destination as *mut $Type, $value);
                        cursors[$idx] = destination.add(comp_size);
                    }
                )+
            }
        }

        impl<$($Type: 'static),+> Bundle for ($($Type,)+) {
            fn cached_meta() -> (Archetype, &'static [(usize, usize)]) {
                let meta = bundle_meta::<Self>(|| smallvec![$(component_type::<$Type>()),+]);
                (meta.archetype, &meta.columns)
            }

            fn archetype() -> Archetype {
                Self::cached_meta().0
            }

            unsafe fn write(self, chunk: &mut Chunk, entity_index: usize) {
                let (_, columns) = Self::cached_meta();
                // SAFETY: this method has the same slot/archetype contract as
                // `write_fast`, and these columns are this bundle's cache.
                unsafe { self.write_fast(chunk, entity_index, columns) };
            }

            unsafe fn write_fast(self, chunk: &mut Chunk, entity_index: usize, columns: &[(usize, usize)]) {
                let ($($value,)+) = self;

                $(
                    let (component_index, comp_size) = columns[$idx];
                    let col_ptr = chunk.column_ptr(component_index);
                    // SAFETY: the method contract guarantees that `columns`
                    // describes this tuple and `entity_index` is a valid,
                    // uninitialized slot in the matching chunk.
                    unsafe {
                        ptr::write(
                            col_ptr.add(comp_size * entity_index) as *mut $Type,
                            $value,
                        );
                    }
                )+
            }
        }
    };
}

impl_bundle_tuple!((A, a, 0));
impl_bundle_tuple!((A, a, 0), (B, b, 1));
impl_bundle_tuple!((A, a, 0), (B, b, 1), (C, c, 2));
impl_bundle_tuple!((A, a, 0), (B, b, 1), (C, c, 2), (D, d, 3));
impl_bundle_tuple!((A, a, 0), (B, b, 1), (C, c, 2), (D, d, 3), (E, e, 4));
impl_bundle_tuple!(
    (A, a, 0),
    (B, b, 1),
    (C, c, 2),
    (D, d, 3),
    (E, e, 4),
    (F, f, 5)
);
impl_bundle_tuple!(
    (A, a, 0),
    (B, b, 1),
    (C, c, 2),
    (D, d, 3),
    (E, e, 4),
    (F, f, 5),
    (G, g, 6)
);
impl_bundle_tuple!(
    (A, a, 0),
    (B, b, 1),
    (C, c, 2),
    (D, d, 3),
    (E, e, 4),
    (F, f, 5),
    (G, g, 6),
    (H, h, 7)
);
impl_bundle_tuple!(
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
impl_bundle_tuple!(
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
impl_bundle_tuple!(
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
impl_bundle_tuple!(
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
impl_bundle_tuple!(
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
impl_bundle_tuple!(
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
impl_bundle_tuple!(
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
impl_bundle_tuple!(
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

#[cfg(test)]
mod tests {
    use super::Bundle;
    use crate::ecs::World;

    macro_rules! components {
        ($($name:ident = $value:expr),+ $(,)?) => {
            $(
                #[derive(Debug, PartialEq)]
                struct $name(u8);
            )+

            fn spawn_sixteen(world: &mut World) -> crate::ecs::EntityId {
                world.spawn(($($name($value),)+))
            }
        };
    }

    components!(
        C01 = 1,
        C02 = 2,
        C03 = 3,
        C04 = 4,
        C05 = 5,
        C06 = 6,
        C07 = 7,
        C08 = 8,
        C09 = 9,
        C10 = 10,
        C11 = 11,
        C12 = 12,
        C13 = 13,
        C14 = 14,
        C15 = 15,
        C16 = 16,
    );

    #[test]
    fn sixteen_component_tuple_is_a_bundle() {
        fn assert_bundle<B: Bundle>() {}
        assert_bundle::<(
            C01,
            C02,
            C03,
            C04,
            C05,
            C06,
            C07,
            C08,
            C09,
            C10,
            C11,
            C12,
            C13,
            C14,
            C15,
            C16,
        )>();
    }

    #[test]
    fn sixteen_component_bundle_initializes_every_column() {
        let mut world = World::new();
        let entity = spawn_sixteen(&mut world);

        macro_rules! assert_components {
            ($($name:ident = $value:expr),+ $(,)?) => {
                $(assert_eq!(world.get::<$name>(entity), Some(&$name($value)));)+
            };
        }

        assert_components!(
            C01 = 1,
            C02 = 2,
            C03 = 3,
            C04 = 4,
            C05 = 5,
            C06 = 6,
            C07 = 7,
            C08 = 8,
            C09 = 9,
            C10 = 10,
            C11 = 11,
            C12 = 12,
            C13 = 13,
            C14 = 14,
            C15 = 15,
            C16 = 16,
        );
    }
}
