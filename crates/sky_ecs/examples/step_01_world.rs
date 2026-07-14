//! Purpose: Learn the World, components, bundles, and generational entity IDs.
//! Prerequisites: None.
//! APIs: World, EntityId, spawn, contains, get, get_mut, insert, remove, despawn.
//! Run: cargo run -p sky_ecs --example step_01_world

use sky_ecs::{EntityId, World};

#[derive(Debug, PartialEq)]
struct Name(&'static str);

#[derive(Debug, PartialEq)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(Debug, PartialEq)]
struct Health(u32);

fn main() {
    let mut world = World::new();

    // A tuple is a Bundle: all its components are attached in one spawn.
    let hero: EntityId = world.spawn((Name("hero"), Position { x: 2, y: 3 }, Health(100)));
    assert!(world.contains(hero));
    assert_eq!(world.get::<Name>(hero), Some(&Name("hero")));

    world.get_mut::<Position>(hero).unwrap().x += 5;
    assert_eq!(world.get::<Position>(hero).unwrap().x, 7);

    assert!(world.insert(hero, 25_u32));
    assert_eq!(world.get::<u32>(hero), Some(&25));
    assert!(world.remove::<u32>(hero));
    assert!(world.get::<u32>(hero).is_none());

    assert!(world.despawn(hero));
    assert!(!world.contains(hero));
    assert!(world.get::<Health>(hero).is_none());

    // EntityId contains an index and a generation. Reusing storage cannot make
    // an old ID valid again, because the replacement has a newer generation.
    let replacement = world.spawn((Name("replacement"), Position { x: 0, y: 0 }));
    assert_eq!(hero.index(), replacement.index());
    assert_ne!(hero.generation(), replacement.generation());
    assert_ne!(hero, replacement);
    assert!(!world.contains(hero));
    assert!(world.contains(replacement));

    println!("step 01: the replacement is alive and the stale ID is invalid");
}
