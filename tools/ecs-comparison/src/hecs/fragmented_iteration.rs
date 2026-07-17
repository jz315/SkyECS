use super::dense_iteration::assert_prepared_count;
use super::*;

pub(super) fn fragmented_world() -> World {
    let mut world = World::new();

    macro_rules! add_variant {
        ($world:ident; $($tag:ident),* $(,)?) => {
            $( $world.spawn_batch((0..FRAGMENTED_ENTITIES_PER_VARIANT).map(|_| ($tag(0.0), DataComponent(1.0)))); )*
        };
    }

    add_variant!(world; A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
    world
}
pub fn bench_fragmented_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    debug_assert_eq!(FRAGMENTED_VARIANT_COUNT, 26);

    group.bench_function("fragmented_26x400/hecs", |b| {
        let world = fragmented_world();
        let mut query = PreparedQuery::<&mut DataComponent>::default();
        assert_prepared_count(
            &mut query,
            &world,
            FRAGMENTED_VARIANT_COUNT * FRAGMENTED_ENTITIES_PER_VARIANT,
        );
        b.iter(|| {
            for data in query.query(&world).iter() {
                data.0 = -data.0;
            }
            black_box(&world);
        });
    });
}
