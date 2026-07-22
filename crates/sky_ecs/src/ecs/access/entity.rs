use super::routes::ComponentRoutes;
use crate::ecs::{EntityId, World};
use core::marker::PhantomData;
use core::ptr::NonNull;

/// A read-only component accessor bound to one [`World`].
///
/// Create an accessor with [`World::accessor`] when a hot path repeatedly
/// looks up component `T` on arbitrary [`EntityId`] values. Construction
/// resolves the component column once for every archetype. Individual calls to
/// [`get`](Self::get) still validate and route the supplied entity.
///
/// The accessor borrows the world for its entire lifetime, keeping cached
/// chunk views valid. For a fixed entity sequence used repeatedly, prefer
/// [`World::prepare_access`]. For occasional lookups, prefer [`World::get`].
#[must_use = "entity accessors do nothing until get is called"]
pub struct EntityAccessor<'w, T> {
    world: &'w World,
    routes: ComponentRoutes<T>,
}

impl<'w, T: 'static> EntityAccessor<'w, T> {
    pub(crate) fn new(world: &'w World) -> Self {
        Self {
            world,
            routes: ComponentRoutes::new(world),
        }
    }

    /// Returns component `T` on `entity`.
    ///
    /// Returns `None` when the entity is dead, its generation is stale, or its
    /// archetype does not contain `T`.
    #[inline(always)]
    pub fn get(&self, entity: EntityId) -> Option<&'w T> {
        let pointer = self.routes.resolve(self.world, entity).ok()?;
        Some(unsafe {
            // The pointer was resolved from this live, immutably borrowed
            // World and therefore remains initialized and stable for 'w.
            &*pointer.as_ptr()
        })
    }
}

/// An exclusive component accessor bound to one [`World`].
///
/// Create one with [`World::accessor_mut`] when a hot path repeatedly updates
/// component `T` on arbitrary [`EntityId`] values. The accessor exclusively
/// borrows the world, and every reference returned by
/// [`get_mut`](Self::get_mut) is tied to the corresponding mutable borrow of
/// the accessor.
#[must_use = "entity accessors do nothing until get_mut is called"]
pub struct EntityAccessorMut<'w, T> {
    world: NonNull<World>,
    routes: ComponentRoutes<T>,
    marker: PhantomData<&'w mut World>,
}

impl<'w, T: 'static> EntityAccessorMut<'w, T> {
    pub(crate) fn new(world: &'w mut World) -> Self {
        let routes = ComponentRoutes::new(world);
        let world = NonNull::from(world);

        Self {
            world,
            routes,
            marker: PhantomData,
        }
    }

    /// Returns component `T` on `entity` with exclusive access.
    ///
    /// Returns `None` when the entity is dead, its generation is stale, or its
    /// archetype does not contain `T`.
    #[inline(always)]
    pub fn get_mut(&mut self, entity: EntityId) -> Option<&mut T> {
        let world = unsafe {
            // `marker` retains the exclusive World borrow for 'w, so the World
            // remains alive and no safe structural operation can run.
            self.world.as_ref()
        };
        let pointer = self.routes.resolve(world, entity).ok()?;

        Some(unsafe {
            // The exclusive accessor borrow prevents another component
            // reference from this accessor from overlapping this one.
            &mut *pointer.as_ptr()
        })
    }
}
