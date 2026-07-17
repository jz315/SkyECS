use super::entity_insertion::spawn_suite_batch;
use super::mixed_frame::warm_query;
use super::*;

pub(super) fn world_with_entities(count: usize) -> World {
    let mut world = World::default();
    spawn_suite_batch(&mut world, count);
    world
}
pub fn bench_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("simple_10k/freecs", |b| {
        let mut world = world_with_entities(SIMPLE_ENTITY_COUNT);
        warm_query(&mut world, MOVE_MASK);
        b.iter(|| {
            world.for_each_mut(MOVE_MASK, 0, |_entity, table, index| {
                table.position[index].0 += table.velocity[index].0;
            });
            black_box(&world);
        });
    });
}

pub fn bench_iteration_large(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("simple_100k/freecs", |b| {
        let mut world = world_with_entities(LARGE_ITERATION_ENTITY_COUNT);
        warm_query(&mut world, MOVE_MASK);
        b.iter(|| {
            world.for_each_mut(MOVE_MASK, 0, |_entity, table, index| {
                table.position[index].0 += table.velocity[index].0;
            });
            black_box(&world);
        });
    });
}

pub fn bench_iteration_1m(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("simple_1m/freecs", |b| {
        let mut world = world_with_entities(VERY_LARGE_ITERATION_ENTITY_COUNT);
        warm_query(&mut world, MOVE_MASK);
        b.iter(|| {
            world.for_each_mut(MOVE_MASK, 0, |_entity, table, index| {
                table.position[index].0 += table.velocity[index].0;
            });
            black_box(&world);
        });
    });
}
