use super::*;

pub(super) fn fragmented_world() -> World {
    let mut world = World::new();

    macro_rules! add_variant {
        ($tag:ty) => {{
            for _ in 0..FRAGMENTED_ENTITIES_PER_VARIANT {
                world.spawn((<$tag>::default(), DataComponent(1.0)));
            }
        }};
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

pub fn bench_fragmented_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    debug_assert_eq!(FRAGMENTED_VARIANT_COUNT, 26);
    group.bench_function("fragmented_26x400/sky", |b| {
        let mut world = fragmented_world();
        let mut query = PreparedQuery::<&mut DataComponent>::new();
        assert_eq!(
            query.count(&world),
            FRAGMENTED_VARIANT_COUNT * FRAGMENTED_ENTITIES_PER_VARIANT
        );
        b.iter(|| {
            query.for_each_chunk(&mut world, |data| {
                for value in data {
                    value.0 = -value.0;
                }
            });
            black_box(&world);
        });
    });
}
