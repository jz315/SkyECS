//! Purpose: Move from read-only queries to mutable and filtered typed queries.
//! Prerequisites: step_01_world.
//! APIs: Query, QueryMut, Option<&T>, QueryData, With, Without, Any.
//! Run: cargo run -p sky_ecs --example step_02_queries

use sky_ecs::{Any, QueryData, With, Without, World};

#[derive(Debug, PartialEq)]
struct Position(i32);

#[derive(Debug, PartialEq)]
struct Velocity(i32);

struct Player;
struct Enemy;
struct Disabled;
struct Selected;

#[derive(QueryData)]
struct Movement<'a> {
    position: &'a mut Position,
    velocity: &'a Velocity,
}

fn main() {
    let mut world = World::new();
    let player = world.spawn((Position(0), Velocity(2), Player, Selected));
    let moving_enemy = world.spawn((Position(10), Velocity(-1), Enemy));
    world.spawn((Position(20), Enemy));
    world.spawn((Position(30), Velocity(-5), Enemy, Disabled, Selected));

    let positions = world.query::<&Position>();
    assert_eq!(positions.count(), 4);
    assert!(!positions.is_empty());

    let optional_velocity = world.query::<(&Position, Option<&Velocity>)>();
    let mut moving = 0;
    let mut stationary = 0;
    optional_velocity.for_each(|_, velocity| {
        if velocity.is_some() {
            moving += 1;
        } else {
            stationary += 1;
        }
    });
    assert_eq!((moving, stationary), (3, 1));

    world
        .query_mut::<Movement<'_>>()
        .for_each(|position, velocity| position.0 += velocity.0);
    assert_eq!(world.get::<Position>(player), Some(&Position(2)));
    assert_eq!(world.get::<Position>(moving_enemy), Some(&Position(9)));

    let active_enemies = world
        .query::<&Position>()
        .filter::<(With<Enemy>, Without<Disabled>)>();
    assert_eq!(active_enemies.count(), 2);

    let interesting = world
        .query::<&Position>()
        .filter::<Any<(With<Player>, With<Selected>)>>();
    let mut entity_ids = Vec::new();
    interesting.for_each_with_entity(|entity, _| entity_ids.push(entity));
    assert_eq!(entity_ids.len(), 2);
    assert!(entity_ids.contains(&player));

    println!("step 02: updated 3 moving entities and found 2 active enemies");
}
