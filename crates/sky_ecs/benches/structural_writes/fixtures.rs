use sky_ecs::{EntityId, World};
use std::hint::black_box;

pub(crate) const ENTITY_COUNT: usize = 1_000;
const BULK_ENTITY_COUNT: usize = 10_000;
const WARM_ENTITY_COUNT: usize = 4_096;

#[derive(Clone, Copy)]
pub(crate) struct Position(pub(crate) [f32; 3]);

#[derive(Clone, Copy)]
pub(crate) struct Velocity(pub(crate) [f32; 3]);

#[derive(Clone, Copy)]
struct Transform([f32; 16]);

#[derive(Clone, Copy)]
struct Rotation([f32; 3]);

#[derive(Clone, Copy)]
struct ReuseSeed;

pub(crate) fn light_bundle() -> (Position, Velocity) {
    (Position([1.0, 2.0, 3.0]), Velocity([0.25, 0.5, 0.75]))
}

pub(crate) fn fresh_world() -> World {
    World::new()
}

pub(crate) fn cold_storage_with_reused_ids() -> World {
    let mut world = World::new();
    let entities: Vec<_> = (0..WARM_ENTITY_COUNT)
        .map(|_| world.spawn((ReuseSeed,)))
        .collect();
    for entity in entities.into_iter().rev() {
        assert!(world.despawn(entity));
    }
    world
}

pub(crate) fn warm_started_light_storage() -> World {
    let mut world = World::new();
    let entities: Vec<_> = (0..WARM_ENTITY_COUNT)
        .map(|_| world.spawn(light_bundle()))
        .collect();
    for entity in entities.into_iter().rev() {
        assert!(world.despawn(entity));
    }
    world
}

pub(crate) fn spawn_light_rows(world: &mut World) {
    for _ in 0..ENTITY_COUNT {
        black_box(world.spawn(light_bundle()));
    }
}

pub(crate) fn populated_light_world() -> (World, Vec<EntityId>) {
    let mut world = World::new();
    let entities = (0..ENTITY_COUNT)
        .map(|_| world.spawn(light_bundle()))
        .collect();
    (world, entities)
}

pub(crate) fn shuffled_deletion_order() -> Vec<usize> {
    let mut order: Vec<_> = (0..ENTITY_COUNT).collect();
    let mut state = 0x9e37_79b9_u32;
    for index in (1..order.len()).rev() {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        order.swap(index, state as usize % (index + 1));
    }
    order
}

pub(crate) struct ChurnFixture {
    pub(crate) world: World,
    entities: Vec<EntityId>,
    deletion_order: Vec<usize>,
}

impl ChurnFixture {
    pub(crate) fn new() -> Self {
        Self {
            world: warm_started_light_storage(),
            entities: Vec::with_capacity(ENTITY_COUNT),
            deletion_order: shuffled_deletion_order(),
        }
    }

    pub(crate) fn cycle(&mut self) {
        self.entities.clear();
        for _ in 0..ENTITY_COUNT {
            self.entities.push(self.world.spawn(light_bundle()));
        }
        for &index in &self.deletion_order {
            assert!(self.world.despawn(self.entities[index]));
        }
    }
}

type BulkColumns = (Vec<Transform>, Vec<Position>, Vec<Rotation>, Vec<Velocity>);

pub(crate) struct BulkFixture {
    world: World,
    columns: BulkColumns,
}

fn bulk_columns() -> BulkColumns {
    (
        vec![Transform([1.0; 16]); BULK_ENTITY_COUNT],
        vec![Position([2.0; 3]); BULK_ENTITY_COUNT],
        vec![Rotation([3.0; 3]); BULK_ENTITY_COUNT],
        vec![Velocity([4.0; 3]); BULK_ENTITY_COUNT],
    )
}

pub(crate) fn fresh_bulk_fixture() -> BulkFixture {
    BulkFixture {
        world: World::new(),
        columns: bulk_columns(),
    }
}

pub(crate) fn reused_bulk_fixture() -> BulkFixture {
    let mut world = World::new();
    let entities: Vec<_> = (0..BULK_ENTITY_COUNT)
        .map(|_| world.spawn((ReuseSeed,)))
        .collect();
    for entity in entities.into_iter().rev() {
        assert!(world.despawn(entity));
    }
    BulkFixture {
        world,
        columns: bulk_columns(),
    }
}

pub(crate) fn insert_bulk(fixture: &mut BulkFixture) {
    fixture.world.spawn_columns(&mut fixture.columns).unwrap();
    black_box(&fixture.world);
}
