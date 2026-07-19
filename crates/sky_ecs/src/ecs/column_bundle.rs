use super::{Archetype, ArchetypeStorage, Bundle, ChunkRowSpan};
use std::fmt;
use std::ptr;

/// Reports columns with different row counts passed to [`World::spawn_columns`].
///
/// [`World::spawn_columns`]: crate::ecs::World::spawn_columns
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColumnLengthMismatch {
    column_index: usize,
    expected: usize,
    actual: usize,
}

impl ColumnLengthMismatch {
    fn new(column_index: usize, expected: usize, actual: usize) -> Self {
        Self {
            column_index,
            expected,
            actual,
        }
    }

    /// Returns the zero-based index of the mismatching column.
    pub fn column_index(self) -> usize {
        self.column_index
    }

    /// Returns the row count established by the first column.
    pub fn expected(self) -> usize {
        self.expected
    }

    /// Returns the actual row count of the mismatching column.
    pub fn actual(self) -> usize {
        self.actual
    }
}

impl fmt::Display for ColumnLengthMismatch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "component column {} has {} rows, expected {}",
            self.column_index, self.actual, self.expected
        )
    }
}

impl std::error::Error for ColumnLengthMismatch {}

// The sealed implementation contract intentionally names crate-private
// storage types. External crates can use `ColumnBundle` but cannot implement
// this trait or reach these methods.
#[allow(private_interfaces)]
pub(crate) mod sealed {
    use super::*;

    pub trait ColumnBundleSealed {
        fn row_count(&self) -> Result<usize, ColumnLengthMismatch>;

        fn cached_meta() -> (Archetype, &'static [(usize, usize)]);

        /// Moves every source value into the uninitialized destination spans.
        ///
        /// # Safety
        ///
        /// The spans must belong to `storage`, contain exactly `row_count()`
        /// rows, and remain logically unoccupied. `columns` must be the
        /// metadata returned by `cached_meta()` for this column tuple.
        unsafe fn move_into(
            &mut self,
            storage: &mut ArchetypeStorage,
            spans: &[ChunkRowSpan],
            columns: &[(usize, usize)],
        );
    }
}

/// A sealed tuple of one to sixteen component `Vec`s accepted by
/// [`World::spawn_columns`].
///
/// Users do not implement this trait directly. Sky ECS supplies tuple
/// implementations such as `(Vec<Position>, Vec<Velocity>)`.
///
/// [`World::spawn_columns`]: crate::ecs::World::spawn_columns
pub trait ColumnBundle: sealed::ColumnBundleSealed + 'static {}

unsafe fn move_column<T>(
    source: &mut Vec<T>,
    storage: &mut ArchetypeStorage,
    spans: &[ChunkRowSpan],
    component_index: usize,
    component_size: usize,
) {
    debug_assert_eq!(component_size, std::mem::size_of::<T>());
    let mut source_index = 0usize;

    for span in spans {
        let chunk = &mut storage.chunks[span.chunk_index];
        let destination = unsafe {
            chunk
                .column_ptr(component_index)
                .add(component_size * span.first_entity_index)
                .cast::<T>()
        };
        unsafe {
            ptr::copy_nonoverlapping(
                source.as_ptr().add(source_index),
                destination,
                span.entity_count,
            );
        }
        source_index += span.entity_count;
    }

    debug_assert_eq!(source_index, source.len());
    // Ownership of every element now belongs to its destination chunk. Keep
    // the source allocation but prevent its destructor from dropping values a
    // second time.
    unsafe {
        source.set_len(0);
    }
}

macro_rules! impl_column_bundle_tuple {
    ($(($Type:ident, $index:tt)),+ $(,)?) => {
        impl<$($Type: 'static),+> sealed::ColumnBundleSealed for ($(Vec<$Type>,)+) {
            fn row_count(&self) -> Result<usize, ColumnLengthMismatch> {
                let expected = self.0.len();
                $(
                    if self.$index.len() != expected {
                        return Err(ColumnLengthMismatch::new(
                            $index,
                            expected,
                            self.$index.len(),
                        ));
                    }
                )+
                Ok(expected)
            }

            fn cached_meta() -> (Archetype, &'static [(usize, usize)]) {
                <($($Type,)+) as Bundle>::cached_meta()
            }

            #[allow(private_interfaces)]
            unsafe fn move_into(
                &mut self,
                storage: &mut ArchetypeStorage,
                spans: &[ChunkRowSpan],
                columns: &[(usize, usize)],
            ) {
                $(
                    let (component_index, component_size) = columns[$index];
                    unsafe {
                        move_column(
                            &mut self.$index,
                            storage,
                            spans,
                            component_index,
                            component_size,
                        );
                    }
                )+
            }
        }

        impl<$($Type: 'static),+> ColumnBundle for ($(Vec<$Type>,)+) {}
    };
}

impl_column_bundle_tuple!((A, 0));
impl_column_bundle_tuple!((A, 0), (B, 1));
impl_column_bundle_tuple!((A, 0), (B, 1), (C, 2));
impl_column_bundle_tuple!((A, 0), (B, 1), (C, 2), (D, 3));
impl_column_bundle_tuple!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4));
impl_column_bundle_tuple!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5));
impl_column_bundle_tuple!((A, 0), (B, 1), (C, 2), (D, 3), (E, 4), (F, 5), (G, 6));
impl_column_bundle_tuple!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7)
);
impl_column_bundle_tuple!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7),
    (I, 8)
);
impl_column_bundle_tuple!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7),
    (I, 8),
    (J, 9)
);
impl_column_bundle_tuple!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7),
    (I, 8),
    (J, 9),
    (K, 10)
);
impl_column_bundle_tuple!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7),
    (I, 8),
    (J, 9),
    (K, 10),
    (L, 11)
);
impl_column_bundle_tuple!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7),
    (I, 8),
    (J, 9),
    (K, 10),
    (L, 11),
    (M, 12)
);
impl_column_bundle_tuple!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7),
    (I, 8),
    (J, 9),
    (K, 10),
    (L, 11),
    (M, 12),
    (N, 13)
);
impl_column_bundle_tuple!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7),
    (I, 8),
    (J, 9),
    (K, 10),
    (L, 11),
    (M, 12),
    (N, 13),
    (O, 14)
);
impl_column_bundle_tuple!(
    (A, 0),
    (B, 1),
    (C, 2),
    (D, 3),
    (E, 4),
    (F, 5),
    (G, 6),
    (H, 7),
    (I, 8),
    (J, 9),
    (K, 10),
    (L, 11),
    (M, 12),
    (N, 13),
    (O, 14),
    (P, 15)
);
