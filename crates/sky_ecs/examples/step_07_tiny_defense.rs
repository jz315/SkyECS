//! Purpose: Build a deterministic, scheduler-driven ASCII tower defense loop.
//! Prerequisites: step_05_systems; step_06_parallel is optional.
//! APIs: resources, staged systems, filtered View, Commands, stage-boundary flushes.
//! Run: cargo run -p sky_ecs --example step_07_tiny_defense

use sky_ecs::{Commands, PostUpdate, PreUpdate, ResMut, Update, View, With, World};

#[derive(Clone, Copy)]
struct Position(i32);

#[derive(Clone, Copy)]
struct Velocity(i32);

struct Health(i32);

struct Turret {
    shots_fired: u32,
}

struct Enemy;

struct Bullet {
    damage: i32,
    spent: bool,
}

struct GameState {
    frame: u32,
    score: u32,
    target_score: u32,
    base_health: u32,
    won: bool,
}

fn spawn_and_aim(
    mut state: ResMut<GameState>,
    enemies: View<&Position, With<Enemy>>,
    turrets: View<(&Position, &mut Turret)>,
    mut commands: Commands<'_>,
) {
    // The two waves are deterministic. These enemies become visible after
    // PreUpdate ends, so Update can move them during the same tick.
    if state.frame < 2 {
        commands.spawn((Position(8), Velocity(-1), Health(1), Enemy));
    }

    // A newly queued enemy is not visible inside this stage. The turret fires
    // only at enemies that existed when PreUpdate began.
    if !enemies.is_empty() {
        turrets.for_each(|position, turret| {
            commands.spawn((
                *position,
                Velocity(3),
                Bullet {
                    damage: 1,
                    spent: false,
                },
            ));
            turret.shots_fired += 1;
        });
    }
    state.frame += 1;
}

fn movement(movers: View<(&mut Position, &Velocity)>) {
    movers.for_each(|position, velocity| position.0 += velocity.0);
}

fn collisions(
    bullets: View<(&Position, &mut Bullet)>,
    enemies: View<(&Position, &mut Health), With<Enemy>>,
) {
    bullets.for_each(|bullet_position, bullet| {
        enemies.for_each(|enemy_position, health| {
            if !bullet.spent && bullet_position.0 >= enemy_position.0 {
                health.0 -= bullet.damage;
                bullet.spent = true;
            }
        });
    });
}

fn cleanup_and_score(
    enemies: View<(&Position, &Health), With<Enemy>>,
    bullets: View<&Bullet>,
    mut state: ResMut<GameState>,
    mut commands: Commands<'_>,
) {
    enemies.for_each_with_entity(|entity, position, health| {
        if health.0 <= 0 {
            state.score += 1;
            commands.despawn(entity);
        } else if position.0 <= 0 {
            state.base_health -= 1;
            commands.despawn(entity);
        }
    });

    state.won = state.score == state.target_score;
    bullets.for_each_with_entity(|entity, bullet| {
        if bullet.spent || state.won {
            commands.despawn(entity);
        }
    });
}

fn ascii_line(world: &World) -> String {
    let mut cells = ['.'; 9];
    world
        .query::<(&Position, &Turret)>()
        .for_each(|position, _| cells[position.0 as usize] = 'T');
    world
        .query::<&Position>()
        .filter::<With<Enemy>>()
        .for_each(|position| cells[position.0 as usize] = 'E');
    world
        .query::<(&Position, &Bullet)>()
        .for_each(|position, _| cells[position.0 as usize] = 'B');
    cells.into_iter().collect()
}

fn main() {
    let mut world = World::new();
    world.insert_resource(GameState {
        frame: 0,
        score: 0,
        target_score: 2,
        base_health: 3,
        won: false,
    });
    let turret = world.spawn((Position(0), Turret { shots_fired: 0 }));

    world.stage(PreUpdate).add(spawn_and_aim);
    world.stage(Update).add(movement).add(collisions);
    world.stage(PostUpdate).add(cleanup_and_score);

    let expected = ["T......E.", "T..B..EE.", "T..B..E..", "T........"];
    for (frame, expected_line) in expected.iter().enumerate() {
        world.tick_with_delta(1.0).unwrap();
        let line = ascii_line(&world);
        assert_eq!(&line, expected_line);
        println!("frame {frame}: {line}");
    }

    let state = world.get_resource::<GameState>().unwrap();
    assert!(state.won);
    assert_eq!(state.score, 2);
    assert_eq!(state.base_health, 3);
    assert_eq!(world.query::<&Enemy>().count(), 0);
    assert_eq!(world.query::<&Bullet>().count(), 0);
    assert_eq!(world.get::<Turret>(turret).unwrap().shots_fired, 3);

    world.shutdown();
    println!("victory: score=2, base_health=3");
}
