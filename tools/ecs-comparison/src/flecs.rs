use crate::common::*;
use crate::shared::sample_entities;
use cgmath::{SquareMatrix, Transform as _};
use criterion::{measurement::WallTime, BatchSize, BenchmarkGroup};
use flecs_ecs::core::{
    Builder, ComponentId, Entity as FlecsEntity, EntityViewGet, IdOperations, QueryAPI,
    QueryBuilderImpl, QueryCacheKind, World,
};
use flecs_ecs::sys;
use std::hint::black_box;
use std::mem::size_of;

#[derive(Clone, Copy)]
struct LightTarget {
    table: *mut sys::ecs_table_t,
}

impl LightTarget {
    fn new(world: &World) -> Self {
        let position_id = u64::from(world.component_id::<PositionComponent>());
        let velocity_id = u64::from(world.component_id::<VelocityComponent>());
        let table = find_table(world, [position_id, velocity_id]);
        Self { table }
    }

    fn spawn(&self, world: &World) -> FlecsEntity {
        // SAFETY: The prepared table belongs to this live world. Flecs runs
        // registered component constructors while creating directly in it.
        unsafe {
            let entity = sys::ecs_new_w_table(world.ptr_mut(), self.table);
            assert_ne!(
                entity, 0,
                "Flecs should create an entity in the target table"
            );
            FlecsEntity::from(entity)
        }
    }
}

#[derive(Clone, Copy)]
struct SuiteTarget {
    table: *mut sys::ecs_table_t,
    table_ids: [u64; 4],
    transform_id: u64,
    position_id: u64,
    rotation_id: u64,
    velocity_id: u64,
}

impl SuiteTarget {
    fn new(world: &World) -> Self {
        let transform_id = u64::from(world.component_id::<TransformComponent>());
        let position_id = u64::from(world.component_id::<PositionComponent>());
        let rotation_id = u64::from(world.component_id::<RotationComponent>());
        let velocity_id = u64::from(world.component_id::<VelocityComponent>());
        let mut table_ids = [transform_id, position_id, rotation_id, velocity_id];
        table_ids.sort_unstable();
        let table = find_sorted_table(world, &table_ids);
        Self {
            table,
            table_ids,
            transform_id,
            position_id,
            rotation_id,
            velocity_id,
        }
    }

    fn spawn(&self, world: &World, bundle: SuiteBundle) -> FlecsEntity {
        let (transform, position, rotation, velocity) = bundle;
        // SAFETY: The prepared table and IDs belong to this live world, and
        // every value has the exact registered Rust component layout.
        unsafe {
            let entity = sys::ecs_new_w_table(world.ptr_mut(), self.table);
            assert_ne!(
                entity, 0,
                "Flecs should create an entity in the target table"
            );
            set_existing(world, entity, self.transform_id, &transform);
            set_existing(world, entity, self.position_id, &position);
            set_existing(world, entity, self.rotation_id, &rotation);
            set_existing(world, entity, self.velocity_id, &velocity);
            FlecsEntity::from(entity)
        }
    }

    fn bulk_spawn(
        &self,
        world: &World,
        transforms: &[TransformComponent],
        positions: &[PositionComponent],
        rotations: &[RotationComponent],
        velocities: &[VelocityComponent],
    ) -> FlecsEntity {
        let count = transforms.len();
        assert_ne!(count, 0);
        assert_eq!(positions.len(), count);
        assert_eq!(rotations.len(), count);
        assert_eq!(velocities.len(), count);
        assert!(count <= i32::MAX as usize);

        let mut descriptor: sys::ecs_bulk_desc_t = unsafe { std::mem::zeroed() };
        descriptor.count = count as i32;
        descriptor.ids[..self.table_ids.len()].copy_from_slice(&self.table_ids);
        let mut data = self.table_ids.map(|id| {
            if id == self.transform_id {
                transforms.as_ptr().cast_mut().cast()
            } else if id == self.position_id {
                positions.as_ptr().cast_mut().cast()
            } else if id == self.rotation_id {
                rotations.as_ptr().cast_mut().cast()
            } else {
                debug_assert_eq!(id, self.velocity_id);
                velocities.as_ptr().cast_mut().cast()
            }
        });
        descriptor.data = data.as_mut_ptr();

        // SAFETY: IDs and data pointers have the same sorted order. The target
        // table already exists in this World, and Flecs copies the slices.
        let entities = unsafe { sys::ecs_bulk_init(world.ptr_mut(), &descriptor) };
        assert!(!entities.is_null(), "Flecs bulk creation should succeed");
        // SAFETY: Flecs returns `count` entity IDs for a successful bulk call.
        FlecsEntity::from(unsafe { *entities.add(count - 1) })
    }
}

