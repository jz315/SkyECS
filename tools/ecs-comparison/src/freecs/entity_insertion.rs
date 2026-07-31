use super::*;
use ::freecs::ecs;

macro_rules! define_fragment_components {
    ($($name:ident),+ $(,)?) => {
        $(
            #[repr(transparent)]
            #[derive(Clone, Copy, Default)]
            pub struct $name(pub f32);
        )+
    };
}

define_fragment_components!(
    FragmentA, FragmentB, FragmentC, FragmentD, FragmentE, FragmentF, FragmentG, FragmentH,
    FragmentI, FragmentJ, FragmentK, FragmentL, FragmentM, FragmentN, FragmentO, FragmentP,
    FragmentQ, FragmentR, FragmentS, FragmentT, FragmentU, FragmentV, FragmentW, FragmentX,
    FragmentY, FragmentZ,
);

ecs! {
    World {
        transform: TransformComponent => TRANSFORM_MASK,
        position: PositionComponent => POSITION_MASK,
        rotation: RotationComponent => ROTATION_MASK,
        velocity: VelocityComponent => VELOCITY_MASK,
        data: DataComponent => DATA_MASK,
        health: Health => HEALTH_MASK,
        damage: Damage => DAMAGE_MASK,
        regen: Regen => REGEN_MASK,
        is_enemy: IsEnemy => IS_ENEMY_MASK,
        is_ally: IsAlly => IS_ALLY_MASK,
        lifetime: Lifetime => LIFETIME_MASK,
        target_slot: TargetSlot => TARGET_SLOT_MASK,
        cooldown: Cooldown => COOLDOWN_MASK,
        owner_slot: OwnerSlot => OWNER_SLOT_MASK,
        stunned: Stunned => STUNNED_MASK,
        fragment_a: FragmentA => A_MASK,
        fragment_b: FragmentB => B_MASK,
        fragment_c: FragmentC => C_MASK,
        fragment_d: FragmentD => D_MASK,
        fragment_e: FragmentE => E_MASK,
        fragment_f: FragmentF => F_MASK,
        fragment_g: FragmentG => G_MASK,
        fragment_h: FragmentH => H_MASK,
        fragment_i: FragmentI => I_MASK,
        fragment_j: FragmentJ => J_MASK,
        fragment_k: FragmentK => K_MASK,
        fragment_l: FragmentL => L_MASK,
        fragment_m: FragmentM => M_MASK,
        fragment_n: FragmentN => N_MASK,
        fragment_o: FragmentO => O_MASK,
        fragment_p: FragmentP => P_MASK,
        fragment_q: FragmentQ => Q_MASK,
        fragment_r: FragmentR => R_MASK,
        fragment_s: FragmentS => S_MASK,
        fragment_t: FragmentT => T_MASK,
        fragment_u: FragmentU => U_MASK,
        fragment_v: FragmentV => V_MASK,
        fragment_w: FragmentW => W_MASK,
        fragment_x: FragmentX => X_MASK,
        fragment_y: FragmentY => Y_MASK,
        fragment_z: FragmentZ => Z_MASK,
        tag_a: TagA => TAG_A_MASK,
        tag_b: TagB => TAG_B_MASK,
        tag_c: TagC => TAG_C_MASK,
        tag_d: TagD => TAG_D_MASK,
        tag_e: TagE => TAG_E_MASK,
        tag_f: TagF => TAG_F_MASK,
        tag_g: TagG => TAG_G_MASK,
        tag_h: TagH => TAG_H_MASK,
        tag_i: TagI => TAG_I_MASK,
        tag_j: TagJ => TAG_J_MASK,
        tag_k: TagK => TAG_K_MASK,
        tag_l: TagL => TAG_L_MASK,
        tag_m: TagM => TAG_M_MASK,
        tag_n: TagN => TAG_N_MASK,
        tag_o: TagO => TAG_O_MASK,
        tag_p: TagP => TAG_P_MASK,
    }
    Resources {}
}

