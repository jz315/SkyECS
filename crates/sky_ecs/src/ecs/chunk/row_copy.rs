use super::Chunk;
use std::ptr;

impl Chunk {
    #[inline(always)]
    fn copy_entity_components(
        source: &Chunk,
        source_index: usize,
        target: &Chunk,
        target_index: usize,
    ) {
        #[inline(always)]
        // Safety: the source row must contain a live component, the target row
        // must be an in-bounds uninitialized or already-dropped slot, and the
        // two byte ranges must not overlap.
        unsafe fn copy_column(
            source: &Chunk,
            source_index: usize,
            target: &Chunk,
            target_index: usize,
            component_index: usize,
            component_size: usize,
        ) {
            if component_size == 0 {
                return;
            }
            unsafe {
                ptr::copy_nonoverlapping(
                    source.component_ptr_unchecked(component_index, source_index),
                    target.component_ptr_unchecked(component_index, target_index),
                    component_size,
                );
            }
        }

        macro_rules! copy_columns {
            ($components:ident; $($index:literal),+ $(,)?) => {{
                $(
                    // SAFETY: the fixed arity match proves the component
                    // index exists, and row-copy callers provide valid,
                    // non-overlapping source and target rows.
                    unsafe {
                        copy_column(
                            source,
                            source_index,
                            target,
                            target_index,
                            $index,
                            $components[$index].size,
                        );
                    }
                )+
            }};
        }

        debug_assert_eq!(source.archetype.id(), target.archetype.id());
        let components = source.archetype.components.as_slice();
        match components.len() {
            0 => {}
            1 => copy_columns!(components; 0),
            2 => copy_columns!(components; 0, 1),
            3 => copy_columns!(components; 0, 1, 2),
            4 => copy_columns!(components; 0, 1, 2, 3),
            _ => {
                for (component_index, component) in components.iter().enumerate() {
                    // SAFETY: iteration supplies a valid component index, and
                    // row-copy callers provide valid, non-overlapping source
                    // and target rows.
                    unsafe {
                        copy_column(
                            source,
                            source_index,
                            target,
                            target_index,
                            component_index,
                            component.size,
                        );
                    }
                }
            }
        }
    }

    /// Bitwise-copies one row into an uninitialized or already-dropped row in
    /// the same chunk.
    #[inline(always)]
    pub(crate) fn copy_entity_within(&mut self, source_index: usize, target_index: usize) {
        if source_index == target_index {
            return;
        }

        Self::copy_entity_components(self, source_index, self, target_index);
        self.entities[target_index] = self.entities[source_index];
    }

    /// Bitwise-copies one row from another chunk of the same archetype into an
    /// uninitialized or already-dropped target row.
    #[inline(always)]
    pub(crate) fn copy_entity_from(
        &mut self,
        source: &Chunk,
        source_index: usize,
        target_index: usize,
    ) {
        debug_assert_eq!(self.archetype.id(), source.archetype.id());

        Self::copy_entity_components(source, source_index, self, target_index);
        self.entities[target_index] = source.entities[source_index];
    }
}