fn prepared_insert_context() -> (World, SuiteTarget) {
    let world = World::new();
    let target = SuiteTarget::new(&world);
    (world, target)
}

fn find_table<const N: usize>(world: &World, mut ids: [u64; N]) -> *mut sys::ecs_table_t {
    ids.sort_unstable();
    find_sorted_table(world, &ids)
}

fn find_sorted_table(world: &World, ids: &[u64]) -> *mut sys::ecs_table_t {
    // SAFETY: Every ID was registered in this world, and the sorted array is
    // valid for the duration of the call. Flecs owns the returned table.
    let table = unsafe { sys::ecs_table_find(world.ptr_mut(), ids.as_ptr(), ids.len() as i32) };
    assert!(!table.is_null(), "Flecs should resolve the target table");
    table
}

unsafe fn set_existing<T>(world: &World, entity: u64, component: u64, value: &T) {
    // SAFETY: Callers guarantee that `component` is present on `entity` and was
    // registered with layout `T`; ecs_set_id copies the value during the call.
    unsafe {
        sys::ecs_set_id(
            world.ptr_mut(),
            entity,
            component,
            size_of::<T>(),
            (value as *const T).cast(),
        );
    }
}

fn delete_entity(world: &World, entity: FlecsEntity) {
    // SAFETY: The ID was created in this world and is deleted exactly once.
    unsafe { sys::ecs_delete(world.ptr_mut(), u64::from(entity)) };
}

// ---------------------------------------------------------------------------
// World builders
// ---------------------------------------------------------------------------

fn world_with_entities(n: usize) -> World {
    let world = World::new();
    for _ in 0..n {
        let (t, p, r, v) = suite_bundle();
        world.entity().set(t).set(p).set(r).set(v);
    }
    world
}

fn fragmented_world() -> World {
    let world = World::new();

    macro_rules! add_variant {
        ($tag:ty) => {
            for _ in 0..FRAGMENTED_ENTITIES_PER_VARIANT {
                world
                    .entity()
                    .set(<$tag>::default())
                    .set(DataComponent(1.0));
            }
        };
    }

    add_variant!(A);
    add_variant!(B);
    add_variant!(C);
    add_variant!(D);
    add_variant!(E);
    add_variant!(F);
    add_variant!(G);
    add_variant!(H);
    add_variant!(I);
    add_variant!(J);
    add_variant!(K);
    add_variant!(L);
    add_variant!(M);
    add_variant!(N);
    add_variant!(O);
    add_variant!(P);
    add_variant!(Q);
    add_variant!(R);
    add_variant!(S);
    add_variant!(T);
    add_variant!(U);
    add_variant!(V);
    add_variant!(W);
    add_variant!(X);
    add_variant!(Y);
    add_variant!(Z);

    world
}

fn random_fragmented_world(component_count: usize, entity_count: usize) -> (World, usize) {
    let masks = random_fragment_masks_for(component_count, entity_count);
    let expected = random_fragment_match_count(&masks);
    let world = World::new();

    for mask in masks {
        let entity = world.entity();
        macro_rules! component {
            ($bit:expr, $component:ident) => {
                if mask & (1 << $bit) != 0 {
                    entity.set($component(10.0));
                }
            };
        }
        component!(0, A);
        component!(1, B);
        component!(2, C);
        component!(3, D);
        component!(4, E);
        component!(5, F);
        component!(6, G);
        component!(7, H);
        component!(8, I);
        component!(9, J);
        component!(10, K);
        component!(11, L);
        component!(12, M);
        component!(13, N);
        component!(14, O);
        component!(15, P);
    }

    (world, expected)
}

fn heavy_world() -> World {
    let world = World::new();
    for _ in 0..HEAVY_ENTITY_COUNT {
        let (t, p, r, v) = heavy_bundle();
        world.entity().set(t).set(p).set(r).set(v);
    }
    world
}