pub(super) const SUITE_MASK: u64 = TRANSFORM_MASK | POSITION_MASK | ROTATION_MASK | VELOCITY_MASK;
pub(super) const LIGHT_MASK: u64 = POSITION_MASK | VELOCITY_MASK;
pub(super) const MOVE_MASK: u64 = POSITION_MASK | VELOCITY_MASK;
pub(super) const ENEMY_HEALTH_MASK: u64 = HEALTH_MASK | DAMAGE_MASK;
pub(super) const ALLY_HEALTH_MASK: u64 = HEALTH_MASK | REGEN_MASK;
pub(super) const HEAVY_MASK: u64 = POSITION_MASK | TRANSFORM_MASK;

pub(super) fn prepared_insert_world() -> World {
    // FreeCS declares its component schema in the `ecs!` expansion, so a new
    // empty World is already at its zero-entity prepared schema state.
    World::default()
}

pub(super) struct BulkConstructionContext {
    pub world: World,
    pub columns: SuiteColumns,
    pub entities: Vec<Entity>,
}

pub(super) fn bulk_construction_context(columns: SuiteColumns) -> BulkConstructionContext {
    BulkConstructionContext {
        world: prepared_insert_world(),
        columns,
        entities: Vec::new(),
    }
}

pub(super) fn insert_bulk_from_columns(context: &mut BulkConstructionContext) {
    let count = context.columns.0.len();
    assert_eq!(context.columns.1.len(), count);
    assert_eq!(context.columns.2.len(), count);
    assert_eq!(context.columns.3.len(), count);
    let mut transforms = context.columns.0.drain(..);
    let mut positions = context.columns.1.drain(..);
    let mut rotations = context.columns.2.drain(..);
    let mut velocities = context.columns.3.drain(..);
    context.entities = context
        .world
        .spawn_batch(SUITE_MASK, count, |table, index| {
            table.transform[index] = transforms.next().expect("one transform per row");
            table.position[index] = positions.next().expect("one position per row");
            table.rotation[index] = rotations.next().expect("one rotation per row");
            table.velocity[index] = velocities.next().expect("one velocity per row");
        });
}

pub(super) fn spawn_suite_batch(world: &mut World, count: usize) -> Vec<Entity> {
    world.spawn_batch(SUITE_MASK, count, |table, index| {
        let (transform, position, rotation, velocity) = suite_bundle();
        table.transform[index] = transform;
        table.position[index] = position;
        table.rotation[index] = rotation;
        table.velocity[index] = velocity;
    })
}

pub(super) fn spawn_suite_bundles(world: &mut World, bundles: &[SuiteBundle]) -> Vec<Entity> {
    let mut bundles = bundles.iter().copied();
    world.spawn_batch(SUITE_MASK, bundles.len(), |table, index| {
        // FreeCS passes the absolute row in the destination table, not the
        // batch-local offset. Consume the prepared inputs in call order.
        let (transform, position, rotation, velocity) = bundles
            .next()
            .expect("FreeCS should initialize each requested entity once");
        table.transform[index] = transform;
        table.position[index] = position;
        table.rotation[index] = rotation;
        table.velocity[index] = velocity;
    })
}

pub(super) fn spawn_suite_bundle(world: &mut World, bundle: SuiteBundle) -> Entity {
    spawn_suite_bundles(world, std::slice::from_ref(&bundle))
        .pop()
        .expect("FreeCS should return the spawned entity")
}
pub fn bench_bulk_construction(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("bulk_from_columns_10k/freecs", |b| {
        b.iter_batched_ref(
            || bulk_construction_context(suite_columns(SIMPLE_ENTITY_COUNT)),
            |context| {
                insert_bulk_from_columns(context);
                black_box(&context.entities);
                black_box(&context.world);
            },
            construction_batch_size(),
        );
    });
}

pub fn bench_single_insert(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("single_insert_10k/freecs", |b| {
        let bundles = suite_bundles(SIMPLE_ENTITY_COUNT);
        b.iter_batched_ref(
            prepared_insert_world,
            |world| {
                for &bundle in &bundles {
                    let _ = spawn_suite_bundle(world, bundle);
                }
                black_box(&world);
            },
            construction_batch_size(),
        );
    });
}
