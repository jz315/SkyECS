use super::{component_type, EntityId, World};
use core::marker::PhantomData;
use core::ptr::NonNull;
use core::slice;

type DataColumns<'w, T> = Option<Box<[&'w [T]]>>;
type DataColumnsMut<T> = Option<Box<[ComponentColumn<T>]>>;

struct ComponentColumn<T> {
    ptr: NonNull<T>,
    len: usize,
}

/// A read-only component accessor bound to one [`World`].
///
/// Create an accessor with [`World::accessor`] when a hot path repeatedly
/// looks up the same component type by [`EntityId`]. Construction resolves the
/// component column once for every archetype and stores typed views of the
/// matching chunk columns. Individual calls to [`get`](Self::get) therefore do
/// not need to search archetype metadata again.
///
/// The accessor borrows the world for its entire lifetime. This keeps its
/// cached chunk views valid and prevents structural changes while it is in
/// use. For occasional lookups, prefer [`World::get`].
#[must_use = "component accessors do nothing until get is called"]
pub struct ComponentAccessor<'w, T> {
    world: &'w World,
    columns: Box<[DataColumns<'w, T>]>,
}

impl<'w, T: 'static> ComponentAccessor<'w, T> {
    pub(crate) fn new(world: &'w World) -> Self {
        let component = component_type::<T>();
        let columns = world
            .data
            .iter()
            .map(|data| {
                let component_index = data.archetype.query_component_index(&component)?;
                let chunks = data
                    .chunks
                    .iter()
                    .map(|chunk| unsafe {
                        // The archetype lookup above proves that this column stores T.
                        // Chunk storage remains fixed for 'w because the accessor holds
                        // a shared World borrow, and entity_count is the initialized
                        // prefix of every component column.
                        slice::from_raw_parts(
                            chunk.column_ptr(component_index).cast::<T>(),
                            chunk.entity_count,
                        )
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice();
                Some(chunks)
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self { world, columns }
    }

    /// Returns component `T` on `entity`.
    ///
    /// Returns `None` when the entity is dead, its generation is stale, or its
    /// archetype does not contain `T`.
    #[inline(always)]
    pub fn get(&self, entity: EntityId) -> Option<&'w T> {
        let location = self.world.entity_location(entity)?;

        unsafe {
            // A live EntityRecord always names an existing Data, Chunk, and
            // initialized entity row. The immutable World borrow prevents all
            // structural operations that could invalidate those indices. The
            // optional entry is None exactly when that Data lacks T.
            let chunks = self.columns.get_unchecked(location.data_index).as_ref()?;
            let column: &'w [T] = chunks.get_unchecked(location.chunk_index);
            Some(column.get_unchecked(location.entity_index))
        }
    }
}

/// An exclusive component accessor bound to one [`World`].
///
/// Create one with [`World::accessor_mut`] when a hot path repeatedly updates
/// the same component type by [`EntityId`]. The accessor exclusively borrows
/// the world, and every reference returned by [`get_mut`](Self::get_mut) is
/// tied to the corresponding mutable borrow of the accessor. This prevents
/// overlapping mutable component references through the safe API.
#[must_use = "component accessors do nothing until get_mut is called"]
pub struct ComponentAccessorMut<'w, T> {
    world: NonNull<World>,
    columns: Box<[DataColumnsMut<T>]>,
    marker: PhantomData<&'w mut World>,
}

impl<'w, T: 'static> ComponentAccessorMut<'w, T> {
    pub(crate) fn new(world: &'w mut World) -> Self {
        let world_ptr = NonNull::from(&mut *world);
        let component = component_type::<T>();
        let columns = world
            .data
            .iter()
            .map(|data| {
                let component_index = data.archetype.query_component_index(&component)?;
                let chunks = data
                    .chunks
                    .iter()
                    .map(|chunk| ComponentColumn {
                        ptr: unsafe {
                            // The archetype lookup proves this column stores T,
                            // and Chunk always owns a non-null aligned backing block.
                            NonNull::new_unchecked(chunk.column_ptr(component_index).cast::<T>())
                        },
                        len: chunk.entity_count,
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice();
                Some(chunks)
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self {
            world: world_ptr,
            columns,
            marker: PhantomData,
        }
    }

    /// Returns component `T` on `entity` with exclusive access.
    ///
    /// Returns `None` when the entity is dead, its generation is stale, or its
    /// archetype does not contain `T`.
    #[inline(always)]
    pub fn get_mut(&mut self, entity: EntityId) -> Option<&mut T> {
        let location = unsafe {
            // `marker` retains the exclusive World borrow for 'w, so the World
            // remains alive and no safe structural operation can run.
            self.world.as_ref().entity_location(entity)?
        };

        unsafe {
            // A live EntityRecord names an existing Data, Chunk, and initialized
            // row. The exclusive World borrow keeps those locations stable.
            // The returned reference is tied to `&mut self`, preventing another
            // accessor call until that reference is no longer used.
            let chunks = self.columns.get_unchecked(location.data_index).as_ref()?;
            let column = chunks.get_unchecked(location.chunk_index);
            debug_assert!(location.entity_index < column.len);
            Some(&mut *column.ptr.as_ptr().add(location.entity_index))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Position(u32);

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Velocity(u32);

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Marker;

    #[allow(dead_code)]
    #[derive(Clone, Copy)]
    struct Large([u8; 4 * 1024]);

    #[test]
    fn reads_components_across_archetypes_and_reports_missing_components() {
        let mut world = World::new();
        let position_only = world.spawn((Position(1),));
        let both = world.spawn((Position(2), Velocity(3)));
        let velocity_only = world.spawn((Velocity(4),));

        let positions = world.accessor::<Position>();

        assert_eq!(positions.get(position_only), Some(&Position(1)));
        assert_eq!(positions.get(both), Some(&Position(2)));
        assert_eq!(positions.get(velocity_only), None);
    }

    #[test]
    fn rejects_invalid_and_stale_entity_ids() {
        let mut world = World::new();
        let stale = world.spawn((Position(1),));
        assert!(world.despawn(stale));
        let current = world.spawn((Position(2),));
        assert_eq!(stale.index(), current.index());

        let positions = world.accessor::<Position>();

        assert_eq!(positions.get(stale), None);
        assert_eq!(positions.get(EntityId::new(u32::MAX, 0)), None);
        assert_eq!(positions.get(current), Some(&Position(2)));
    }

    #[test]
    fn reads_small_and_large_chunks() {
        let mut world = World::new();
        let entities: Vec<_> = (0..160)
            .map(|index| world.spawn((Large([index as u8; 4 * 1024]), Position(index))))
            .collect();

        let positions = world.accessor::<Position>();

        for (index, entity) in entities.into_iter().enumerate() {
            assert_eq!(positions.get(entity), Some(&Position(index as u32)));
        }
    }

    #[test]
    fn reads_zero_sized_components() {
        let mut world = World::new();
        let entity = world.spawn((Marker, Position(1)));

        let markers = world.accessor::<Marker>();

        assert_eq!(markers.get(entity), Some(&Marker));
    }

    struct Droppable {
        drops: Arc<AtomicUsize>,
    }

    impl Drop for Droppable {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn reads_non_copy_components_without_affecting_drop_semantics() {
        let drops = Arc::new(AtomicUsize::new(0));
        {
            let mut world = World::new();
            let entity = world.spawn((Droppable {
                drops: Arc::clone(&drops),
            },));

            {
                let components = world.accessor::<Droppable>();
                assert!(Arc::ptr_eq(&components.get(entity).unwrap().drops, &drops));
                assert_eq!(drops.load(Ordering::Relaxed), 0);
            }
        }

        assert_eq!(drops.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn mutably_updates_components_across_archetypes() {
        let mut world = World::new();
        let position_only = world.spawn((Position(1),));
        let both = world.spawn((Position(2), Velocity(3)));
        let velocity_only = world.spawn((Velocity(4),));

        {
            let mut positions = world.accessor_mut::<Position>();
            positions.get_mut(position_only).unwrap().0 += 10;
            positions.get_mut(both).unwrap().0 += 20;
            assert!(positions.get_mut(velocity_only).is_none());
        }

        assert_eq!(world.get::<Position>(position_only), Some(&Position(11)));
        assert_eq!(world.get::<Position>(both), Some(&Position(22)));
    }

    #[test]
    fn mutable_accessor_rejects_invalid_and_stale_entity_ids() {
        let mut world = World::new();
        let stale = world.spawn((Position(1),));
        assert!(world.despawn(stale));
        let current = world.spawn((Position(2),));

        let mut positions = world.accessor_mut::<Position>();

        assert!(positions.get_mut(stale).is_none());
        assert!(positions.get_mut(EntityId::new(u32::MAX, 0)).is_none());
        positions.get_mut(current).unwrap().0 = 5;
        assert_eq!(
            positions.get_mut(current).map(|position| position.0),
            Some(5)
        );
    }

    #[test]
    fn mutable_accessor_handles_multiple_chunk_layouts() {
        let mut world = World::new();
        let entities: Vec<_> = (0..160)
            .map(|index| world.spawn((Large([index as u8; 4 * 1024]), Position(index))))
            .collect();

        {
            let mut positions = world.accessor_mut::<Position>();
            for entity in entities.iter().copied() {
                positions.get_mut(entity).unwrap().0 += 1;
            }
        }

        for (index, entity) in entities.into_iter().enumerate() {
            assert_eq!(
                world.get::<Position>(entity),
                Some(&Position(index as u32 + 1))
            );
        }
    }

    #[test]
    fn mutable_accessor_updates_non_copy_components() {
        let mut world = World::new();
        let entity = world.spawn((String::from("Sky"),));

        {
            let mut strings = world.accessor_mut::<String>();
            strings.get_mut(entity).unwrap().push_str(" ECS");
        }

        assert_eq!(
            world.get::<String>(entity).map(String::as_str),
            Some("Sky ECS")
        );
    }
}