/// Returns (world, random_entity_ids, churn_entity_ids).
/// Entity IDs stored as `FlecsEntity` (`Copy` u64 wrapper) to outlive borrows.
fn mixed_world() -> (World, Vec<FlecsEntity>, Vec<FlecsEntity>) {
    let world = World::new();
    let mut all_ids = Vec::with_capacity(
        MIXED_FRAME_MOVERS + MIXED_FRAME_ENEMIES + MIXED_FRAME_ALLIES + MIXED_FRAME_HEAVY,
    );
    let mut churn_ids = Vec::with_capacity(MIXED_FRAME_CHURN_COUNT);

    for _ in 0..MIXED_FRAME_MOVERS {
        let (p, v) = mixed_mover_bundle();
        let e = world.entity().set(p).set(v);
        let id = e.id();
        if churn_ids.len() < MIXED_FRAME_CHURN_COUNT {
            churn_ids.push(id);
        }
        all_ids.push(id);
    }

    // IsEnemy/IsAlly are zero-sized tags — use `add(component_id)` instead of `set()`.
    let enemy_tag = world.component_id::<IsEnemy>();
    let ally_tag = world.component_id::<IsAlly>();

    for _ in 0..MIXED_FRAME_ENEMIES {
        let (p, v, h, d, _tag) = mixed_enemy_bundle();
        let e = world.entity().set(p).set(v).set(h).set(d).add(enemy_tag);
        all_ids.push(e.id());
    }

    for _ in 0..MIXED_FRAME_ALLIES {
        let (p, v, h, r, _tag) = mixed_ally_bundle();
        let e = world.entity().set(p).set(v).set(h).set(r).add(ally_tag);
        all_ids.push(e.id());
    }

    for _ in 0..MIXED_FRAME_HEAVY {
        let (t, p, v) = mixed_heavy_bundle();
        let e = world.entity().set(t).set(p).set(v);
        all_ids.push(e.id());
    }

    let random_ids = sample_entities(&all_ids, MIXED_FRAME_RANDOM_COUNT);
    (world, random_ids, churn_ids)
}

// ---------------------------------------------------------------------------
// Insert benchmarks
// ---------------------------------------------------------------------------

