use super::*;

pub(super) fn fragmented_world() -> World {
    let mut world = World::default();

    for component_mask in [
        A_MASK, B_MASK, C_MASK, D_MASK, E_MASK, F_MASK, G_MASK, H_MASK, I_MASK, J_MASK, K_MASK,
        L_MASK, M_MASK, N_MASK, O_MASK, P_MASK, Q_MASK, R_MASK, S_MASK, T_MASK, U_MASK, V_MASK,
        W_MASK, X_MASK, Y_MASK, Z_MASK,
    ] {
        world.spawn_batch(
            component_mask | DATA_MASK,
            FRAGMENTED_ENTITIES_PER_VARIANT,
            |table, index| {
                table.data[index] = DataComponent(1.0);
            },
        );
    }

    world
}
pub fn bench_fragmented_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    debug_assert_eq!(FRAGMENTED_VARIANT_COUNT, 26);

    group.bench_function("fragmented_26x400/freecs", |b| {
        let mut world = fragmented_world();
        b.iter(|| {
            world.for_each_data_mut(|data| {
                data.0 = -data.0;
            });
            black_box(&world);
        });
    });
}
