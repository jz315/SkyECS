//! Purpose: Apply the same update through serial View and parallel ParView.
//! Prerequisites: step_05_systems.
//! APIs: View, ParView, for_each, par_for_each.
//! Run: cargo run -p sky_ecs --example step_06_parallel

use sky_ecs::{ParView, Res, Time, Update, View, World};

#[derive(Clone, Copy)]
struct Position(f64);

#[derive(Clone, Copy)]
struct Velocity(f64);

fn serial_integrate(bodies: View<(&mut Position, &Velocity)>, time: Res<Time>) {
    bodies.for_each(|position, velocity| position.0 += velocity.0 * f64::from(time.delta));
}

fn parallel_integrate(bodies: ParView<(&mut Position, &Velocity)>, time: Res<Time>) {
    bodies.par_for_each(|position, velocity| {
        position.0 += velocity.0 * f64::from(time.delta);
    });
}

fn checksum(world: &World) -> f64 {
    let mut sum = 0.0;
    world
        .query::<&Position>()
        .for_each(|position| sum += position.0);
    sum
}

fn build_world() -> World {
    let mut world = World::new();
    world.spawn_batch((0..100_000).map(|i| (Position(f64::from(i)), Velocity(2.0))));
    world
}

fn main() {
    let mut serial = build_world();
    let mut parallel = build_world();
    serial.stage(Update).add(serial_integrate);
    parallel.stage(Update).add(parallel_integrate);

    serial.tick_with_delta(0.5).unwrap();
    parallel.tick_with_delta(0.5).unwrap();

    let serial_sum = checksum(&serial);
    let parallel_sum = checksum(&parallel);
    assert_eq!(serial_sum, parallel_sum);
    assert_eq!(parallel_sum, 5_000_050_000.0);

    serial.shutdown();
    parallel.shutdown();
    // ParView automatically uses the serial path when a workload is too small.
    println!("step 06: serial and parallel checksums match: {parallel_sum:.0}");
}