pub fn bench_insert(group: &mut BenchmarkGroup<'_, WallTime>) {
    let transforms = vec![suite_transform(); SIMPLE_ENTITY_COUNT];
    let positions = vec![suite_position(); SIMPLE_ENTITY_COUNT];
    let rotations = vec![suite_rotation(); SIMPLE_ENTITY_COUNT];
    let velocities = vec![suite_velocity(); SIMPLE_ENTITY_COUNT];

    group.bench_function("bulk_insert_10k/flecs", |b| {
        b.iter_batched_ref(
            prepared_insert_context,
            |(world, _target)| {
                // Keep the binding's public bulk builder and its returned ID
                // vector in the timed region. The target table already exists.
                black_box(
                    world
                        .entity_bulk(SIMPLE_ENTITY_COUNT as u32)
                        .set(&transforms)
                        .set(&positions)
                        .set(&rotations)
                        .set(&velocities)
                        .build(),
                );
                black_box(&world);
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("single_insert_10k/flecs", |b| {
        b.iter_batched_ref(
            prepared_insert_context,
            |(world, target)| {
                for index in 0..SIMPLE_ENTITY_COUNT {
                    target.spawn(
                        world,
                        (
                            transforms[index],
                            positions[index],
                            rotations[index],
                            velocities[index],
                        ),
                    );
                }
                black_box(&world);
            },
            BatchSize::SmallInput,
        );
    });
}

pub fn validate_contract() {
    let (bulk_world, bulk_target) = prepared_insert_context();
    let transforms = vec![suite_transform(); 16];
    let positions = vec![suite_position(); 16];
    let rotations = vec![suite_rotation(); 16];
    let velocities = vec![suite_velocity(); 16];
    let mut transforms = transforms;
    let mut positions = positions;
    let mut rotations = rotations;
    let mut velocities = velocities;
    for index in 0..16 {
        transforms[index].0.x.x = (1_000 + index) as f32;
        positions[index].0.x = (2_000 + index) as f32;
        positions[index].0.y = (2_100 + index) as f32;
        positions[index].0.z = (2_200 + index) as f32;
        rotations[index].0.x = (3_000 + index) as f32;
        rotations[index].0.y = (3_100 + index) as f32;
        rotations[index].0.z = (3_200 + index) as f32;
        velocities[index].0.x = (4_000 + index) as f32;
        velocities[index].0.y = (4_100 + index) as f32;
        velocities[index].0.z = (4_200 + index) as f32;
    }
    let last_bulk_entity = bulk_target.bulk_spawn(
        &bulk_world,
        &transforms,
        &positions,
        &rotations,
        &velocities,
    );
    assert!(bulk_world.is_alive(last_bulk_entity));
    bulk_world
        .entity_from_id(u64::from(last_bulk_entity))
        .get::<(
            &TransformComponent,
            &PositionComponent,
            &RotationComponent,
            &VelocityComponent,
        )>(|(transform, position, rotation, velocity)| {
            assert_eq!(transform.0, transforms[15].0);
            assert_eq!(position.0, positions[15].0);
            assert_eq!(rotation.0, rotations[15].0);
            assert_eq!(velocity.0, velocities[15].0);
        });
    let bulk_query = bulk_world.new_query::<(
        &TransformComponent,
        &PositionComponent,
        &RotationComponent,
        &VelocityComponent,
    )>();
    let mut bulk_count = 0;
    bulk_query.each(|(transform, position, rotation, velocity)| {
        assert_eq!(transform.0, transforms[bulk_count].0);
        assert_eq!(position.0, positions[bulk_count].0);
        assert_eq!(rotation.0, rotations[bulk_count].0);
        assert_eq!(velocity.0, velocities[bulk_count].0);
        bulk_count += 1;
    });
    assert_eq!(bulk_count, 16);

    let (single_world, single_target) = prepared_insert_context();
    for index in 0..16 {
        single_target.spawn(
            &single_world,
            (
                transforms[index],
                positions[index],
                rotations[index],
                velocities[index],
            ),
        );
    }
    let single_query = single_world.new_query::<(
        &TransformComponent,
        &PositionComponent,
        &RotationComponent,
        &VelocityComponent,
    )>();
    let mut single_count = 0;
    single_query.each(|(transform, position, rotation, velocity)| {
        assert_eq!(transform.0, transforms[single_count].0);
        assert_eq!(position.0, positions[single_count].0);
        assert_eq!(rotation.0, rotations[single_count].0);
        assert_eq!(velocity.0, velocities[single_count].0);
        single_count += 1;
    });
    assert_eq!(single_count, 16);

    let world = world_with_entities(128);
    let query = world
        .query::<(&mut PositionComponent, &VelocityComponent)>()
        .set_cache_kind(QueryCacheKind::All)
        .build();
    let mut count = 0;
    let mut checksum = 0.0;
    query.each(|(position, velocity)| {
        position.0 += velocity.0;
        count += 1;
        checksum += position.0.x;
    });
    assert_eq!(count, 128);
    assert_eq!(checksum, 256.0);

    let (position, velocity) = light_bundle();
    let entity = world.entity().set(position).set(velocity);
    let id = entity.id();
    let health_id = world.component_id::<Health>();
    assert!(entity.is_alive());
    entity.set(Health(100.0));
    assert!(entity.has(health_id));
    entity.remove(health_id);
    assert!(!entity.has(health_id));
    entity.destruct();
    assert!(!world.is_alive(id));

    let random_world = World::new();
    let random_entities: Vec<_> = (0..CONTRACT_ENTITY_COUNT)
        .map(|_| {
            let (position, velocity) = light_bundle();
            random_world.entity().set(position).set(velocity).id()
        })
        .collect();
    let mut random_references: Vec<_> = random_entities
        .iter()
        .map(|&entity| {
            random_world
                .entity_from_id(entity)
                .cached_ref(PositionComponent::id())
        })
        .collect();
    let random_checksum = random_references
        .iter_mut()
        .fold(0_u64, |checksum, reference| {
            reference.get(|position| add_position_checksum(checksum, position))
        });
    assert_eq!(
        random_checksum,
        position_checksum_value(1.0, CONTRACT_ENTITY_COUNT)
    );

    let fragmented = fragmented_world();
    let fragmented_query = fragmented
        .query::<&mut DataComponent>()
        .set_cache_kind(QueryCacheKind::All)
        .build();
    let mut fragmented_count = 0;
    let mut fragmented_sum = 0.0;
    fragmented_query.each(|value| {
        value.0 = -value.0;
        fragmented_count += 1;
        fragmented_sum += value.0;
    });
    assert_eq!(
        fragmented_count,
        FRAGMENTED_VARIANT_COUNT * FRAGMENTED_ENTITIES_PER_VARIANT
    );
    assert_eq!(fragmented_sum, -(fragmented_count as f32));

    for component_count in RANDOM_FRAGMENT_COMPONENT_COUNTS {
        let (random_fragmented, expected) =
            random_fragmented_world(component_count, CONTRACT_RANDOM_FRAGMENT_ENTITY_COUNT);
        let query = random_fragmented
            .query::<(&A, &B, &C, &D)>()
            .set_cache_kind(QueryCacheKind::All)
            .build();
        let mut matched = 0;
        let mut values = 0.0;
        let mut random_fragment_checksum = 0_u64;
        query.each_entity(|entity, (a, b, c, d)| {
            matched += 1;
            values += a.0 + b.0 + c.0 + d.0;
            random_fragment_checksum = random_fragment_checksum
                .wrapping_add(*entity.id())
                .wrapping_add(a.0 as u64)
                .wrapping_add(b.0 as u64)
                .wrapping_add(c.0 as u64)
                .wrapping_add(d.0 as u64);
        });
        assert_eq!(matched, expected);
        assert_approx_eq(values, expected as f32 * 40.0);
        assert_ne!(random_fragment_checksum, 0);
    }

    let base_count = world.new_query::<&PositionComponent>().count() as usize;
    let mut entity_ops = Vec::with_capacity(ENTITY_OP_COUNT);
    let light_target = LightTarget::new(&world);
    let target_probe = light_target.spawn(&world);
    let target_probe_view = world.entity_from_id(u64::from(target_probe));
    assert!(target_probe_view.has(world.component_id::<PositionComponent>()));
    assert!(target_probe_view.has(world.component_id::<VelocityComponent>()));
    delete_entity(&world, target_probe);
    spawn_despawn_target(&world, &light_target, &mut entity_ops);
    assert_eq!(
        world.new_query::<&PositionComponent>().count() as usize,
        base_count
    );
    assert!(entity_ops.iter().all(|&entity| !world.is_alive(entity)));

    let add_remove_world = World::new();
    let add_remove_ids: Vec<_> = (0..ENTITY_OP_COUNT)
        .map(|_| {
            let (position, velocity) = light_bundle();
            add_remove_world.entity().set(position).set(velocity).id()
        })
        .collect();
    let add_remove_health_id = add_remove_world.component_id::<Health>();
    for &add_remove_id in &add_remove_ids {
        add_remove_world
            .entity_from_id(add_remove_id)
            .set(Health(100.0));
    }
    assert!(add_remove_ids.iter().all(|&add_remove_id| add_remove_world
        .entity_from_id(add_remove_id)
        .has(add_remove_health_id)));
    for &add_remove_id in &add_remove_ids {
        add_remove_world
            .entity_from_id(add_remove_id)
            .remove(add_remove_health_id);
    }
    assert!(add_remove_ids.iter().all(|&add_remove_id| !add_remove_world
        .entity_from_id(add_remove_id)
        .has(add_remove_health_id)));

    let (mixed, random, churn) = mixed_world();
    let mixed_light_target = LightTarget::new(&mixed);
    let expected = mixed.new_query::<&PositionComponent>().count();
    let mut spawned = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);
    let move_query = mixed.new_query::<(&mut PositionComponent, &VelocityComponent)>();
    let enemy_query = mixed.new_query::<(&mut Health, &Damage)>();
    let ally_query = mixed.new_query::<(&mut Health, &Regen)>();
    let heavy_query = mixed.new_query::<(&mut PositionComponent, &TransformComponent)>();
    move_query.each(|(position, velocity)| position.0 += velocity.0);
    enemy_query.each(|(health, damage)| health.0 -= damage.0);
    ally_query.each(|(health, regen)| health.0 += regen.0);
    heavy_query.each(|(position, transform)| {
        let base = transform.0;
        let mut matrix = base;
        for _ in 0..MIXED_FRAME_INVERT_COUNT {
            matrix = black_box(base)
                .invert()
                .expect("mixed-frame matrix should be invertible");
        }
        position.0 = matrix.transform_vector(position.0);
    });
    let mut mixed_random_references: Vec<_> = random
        .iter()
        .map(|&random_id| {
            mixed
                .entity_from_id(random_id)
                .cached_ref(PositionComponent::id())
        })
        .collect();
    let random_checksum = mixed_random_references
        .iter_mut()
        .fold(0_u64, |checksum, reference| {
            reference.get(|position| add_position_checksum(checksum, position))
        });
    assert_ne!(random_checksum, 0);
    let health_id = mixed.component_id::<Health>();
    for &churn_id in &churn {
        mixed.entity_from_id(churn_id).set(Health(100.0));
        assert!(mixed.entity_from_id(churn_id).has(health_id));
    }
    for &churn_id in &churn {
        mixed.entity_from_id(churn_id).remove(health_id);
    }
    mixed_spawn_step_for_validation(&mixed, &mixed_light_target, &mut spawned);
    assert_eq!(mixed.new_query::<&PositionComponent>().count(), expected);
    assert!(!mixed.entity_from_id(churn[0]).has(health_id));
    assert!(spawned.iter().all(|&entity| !mixed.is_alive(entity)));
    let refreshed_random_checksum = mixed_random_references
        .iter_mut()
        .fold(0_u64, |checksum, reference| {
            reference.get(|position| add_position_checksum(checksum, position))
        });
    assert_ne!(refreshed_random_checksum, 0);

    let mut position_count = 0;
    let mut position_sum = 0.0;
    mixed.new_query::<&PositionComponent>().each(|position| {
        position_count += 1;
        position_sum += position.0.x;
    });
    assert_eq!(
        position_count,
        MIXED_FRAME_MOVERS + MIXED_FRAME_ENEMIES + MIXED_FRAME_ALLIES + MIXED_FRAME_HEAVY
    );
    assert_approx_eq(position_sum, 18_500.0);

    let mut health_count = 0;
    let mut health_sum = 0.0;
    mixed.new_query::<&Health>().each(|health| {
        health_count += 1;
        health_sum += health.0;
    });
    assert_eq!(health_count, MIXED_FRAME_ENEMIES + MIXED_FRAME_ALLIES);
    assert_approx_eq(health_sum, 638_400.0);

    let mut deferred_ids = Vec::with_capacity(ENTITY_OP_COUNT);
    spawn_despawn_deferred(&mixed, &mut deferred_ids);
    assert_eq!(mixed.new_query::<&PositionComponent>().count(), expected);
    assert!(deferred_ids.iter().all(|&entity| !mixed.is_alive(entity)));
}

fn mixed_spawn_step_for_validation(
    world: &World,
    target: &LightTarget,
    spawned: &mut Vec<FlecsEntity>,
) {
    spawned.clear();
    for _ in 0..MIXED_FRAME_SPAWN_COUNT {
        spawned.push(target.spawn(world));
    }
    for &entity in spawned.iter() {
        delete_entity(world, entity);
    }
}

fn spawn_despawn_target(world: &World, target: &LightTarget, ids: &mut Vec<FlecsEntity>) {
    ids.clear();
    for _ in 0..ENTITY_OP_COUNT {
        ids.push(target.spawn(world));
    }
    for &id in ids.iter() {
        delete_entity(world, id);
    }
}

fn spawn_despawn_deferred(world: &World, ids: &mut Vec<FlecsEntity>) {
    ids.clear();
    world.defer_begin();
    for _ in 0..ENTITY_OP_COUNT {
        let (position, velocity) = light_bundle();
        ids.push(world.entity().set(position).set(velocity).id());
    }
    for &id in ids.iter() {
        world.entity_from_id(id).destruct();
    }
    world.defer_end();
}

// ---------------------------------------------------------------------------
// Iteration benchmarks  (queries created OUTSIDE the timed loop)
// ---------------------------------------------------------------------------

pub fn bench_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    let world = world_with_entities(SIMPLE_ENTITY_COUNT);
    // Long-lived per-frame queries use Flecs' fully cached prepared path.
    let query = world
        .query::<(&mut PositionComponent, &VelocityComponent)>()
        .set_cache_kind(QueryCacheKind::All)
        .build();

    group.bench_function("simple_10k/flecs", |b| {
        b.iter(|| {
            query.each(|(pos, vel)| {
                pos.0 += vel.0;
            });
            black_box(&world);
        });
    });
}

pub fn bench_iteration_repeated(group: &mut BenchmarkGroup<'_, WallTime>) {
    let world = world_with_entities(SIMPLE_ENTITY_COUNT);
    let query = world
        .query::<(&mut PositionComponent, &VelocityComponent)>()
        .set_cache_kind(QueryCacheKind::All)
        .build();

    group.bench_function("simple_x32/flecs", |b| {
        b.iter(|| {
            for _ in 0..REPEATED_ITERATION_COUNT {
                query.each(|(pos, vel)| {
                    pos.0 += vel.0;
                });
            }
            black_box(&world);
        });
    });
}

pub fn bench_iteration_large(group: &mut BenchmarkGroup<'_, WallTime>) {
    let world = world_with_entities(LARGE_ITERATION_ENTITY_COUNT);
    let query = world
        .query::<(&mut PositionComponent, &VelocityComponent)>()
        .set_cache_kind(QueryCacheKind::All)
        .build();

    group.bench_function("simple_100k/flecs", |b| {
        b.iter(|| {
            query.each(|(pos, vel)| {
                pos.0 += vel.0;
            });
            black_box(&world);
        });
    });
}

pub fn bench_fragmented_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    debug_assert_eq!(FRAGMENTED_VARIANT_COUNT, 26);

    let world = fragmented_world();
    let query = world
        .query::<&mut DataComponent>()
        .set_cache_kind(QueryCacheKind::All)
        .build();

    group.bench_function("fragmented_26x400/flecs", |b| {
        b.iter(|| {
            query.each(|data| {
                data.0 = -data.0;
            });
            black_box(&world);
        });
    });
}

pub fn bench_random_fragmented_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    for component_count in RANDOM_FRAGMENT_COMPONENT_COUNTS {
        let (world, expected) =
            random_fragmented_world(component_count, RANDOM_FRAGMENT_ENTITY_COUNT);
        let query = world
            .query::<(&A, &B, &C, &D)>()
            .set_cache_kind(QueryCacheKind::All)
            .build();
        let mut initial_count = 0;
        query.each_entity(|_, _| initial_count += 1);
        assert_eq!(initial_count, expected);

        group.bench_function(
            format!("random_{component_count}_components_4_terms/flecs"),
            |b| {
                let mut checksum = 0_u64;
                b.iter(|| {
                    query.each_entity(|entity, (a, b, c, d)| {
                        checksum = checksum.wrapping_add(*entity.id());
                        checksum = checksum.wrapping_add(a.0 as u64);
                        checksum = checksum.wrapping_add(b.0 as u64);
                        checksum = checksum.wrapping_add(c.0 as u64);
                        checksum = checksum.wrapping_add(d.0 as u64);
                    });
                });
                black_box(checksum);
            },
        );
    }
}

