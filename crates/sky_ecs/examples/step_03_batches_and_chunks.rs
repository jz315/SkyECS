//! Purpose: Import homogeneous data in bulk and process aligned chunk slices.
//! Prerequisites: step_02_queries.
//! APIs: spawn_batch, for_each, for_each_chunk.
//! Run: cargo run -p sky_ecs --example step_03_batches_and_chunks

use sky_ecs::World;

#[derive(Clone, Copy)]
struct Position(i64);

#[derive(Clone, Copy)]
struct Velocity(i64);

fn main() {
    let mut world = World::new();
    const ENTITY_COUNT: i64 = 10_000;

    // spawn_batch is ideal for imports where every entity has the same shape
    // and the caller does not need to retain every returned EntityId.
    world.spawn_batch((0..ENTITY_COUNT).map(|i| (Position(i), Velocity(2))));

    let mut before = 0_i64;
    world
        .query::<&Position>()
        .for_each(|position| before += position.0);
    assert_eq!(before, (ENTITY_COUNT - 1) * ENTITY_COUNT / 2);

    let mut chunk_count = 0;
    world
        .query_mut::<(&mut Position, &Velocity)>()
        .for_each_chunk(|positions, velocities| {
            // Every slice represents the same entity range in this chunk.
            assert_eq!(positions.len(), velocities.len());
            chunk_count += 1;
            for (position, velocity) in positions.iter_mut().zip(velocities) {
                position.0 += velocity.0;
            }
        });

    let mut after = 0_i64;
    world
        .query::<&Position>()
        .for_each(|position| after += position.0);
    assert!(chunk_count > 0);
    assert_eq!(after, before + ENTITY_COUNT * 2);

    println!("step 03: processed {ENTITY_COUNT} entities across {chunk_count} chunks");
}
