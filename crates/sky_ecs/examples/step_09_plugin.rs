//! Purpose: Package configuration and systems into a reusable ECS plugin.
//! Prerequisites: step_05_systems; step_08_dynamic is optional.
//! APIs: Plugin, PluginResult, World::install, World::has_plugin.
//! Run: cargo run -p sky_ecs --example step_09_plugin

use sky_ecs::{Plugin, PluginResult, Res, Time, Update, View, World};

#[derive(Debug)]
struct Position(f32);

struct Velocity(f32);

struct MovementConfig {
    speed_scale: f32,
}

struct MovementPlugin {
    speed_scale: f32,
}

fn movement(
    bodies: View<(&mut Position, &Velocity)>,
    config: Res<MovementConfig>,
    time: Res<Time>,
) {
    bodies.for_each(|position, velocity| {
        position.0 += velocity.0 * config.speed_scale * time.delta;
    });
}

impl Plugin for MovementPlugin {
    fn name(&self) -> &'static str {
        "MovementPlugin"
    }

    fn install(self, world: &mut World) -> PluginResult {
        world.insert_resource(MovementConfig {
            speed_scale: self.speed_scale,
        });
        world.stage(Update).add(movement);
        Ok(())
    }
}

fn main() {
    let mut world = World::new();
    let entity = world.spawn((Position(0.0), Velocity(3.0)));

    world.install(MovementPlugin { speed_scale: 2.0 }).unwrap();
    assert!(world.has_plugin::<MovementPlugin>());

    let duplicate = world.install(MovementPlugin { speed_scale: 99.0 });
    assert!(duplicate.is_err());
    assert_eq!(
        world.get_resource::<MovementConfig>().unwrap().speed_scale,
        2.0
    );

    world.tick_with_delta(0.5).unwrap();
    assert_eq!(world.get::<Position>(entity).unwrap().0, 3.0);
    world.shutdown();

    println!("step 09: plugin configuration moved the entity to x=3");
}