pub fn bench_heavy_compute(group: &mut BenchmarkGroup<'_, WallTime>) {
    let world = heavy_world();
    let query = world
        .query::<(&mut PositionComponent, &mut TransformComponent)>()
        .set_cache_kind(QueryCacheKind::All)
        .build();

    group.bench_function("heavy/flecs", |b| {
        b.iter(|| {
            query.each(|(position, transform)| {
                let base = transform.0;
                let mut matrix = base;
                for _ in 0..HEAVY_INVERT_COUNT {
                    matrix = black_box(base)
                        .invert()
                        .expect("base heavy matrix should be invertible");
                }
                position.0 = matrix.transform_vector(position.0);
            });
            black_box(&world);
        });
    });
}

// ---------------------------------------------------------------------------
// Random access benchmark
// ---------------------------------------------------------------------------

pub fn bench_random_access(group: &mut BenchmarkGroup<'_, WallTime>) {
    for (name, count) in [
        ("hot_10k", SIMPLE_ENTITY_COUNT),
        ("warm_100k", WARM_RANDOM_ENTITY_COUNT),
        ("cold_1m", COLD_RANDOM_ENTITY_COUNT),
    ] {
        let world = World::new();
        let ids: Vec<FlecsEntity> = (0..count)
            .map(|_| {
                let (position, velocity) = light_bundle();
                world.entity().set(position).set(velocity).id()
            })
            .collect();
        let mut references: Vec<_> = ids
            .iter()
            .map(|&id| world.entity_from_id(id).cached_ref(PositionComponent::id()))
            .collect();
        let indices: Vec<_> = (0..count).collect();
        let orders = deterministic_orders(&indices);
        let mut order = 0;
        group.bench_function(format!("{name}/flecs"), |b| {
            b.iter(|| {
                let indices = &orders[order % orders.len()];
                order += 1;
                let mut checksum = 0_u64;
                for &index in indices {
                    checksum =
                        references[index].get(|position| add_position_checksum(checksum, position));
                }
                black_box(checksum);
            });
        });
    }
}

