use super::{
    EntityAccessor, EntityAccessorMut, PrepareAccessError, PreparedEntityAccess,
    PreparedEntityAccessMut,
};
use crate::ecs::{EntityId, World};

impl World {
    /// Creates a read-only accessor for repeated random access to component `T`.
    ///
    /// Construction resolves `T` once for every archetype and caches typed
    /// views of matching chunk columns. This is useful when a hot loop performs
    /// many lookups by [`EntityId`]. For an occasional lookup, use
    /// [`World::get`] directly.
    ///
    /// The accessor holds a shared borrow of this world, so structural changes
    /// cannot occur while it remains in use.
    ///
    /// ```
    /// use sky_ecs::World;
    ///
    /// #[derive(Debug, PartialEq)]
    /// struct Position(f32, f32);
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn((Position(1.0, 2.0),));
    /// let positions = world.accessor::<Position>();
    ///
    /// assert_eq!(positions.get(entity), Some(&Position(1.0, 2.0)));
    /// ```
    ///
    /// Structural mutation is rejected while the accessor is still used:
    ///
    /// ```compile_fail
    /// use sky_ecs::World;
    ///
    /// struct Position(f32, f32);
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn((Position(1.0, 2.0),));
    /// let positions = world.accessor::<Position>();
    /// world.spawn((Position(3.0, 4.0),));
    /// let _ = positions.get(entity);
    /// ```
    #[inline]
    pub fn accessor<T: 'static>(&self) -> EntityAccessor<'_, T> {
        EntityAccessor::new(self)
    }

    /// Prepares direct read-only access to component `T` for a fixed entity sequence.
    ///
    /// Preparation validates every entity and component once while preserving
    /// the exact input order. A successful plan can then be indexed or iterated
    /// without repeating entity, generation, chunk-route, or component-presence
    /// checks. Use [`World::accessor`] instead when the entity sequence is not
    /// known in advance.
    ///
    /// Duplicate entities are allowed because the plan only produces shared
    /// references. An invalid, stale, or component-missing entity makes the
    /// entire preparation fail with its input index.
    ///
    /// ```
    /// use sky_ecs::World;
    ///
    /// #[derive(Debug, PartialEq)]
    /// struct Position(u32);
    ///
    /// let mut world = World::new();
    /// let entities = [
    ///     world.spawn((Position(1),)),
    ///     world.spawn((Position(2),)),
    /// ];
    /// let positions = world.prepare_access::<Position>(&entities).unwrap();
    ///
    /// assert_eq!(positions.iter().map(|position| position.0).collect::<Vec<_>>(), [1, 2]);
    /// ```
    ///
    /// Structural mutation is rejected while the prepared access remains in use:
    ///
    /// ```compile_fail
    /// use sky_ecs::World;
    ///
    /// struct Position(u32);
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn((Position(1),));
    /// let positions = world.prepare_access::<Position>(&[entity]).unwrap();
    /// world.spawn((Position(2),));
    /// let _ = positions.get(0);
    /// ```
    #[inline]
    pub fn prepare_access<T: 'static>(
        &self,
        entities: &[EntityId],
    ) -> Result<PreparedEntityAccess<'_, T>, PrepareAccessError> {
        PreparedEntityAccess::new(self, entities)
    }

    /// Creates an exclusive accessor for repeated random updates to component `T`.
    ///
    /// Like [`World::accessor`], construction resolves matching component
    /// columns before the hot loop. The accessor exclusively borrows the world,
    /// and each component reference remains tied to one mutable accessor borrow.
    ///
    /// ```
    /// use sky_ecs::World;
    ///
    /// struct Position(f32, f32);
    ///
    /// let mut world = World::new();
    /// let entity = world.spawn((Position(1.0, 2.0),));
    /// {
    ///     let mut positions = world.accessor_mut::<Position>();
    ///     positions.get_mut(entity).unwrap().0 += 3.0;
    /// }
    /// assert_eq!(world.get::<Position>(entity).unwrap().0, 4.0);
    /// ```
    ///
    /// Mutable references from the same accessor cannot overlap:
    ///
    /// ```compile_fail
    /// use sky_ecs::World;
    ///
    /// struct Position(f32, f32);
    ///
    /// let mut world = World::new();
    /// let first = world.spawn((Position(1.0, 2.0),));
    /// let second = world.spawn((Position(3.0, 4.0),));
    /// let mut positions = world.accessor_mut::<Position>();
    /// let first_position = positions.get_mut(first).unwrap();
    /// let second_position = positions.get_mut(second).unwrap();
    /// first_position.0 += second_position.0;
    /// ```
    #[inline]
    pub fn accessor_mut<T: 'static>(&mut self) -> EntityAccessorMut<'_, T> {
        EntityAccessorMut::new(self)
    }

    /// Prepares direct exclusive access to component `T` for a fixed entity sequence.
    ///
    /// This is the mutable counterpart to [`World::prepare_access`]. In
    /// addition to entity and component validation, preparation rejects a
    /// duplicate entity so that iteration can safely yield multiple live
    /// mutable references.
    ///
    /// ```
    /// use sky_ecs::World;
    ///
    /// struct Position(u32);
    ///
    /// let mut world = World::new();
    /// let entities = [
    ///     world.spawn((Position(1),)),
    ///     world.spawn((Position(2),)),
    /// ];
    /// {
    ///     let mut positions = world.prepare_access_mut::<Position>(&entities).unwrap();
    ///     for position in positions.iter_mut() {
    ///         position.0 += 10;
    ///     }
    /// }
    /// assert_eq!(world.get::<Position>(entities[0]).unwrap().0, 11);
    /// assert_eq!(world.get::<Position>(entities[1]).unwrap().0, 12);
    /// ```
    ///
    /// Mutable references from one plan cannot overlap through safe code:
    ///
    /// ```compile_fail
    /// use sky_ecs::World;
    ///
    /// struct Position(u32);
    ///
    /// let mut world = World::new();
    /// let entities = [
    ///     world.spawn((Position(1),)),
    ///     world.spawn((Position(2),)),
    /// ];
    /// let mut positions = world.prepare_access_mut::<Position>(&entities).unwrap();
    /// let first = positions.get_mut(0).unwrap();
    /// let second = positions.get_mut(1).unwrap();
    /// first.0 += second.0;
    /// ```
    #[inline]
    pub fn prepare_access_mut<T: 'static>(
        &mut self,
        entities: &[EntityId],
    ) -> Result<PreparedEntityAccessMut<'_, T>, PrepareAccessError> {
        PreparedEntityAccessMut::new(self, entities)
    }
}
