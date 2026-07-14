//! Purpose: Use runtime-typed ECS access in tools, editors, scripts, or reflection.
//! Prerequisites: step_07_tiny_defense and a need for runtime component types.
//! APIs: DynamicBundle, WorldDynamicExt, DynamicQueryBuilder, read/write slots.
//! Run: cargo run -p sky_ecs --example step_08_dynamic

use sky_ecs::dynamic::{DynamicBundle, DynamicQueryBuilder, WorldDynamicExt};
use sky_ecs::World;

#[derive(Debug, PartialEq)]
struct Position(i32);

#[derive(Debug)]
struct Velocity(i32);

fn main() {
    let mut world = World::new();
    let entity = world
        .spawn_dynamic(DynamicBundle::new().with(Position(10)).with(Velocity(3)))
        .expect("dynamic bundle should contain unique component types");

    // Slots are numbered in builder order: Position is slot 0, Velocity slot 1.
    let mut update = DynamicQueryBuilder::new()
        .write::<Position>()
        .read::<Velocity>()
        .build()
        .unwrap();
    update
        .for_each_chunk_mut(&mut world, |mut chunk| {
            let (positions, velocities) = chunk.write_read::<Position, Velocity>(0, 1)?;
            for (position, velocity) in positions.iter_mut().zip(velocities) {
                position.0 += velocity.0;
            }
            Ok(())
        })
        .unwrap();
    assert_eq!(world.get::<Position>(entity), Some(&Position(13)));

    let mut inspect = DynamicQueryBuilder::new()
        .read::<Position>()
        .build()
        .unwrap();
    let mut checksum = 0;
    inspect
        .for_each_chunk(&world, |chunk| {
            checksum += chunk
                .read::<Position>(0)?
                .iter()
                .map(|value| value.0)
                .sum::<i32>();
            Ok(())
        })
        .unwrap();
    assert_eq!(checksum, 13);

    // Dynamic APIs validate mistakes at runtime instead of at compile time.
    let duplicate = DynamicQueryBuilder::new()
        .read::<Position>()
        .write::<Position>()
        .build();
    let error = match duplicate {
        Ok(_) => panic!("duplicate slots must be rejected"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("duplicate component"));

    println!("step 08: dynamic slots updated Position and rejected an invalid query");
}