// ---------------------------------------------------------------------------
// Entity operations benchmark
// ---------------------------------------------------------------------------

pub fn bench_entity_ops(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("spawn_despawn_1k/flecs", |b| {
        let world = World::new();
        let target = LightTarget::new(&world);
        let mut ids = Vec::with_capacity(ENTITY_OP_COUNT);
        b.iter(|| {
            spawn_despawn_target(&world, &target, &mut ids);
            black_box(&world);
        });
    });

    group.bench_function("add_remove_component_1k/flecs", |b| {
        let world = World::new();
        let ids: Vec<FlecsEntity> = (0..ENTITY_OP_COUNT)
            .map(|_| {
                let (p, v) = light_bundle();
                world.entity().set(p).set(v).id()
            })
            .collect();
        let health_id = world.component_id::<Health>();

        b.iter(|| {
            for &id in &ids {
                world.entity_from_id(id).set(Health(100.0));
            }
            for &id in &ids {
                world.entity_from_id(id).remove(health_id);
            }
            black_box(&world);
        });
    });
}

// ---------------------------------------------------------------------------
// Mixed frame benchmark  (queries created OUTSIDE the timed loop)
// ---------------------------------------------------------------------------

pub fn bench_mixed_frame(group: &mut BenchmarkGroup<'_, WallTime>) {
    let (world, random_ids, churn_ids) = mixed_world();
    let mut random_references: Vec<_> = random_ids
        .iter()
        .map(|&id| world.entity_from_id(id).cached_ref(PositionComponent::id()))
        .collect();
    let light_target = LightTarget::new(&world);
    let mut spawned_ids = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);

    // Keep Flecs' cached reusable query objects outside the timed loop.
    let move_q = world
        .query::<(&mut PositionComponent, &VelocityComponent)>()
        .set_cache_kind(QueryCacheKind::All)
        .build();
    let enemy_q = world
        .query::<(&mut Health, &Damage)>()
        .set_cache_kind(QueryCacheKind::All)
        .build();
    let ally_q = world
        .query::<(&mut Health, &Regen)>()
        .set_cache_kind(QueryCacheKind::All)
        .build();
    let heavy_q = world
        .query::<(&mut PositionComponent, &TransformComponent)>()
        .set_cache_kind(QueryCacheKind::All)
        .build();
    let health_id = world.component_id::<Health>();

    group.bench_function("frame/flecs", |b| {
        b.iter(|| {
            // Movement
            move_q.each(|(pos, vel)| {
                pos.0 += vel.0;
            });
            // Health
            enemy_q.each(|(health, damage)| {
                health.0 -= damage.0;
            });
            ally_q.each(|(health, regen)| {
                health.0 += regen.0;
            });
            // Heavy compute
            heavy_q.each(|(position, transform)| {
                let base = transform.0;
                let mut matrix = base;
                for _ in 0..MIXED_FRAME_INVERT_COUNT {
                    matrix = black_box(base)
                        .invert()
                        .expect("mixed-frame matrix should be invertible");
                }
                position.0 = matrix.transform_vector(position.0);
            });
            // Random access
            let mut random_checksum = 0_u64;
            for reference in &mut random_references {
                random_checksum =
                    reference.get(|position| add_position_checksum(random_checksum, position));
            }
            // Structural churn
            for &id in &churn_ids {
                world.entity_from_id(id).set(Health(100.0));
            }
            for &id in &churn_ids {
                world.entity_from_id(id).remove(health_id);
            }
            // Spawn/despawn
            spawned_ids.clear();
            for _ in 0..MIXED_FRAME_SPAWN_COUNT {
                spawned_ids.push(light_target.spawn(&world));
            }
            for &id in spawned_ids.iter() {
                delete_entity(&world, id);
            }

            black_box(random_checksum);
            black_box(&world);
        });
    });
}

