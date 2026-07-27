use crate::ecs::entity::{EntityRecord, EntityRoute};
use crate::ecs::{EntityId, World};

/// A structurally frozen view of one World's entity-record table.
///
/// The view is reacquired for every accessor construction, prepared-view bind,
/// or scheduler invocation. Component-column caches may survive pure row
/// churn, but this slice must not: appending entity slots can reallocate the
/// record table without changing any component-column base.
#[derive(Clone, Copy)]
pub(crate) struct EntityRouteView<'w> {
    records: &'w [EntityRecord],
}

impl<'w> EntityRouteView<'w> {
    #[inline(always)]
    pub(crate) fn new(world: &'w World) -> Self {
        Self {
            records: world.entity_records(),
        }
    }

    #[inline(always)]
    pub(crate) fn resolve(self, entity: EntityId) -> Option<EntityRoute> {
        EntityRecord::resolve(self.records, entity)
    }

    #[cfg(test)]
    pub(super) fn as_ptr(self) -> *const EntityRecord {
        self.records.as_ptr()
    }

    #[cfg(test)]
    pub(super) fn len(self) -> usize {
        self.records.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Marker;

    #[test]
    fn resolves_only_live_in_range_generations() {
        let mut world = World::new();
        let live = world.spawn((Marker,));

        {
            let routes = EntityRouteView::new(&world);
            assert_eq!(routes.len(), 1);
            assert!(!routes.as_ptr().is_null());
            assert!(routes.resolve(live).is_some());
            assert!(routes
                .resolve(EntityId::new(
                    live.index(),
                    live.generation().wrapping_add(1),
                ))
                .is_none());
            assert!(routes.resolve(EntityId::new(u32::MAX, 0)).is_none());
        }

        assert!(world.despawn(live));
        assert!(EntityRouteView::new(&world).resolve(live).is_none());
    }
}
