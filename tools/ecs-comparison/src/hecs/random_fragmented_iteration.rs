use super::*;

pub(super) fn random_fragmented_component_world(
    component_count: usize,
    entity_count: usize,
) -> (World, Vec<u16>) {
    let masks = random_fragment_masks_for(component_count, entity_count);
    let mut world = World::new();

    for &mask in &masks {
        let entity = world.spawn(());
        macro_rules! component {
            ($bit:expr, $component:ident) => {
                if mask & (1 << $bit) != 0 {
                    world
                        .insert_one(entity, $component(10.0))
                        .expect("random component insertion should succeed");
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

    (world, masks)
}

pub(super) fn random_fragmented_tag_world(
    component_count: usize,
    entity_count: usize,
) -> (World, Vec<u16>) {
    let masks = random_fragment_masks_for(component_count, entity_count);
    let mut world = World::new();

    for &mask in &masks {
        let entity = world.spawn(());
        macro_rules! tag {
            ($bit:expr, $tag:ident) => {
                if mask & (1 << $bit) != 0 {
                    world
                        .insert_one(entity, $tag)
                        .expect("random tag insertion should succeed");
                }
            };
        }
        tag!(0, TagA);
        tag!(1, TagB);
        tag!(2, TagC);
        tag!(3, TagD);
        tag!(4, TagE);
        tag!(5, TagF);
        tag!(6, TagG);
        tag!(7, TagH);
        tag!(8, TagI);
        tag!(9, TagJ);
        tag!(10, TagK);
        tag!(11, TagL);
        tag!(12, TagM);
        tag!(13, TagN);
        tag!(14, TagO);
        tag!(15, TagP);
    }

    (world, masks)
}
fn bench_random_component_1(group: &mut BenchmarkGroup<'_, WallTime>, component_count: usize) {
    group.bench_function(
        format!("random_{component_count}_components_1_term/hecs"),
        |bencher| {
            let (world, masks) =
                random_fragmented_component_world(component_count, RANDOM_FRAGMENT_ENTITY_COUNT);
            let expected = random_fragment_match_count(&masks, 1);
            let mut query = PreparedQuery::<(hecs::Entity, &A)>::default();
            assert_eq!(query.query(&world).iter().count(), expected);
            bencher.iter(|| {
                let mut checksum = 0_u64;
                query.query(&world).iter().for_each(|(entity, a)| {
                    checksum = add_random_fragment_component_1_checksum(
                        checksum,
                        entity.to_bits().get(),
                        a.0,
                    );
                });
                checksum
            });
        },
    );
}

fn bench_random_component_4(group: &mut BenchmarkGroup<'_, WallTime>, component_count: usize) {
    group.bench_function(
        format!("random_{component_count}_components_4_terms/hecs"),
        |bencher| {
            let (world, masks) =
                random_fragmented_component_world(component_count, RANDOM_FRAGMENT_ENTITY_COUNT);
            let expected = random_fragment_match_count(&masks, 4);
            let mut query = PreparedQuery::<(hecs::Entity, &A, &B, &C, &D)>::default();
            assert_eq!(query.query(&world).iter().count(), expected);
            bencher.iter(|| {
                let mut checksum = 0_u64;
                query.query(&world).iter().for_each(|(entity, a, b, c, d)| {
                    checksum = add_random_fragment_checksum(
                        checksum,
                        entity.to_bits().get(),
                        a.0,
                        b.0,
                        c.0,
                        d.0,
                    );
                });
                checksum
            });
        },
    );
}

fn bench_random_component_8(group: &mut BenchmarkGroup<'_, WallTime>, component_count: usize) {
    group.bench_function(
        format!("random_{component_count}_components_8_terms/hecs"),
        |bencher| {
            let (world, masks) =
                random_fragmented_component_world(component_count, RANDOM_FRAGMENT_ENTITY_COUNT);
            let expected = random_fragment_match_count(&masks, 8);
            let mut query =
                PreparedQuery::<(hecs::Entity, &A, &B, &C, &D, &E, &F, &G, &H)>::default();
            assert_eq!(query.query(&world).iter().count(), expected);
            bencher.iter(|| {
                let mut checksum = 0_u64;
                query
                    .query(&world)
                    .iter()
                    .for_each(|(entity, a, b, c, d, e, f, g, h)| {
                        checksum = add_random_fragment_component_8_checksum(
                            checksum,
                            entity.to_bits().get(),
                            a.0,
                            b.0,
                            c.0,
                            d.0,
                            e.0,
                            f.0,
                            g.0,
                            h.0,
                        );
                    });
                checksum
            });
        },
    );
}

macro_rules! bench_random_tags {
    ($name:ident, $terms:literal, $query:ty, $pattern:pat_param, $entity:ident) => {
        fn $name(group: &mut BenchmarkGroup<'_, WallTime>, component_count: usize) {
            let suffix = if $terms == 1 { "term" } else { "terms" };
            group.bench_function(
                format!("random_{component_count}_tags_{}_{suffix}/hecs", $terms),
                |bencher| {
                    let (world, masks) =
                        random_fragmented_tag_world(component_count, RANDOM_FRAGMENT_ENTITY_COUNT);
                    let expected = random_fragment_match_count(&masks, $terms);
                    let mut query = PreparedQuery::<$query>::default();
                    assert_eq!(query.query(&world).iter().count(), expected);
                    bencher.iter(|| {
                        let mut checksum = 0_u64;
                        query.query(&world).iter().for_each(|$pattern| {
                            checksum = checksum.wrapping_add($entity.to_bits().get());
                        });
                        checksum
                    });
                },
            );
        }
    };
}

bench_random_tags!(
    bench_random_tag_1,
    1,
    (hecs::Entity, &TagA),
    (entity, _),
    entity
);
bench_random_tags!(
    bench_random_tag_4,
    4,
    (hecs::Entity, &TagA, &TagB, &TagC, &TagD),
    (entity, _, _, _, _),
    entity
);
bench_random_tags!(
    bench_random_tag_8,
    8,
    (
        hecs::Entity,
        &TagA,
        &TagB,
        &TagC,
        &TagD,
        &TagE,
        &TagF,
        &TagG,
        &TagH
    ),
    (entity, _, _, _, _, _, _, _, _),
    entity
);

pub fn bench_random_fragmented_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    for (component_count, term_count) in RANDOM_FRAGMENT_WORKLOADS {
        match term_count {
            1 => bench_random_tag_1(group, component_count),
            4 => bench_random_tag_4(group, component_count),
            8 => bench_random_tag_8(group, component_count),
            _ => unreachable!(),
        }
    }
    for (component_count, term_count) in RANDOM_FRAGMENT_WORKLOADS {
        match term_count {
            1 => bench_random_component_1(group, component_count),
            4 => bench_random_component_4(group, component_count),
            8 => bench_random_component_8(group, component_count),
            _ => unreachable!(),
        }
    }
}