// ---------------------------------------------------------------------------
// Mixed frame phases benchmark  (queries created OUTSIDE the timed loop)
// ---------------------------------------------------------------------------

pub fn bench_mixed_frame_phases(group: &mut BenchmarkGroup<'_, WallTime>) {
    {
        let (world, _, _) = mixed_world();
        let query = world
            .query::<(&mut PositionComponent, &VelocityComponent)>()
            .set_cache_kind(QueryCacheKind::All)
            .build();
        group.bench_function("movement/flecs", |b| {
            b.iter(|| {
                query.each(|(pos, vel)| {
                    pos.0 += vel.0;
                });
                black_box(&world);
            });
        });
    }

    {
        let (world, _, _) = mixed_world();
        let enemy_q = world
            .query::<(&mut Health, &Damage)>()
            .set_cache_kind(QueryCacheKind::All)
            .build();
        let ally_q = world
            .query::<(&mut Health, &Regen)>()
            .set_cache_kind(QueryCacheKind::All)
            .build();
        group.bench_function("health/flecs", |b| {
            b.iter(|| {
                for _ in 0..MIXED_PHASE_HEALTH_REPEAT {
                    enemy_q.each(|(health, damage)| {
                        health.0 -= damage.0;
                    });
                    ally_q.each(|(health, regen)| {
                        health.0 += regen.0;
                    });
                }
                black_box(&world);
            });
        });
    }

    {
        let (world, _, _) = mixed_world();
        let query = world
            .query::<(&mut PositionComponent, &TransformComponent)>()
            .set_cache_kind(QueryCacheKind::All)
            .build();
        group.bench_function("heavy/flecs", |b| {
            b.iter(|| {
                query.each(|(position, transform)| {
                    let base = transform.0;
                    let mut matrix = base;
                    for _ in 0..MIXED_FRAME_INVERT_COUNT {
                        matrix = black_box(base)
                            .invert()
                            .expect("mixed-frame matrix should be invertible");
                    }
                    position.0 = matrix.transform_vector(position.0);
                });
                black_box(&world);
            });
        });
    }

    {
        let (world, random_ids, _) = mixed_world();
        let mut random_references: Vec<_> = random_ids
            .iter()
            .map(|&id| world.entity_from_id(id).cached_ref(PositionComponent::id()))
            .collect();
        group.bench_function("random_access/flecs", |b| {
            b.iter(|| {
                let mut checksum = 0_u64;
                for reference in &mut random_references {
                    checksum = reference.get(|position| add_position_checksum(checksum, position));
                }
                black_box(checksum);
                black_box(&world);
            });
        });
    }

    {
        let (world, _, churn_ids) = mixed_world();
        let health_id = world.component_id::<Health>();
        group.bench_function("structural_churn/flecs", |b| {
            b.iter(|| {
                for &id in &churn_ids {
                    world.entity_from_id(id).set(Health(100.0));
                }
                for &id in &churn_ids {
                    world.entity_from_id(id).remove(health_id);
                }
                black_box(&world);
            });
        });
    }

    {
        let (world, _, _) = mixed_world();
        let light_target = LightTarget::new(&world);
        let mut spawned_ids = Vec::with_capacity(MIXED_FRAME_SPAWN_COUNT);
        group.bench_function("spawn_despawn/flecs", |b| {
            b.iter(|| {
                for _ in 0..MIXED_PHASE_SPAWN_REPEAT {
                    spawned_ids.clear();
                    for _ in 0..MIXED_FRAME_SPAWN_COUNT {
                        spawned_ids.push(light_target.spawn(&world));
                    }
                    for &id in spawned_ids.iter() {
                        delete_entity(&world, id);
                    }
                }
                black_box(&world);
            });
        });
    }
}
