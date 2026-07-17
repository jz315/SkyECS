use super::*;

pub(super) fn fragmented_world() -> World {
    let mut world = World::new();

    macro_rules! add_variant {
        ($world:ident; $($tag:ident),* $(,)?) => {
            $( $world.bulk_add_entity((0..FRAGMENTED_ENTITIES_PER_VARIANT).map(|_| ($tag(0.0), DataComponent(1.0)))); )*
        };
    }

    add_variant!(world; A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
    world
}
pub fn bench_fragmented_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    debug_assert_eq!(FRAGMENTED_VARIANT_COUNT, 26);
    group.bench_function("fragmented_26x400/shipyard", |b| {
        let world = fragmented_world();
        let mut data = world.borrow::<ViewMut<DataComponent>>().unwrap();
        b.iter(|| {
            (&mut data).iter().for_each(|data| data.0 = -data.0);
            black_box(&world);
        });
    });
}
