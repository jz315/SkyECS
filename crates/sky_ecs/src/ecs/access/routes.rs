use crate::ecs::{component_type, EntityId, World};
use core::ptr::NonNull;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ResolveError {
    InvalidEntity,
    MissingComponent,
}

pub(super) struct ComponentRoutes<T> {
    columns: Box<[Option<NonNull<T>>]>,
}

impl<T: 'static> ComponentRoutes<T> {
    pub(super) fn new(world: &World) -> Self {
        let component = component_type::<T>();
        let mut columns = Vec::with_capacity(world.chunk_route_slot_count());
        columns.resize_with(world.chunk_route_slot_count(), || None);

        if let Some(postings) = world.component_posting(&component) {
            for posting_index in 0..postings.len() {
                let entry = postings
                    .entry(posting_index)
                    .expect("posting index must stay in bounds");
                let data = &world.data[entry.data_index()];
                for (chunk_index, chunk) in data.chunks.iter().enumerate() {
                    let chunk_id = data.chunk_id(chunk_index);
                    assert!(chunk_id.is_assigned(), "live chunk must have a route");
                    let column = unsafe {
                        // The posting entry proves this column stores T. Chunk
                        // always owns a non-null, correctly aligned backing block,
                        // including the aligned dangling address used for ZSTs.
                        NonNull::new_unchecked(
                            chunk.column_ptr(entry.column_index() as usize).cast::<T>(),
                        )
                    };
                    debug_assert!(columns[chunk_id.index()].is_none());
                    columns[chunk_id.index()] = Some(column);
                }
            }
        }

        Self {
            columns: columns.into_boxed_slice(),
        }
    }

    #[inline(always)]
    pub(super) fn resolve(
        &self,
        world: &World,
        entity: EntityId,
    ) -> Result<NonNull<T>, ResolveError> {
        let route = world
            .entity_route(entity)
            .ok_or(ResolveError::InvalidEntity)?;

        let column = unsafe {
            // Every live entity route names a registered ChunkId. The route
            // table was sized from the same immutably borrowed World, so its
            // slot index is in bounds for the lifetime of this table.
            *self.columns.get_unchecked(route.chunk_id.index())
        }
        .ok_or(ResolveError::MissingComponent)?;

        let pointer = unsafe {
            // A live entity route names an initialized row in this chunk. The
            // immutable World view prevents relocation while the pointer is
            // being resolved; add(0) also preserves the aligned ZST pointer.
            NonNull::new_unchecked(column.as_ptr().add(route.entity_index))
        };
        Ok(pointer)
    }

    #[cfg(test)]
    pub(super) fn slot_count(&self) -> usize {
        self.columns.len()
    }

    #[cfg(test)]
    pub(super) fn matching_chunk_count(&self) -> usize {
        self.columns
            .iter()
            .filter(|column| column.is_some())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy)]
    struct Position;

    #[derive(Clone, Copy)]
    struct Velocity;

    #[test]
    fn stores_matching_columns_at_stable_chunk_route_ids() {
        let mut world = World::new();
        world.spawn((Position,));
        world.spawn((Position, Velocity));
        world.spawn((Velocity,));

        let routes = ComponentRoutes::<Position>::new(&world);
        let expected_columns = world
            .data
            .iter()
            .filter(|data| {
                data.archetype
                    .query_component_index(&component_type::<Position>())
                    .is_some()
            })
            .map(|data| data.chunks.len())
            .sum::<usize>();

        assert_eq!(routes.slot_count(), world.chunk_route_slot_count());
        assert_eq!(routes.matching_chunk_count(), expected_columns);
        assert_eq!(
            core::mem::size_of::<Option<NonNull<Position>>>(),
            core::mem::size_of::<usize>()
        );
    }
}
