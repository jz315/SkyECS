//! Purpose: Defer structural changes until active query borrows have ended.
//! Prerequisites: step_03_batches_and_chunks.
//! APIs: CommandBuffer spawn, despawn, insert, remove, apply, clear.
//! Run: cargo run -p sky_ecs --example step_04_commands

use sky_ecs::{CommandBuffer, World};

#[derive(Debug, PartialEq)]
struct Name(&'static str);

#[derive(Debug, PartialEq)]
struct Health(u32);

struct Poisoned;
struct Shielded;

fn main() {
    let mut world = World::new();

    // Direct structural changes are simplest when no query is borrowing World.
    let warrior = world.spawn((Name("warrior"), Health(100)));
    assert!(world.insert(warrior, Poisoned));
    assert!(world.remove::<Poisoned>(warrior));

    let rogue = world.spawn((Name("rogue"), Health(40)));
    let mage = world.spawn((Name("mage"), Health(60), Poisoned));

    // During iteration, collect structural work instead of mutating World.
    let mut commands = CommandBuffer::new();
    world
        .query::<(&Name, &Health)>()
        .for_each_with_entity(|entity, name, health| match name.0 {
            "warrior" => commands.insert(entity, Shielded),
            "rogue" if health.0 < 50 => commands.despawn(entity),
            "mage" => commands.remove::<Poisoned>(entity),
            _ => {}
        });
    commands.spawn((Name("healer"), Health(80)));

    assert_eq!(world.query::<&Name>().count(), 3);
    assert!(world.get::<Shielded>(warrior).is_none());
    commands.apply(&mut world);

    assert_eq!(world.query::<&Name>().count(), 3);
    assert!(world.get::<Shielded>(warrior).is_some());
    assert!(!world.contains(rogue));
    assert!(world.get::<Poisoned>(mage).is_none());

    // clear discards queued work without applying it.
    commands.despawn(warrior);
    commands.clear();
    commands.apply(&mut world);
    assert!(world.contains(warrior));

    println!("step 04: applied one deferred batch and discarded another");
}
