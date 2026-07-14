//! Purpose: Turn ordinary functions into staged systems with typed parameters.
//! Prerequisites: step_04_commands.
//! APIs: View, Res, ResMut, Time, Commands, Update, PostUpdate, FixedUpdate.
//! Run: cargo run -p sky_ecs --example step_05_systems

use sky_ecs::{
    Commands, FixedStep, FixedUpdate, PostUpdate, Res, ResMut, Time, Update, View, World,
};

#[derive(Debug)]
struct Position(f32);

#[derive(Debug)]
struct Velocity(f32);

#[derive(Debug)]
struct Lifetime(u32);

#[derive(Default)]
struct FrameCount(u32);

#[derive(Default)]
struct FixedCount(u32);

#[derive(Default)]
struct ExecutionLog(Vec<&'static str>);

fn movement(
    bodies: View<(&mut Position, &Velocity)>,
    time: Res<Time>,
    mut log: ResMut<ExecutionLog>,
) {
    bodies.for_each(|(position, velocity)| position.0 += velocity.0 * time.delta);
    log.0.push("movement");
}

fn count_frame(mut frames: ResMut<FrameCount>, mut log: ResMut<ExecutionLog>) {
    frames.0 += 1;
    log.0.push("count_frame");
}

fn expire_entities(
    lifetimes: View<&mut Lifetime>,
    mut commands: Commands<'_>,
    mut log: ResMut<ExecutionLog>,
) {
    lifetimes.for_each_with_entity(|entity, lifetime| {
        lifetime.0 -= 1;
        if lifetime.0 == 0 {
            commands.despawn(entity);
        }
    });
    log.0.push("expire_entities");
}

fn fixed_update(mut fixed: ResMut<FixedCount>, mut log: ResMut<ExecutionLog>) {
    fixed.0 += 1;
    log.0.push("fixed_update");
}

fn main() {
    let mut world = World::new();
    world.insert_resource(FrameCount::default());
    world.insert_resource(FixedCount::default());
    world.insert_resource(ExecutionLog::default());
    let short_lived = world.spawn((Position(0.0), Velocity(4.0), Lifetime(1)));

    world.stage(Update).add(movement).add(count_frame);
    world.stage(PostUpdate).add(expire_entities);
    world
        .stage(FixedUpdate)
        .fixed(FixedStep::hz(50))
        .expect("FixedUpdate should be configured only once")
        .add(fixed_update);

    // Update runs before PostUpdate. Commands queued in PostUpdate are flushed
    // at that stage boundary, so the entity is gone when tick returns.
    world.tick_with_delta(0.021).unwrap();
    assert!(!world.contains(short_lived));
    assert_eq!(world.get_resource::<FrameCount>().unwrap().0, 1);
    assert_eq!(world.get_resource::<FixedCount>().unwrap().0, 1);
    assert_eq!(
        world.get_resource::<ExecutionLog>().unwrap().0,
        ["fixed_update", "movement", "count_frame", "expire_entities"]
    );

    world.shutdown();
    println!("step 05: FixedUpdate, Update, and PostUpdate ran in stable stage order");
}
