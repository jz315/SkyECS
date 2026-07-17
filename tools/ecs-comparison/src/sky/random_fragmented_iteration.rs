use super::*;

pub(super) fn random_fragmented_component_world(
    component_count: usize,
    entity_count: usize,
) -> (World, Vec<u16>) {
    let masks = random_fragment_masks_for(component_count, entity_count);
    let mut world = World::new();

    for &mask in &masks {
        let entity = world
            .spawn_dynamic(DynamicBundle::new())
            .expect("empty dynamic bundle should be valid");
        macro_rules! component {
            ($bit:expr, $component:ident) => {
                if mask & (1 << $bit) != 0 {
                    assert!(world.insert(entity, $component(10.0)));
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
        let entity = world
            .spawn_dynamic(DynamicBundle::new())
            .expect("empty dynamic bundle should be valid");
        macro_rules! tag {
            ($bit:expr, $tag:ident) => {
                if mask & (1 << $bit) != 0 {
                    assert!(world.insert(entity, $tag));
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
        format!("random_{component_count}_components_1_term/sky"),
        |bencher| {
            let (mut world, masks) =
                random_fragmented_component_world(component_count, RANDOM_FRAGMENT_ENTITY_COUNT);
            let expected = random_fragment_match_count(&masks, 1);
            let mut query = PreparedQuery::<&A>::new();
            assert_eq!(query.count(&world), expected);
            bencher.iter(|| {
                let mut checksum = 0_u64;
                query.for_each_chunk_with_entities(&mut world, |entities, values| {
                    for (&entity, a) in entities.iter().zip(values) {
                        checksum = add_random_fragment_component_1_checksum(
                            checksum,
                            generational_entity_key(entity.index(), entity.generation()),
                            a.0,
                        );
                    }
                });
                checksum
            });
        },
    );
}

fn bench_random_component_4(group: &mut BenchmarkGroup<'_, WallTime>, component_count: usize) {
    group.bench_function(
        format!("random_{component_count}_components_4_terms/sky"),
        |bencher| {
            let (mut world, masks) =
                random_fragmented_component_world(component_count, RANDOM_FRAGMENT_ENTITY_COUNT);
            let expected = random_fragment_match_count(&masks, 4);
            let mut query = PreparedQuery::<(&A, &B, &C, &D)>::new();
            assert_eq!(query.count(&world), expected);
            bencher.iter(|| {
                let mut checksum = 0_u64;
                query.for_each_chunk_with_entities(
                    &mut world,
                    |entities, (a_values, b_values, c_values, d_values)| {
                        for index in 0..entities.len() {
                            let entity = entities[index];
                            checksum = add_random_fragment_checksum(
                                checksum,
                                generational_entity_key(entity.index(), entity.generation()),
                                a_values[index].0,
                                b_values[index].0,
                                c_values[index].0,
                                d_values[index].0,
                            );
                        }
                    },
                );
                checksum
            });
        },
    );
}

fn bench_random_component_8(group: &mut BenchmarkGroup<'_, WallTime>, component_count: usize) {
    group.bench_function(
        format!("random_{component_count}_components_8_terms/sky"),
        |bencher| {
            let (mut world, masks) =
                random_fragmented_component_world(component_count, RANDOM_FRAGMENT_ENTITY_COUNT);
            let expected = random_fragment_match_count(&masks, 8);
            let mut query = PreparedQuery::<(&A, &B, &C, &D, &E, &F, &G, &H)>::new();
            assert_eq!(query.count(&world), expected);
            bencher.iter(|| {
                let mut checksum = 0_u64;
                query.for_each_chunk_with_entities(
                    &mut world,
                    |entities,
                     (
                        a_values,
                        b_values,
                        c_values,
                        d_values,
                        e_values,
                        f_values,
                        g_values,
                        h_values,
                    )| {
                        for index in 0..entities.len() {
                            let entity = entities[index];
                            checksum = add_random_fragment_component_8_checksum(
                                checksum,
                                generational_entity_key(entity.index(), entity.generation()),
                                a_values[index].0,
                                b_values[index].0,
                                c_values[index].0,
                                d_values[index].0,
                                e_values[index].0,
                                f_values[index].0,
                                g_values[index].0,
                                h_values[index].0,
                            );
                        }
                    },
                );
                checksum
            });
        },
    );
}

macro_rules! bench_random_tags {
    ($name:ident, $terms:literal, $query:ty) => {
        fn $name(group: &mut BenchmarkGroup<'_, WallTime>, component_count: usize) {
            let suffix = if $terms == 1 { "term" } else { "terms" };
            group.bench_function(
                format!("random_{component_count}_tags_{}_{suffix}/sky", $terms),
                |bencher| {
                    let (mut world, masks) =
                        random_fragmented_tag_world(component_count, RANDOM_FRAGMENT_ENTITY_COUNT);
                    let expected = random_fragment_match_count(&masks, $terms);
                    let mut query = PreparedQuery::<$query>::new();
                    assert_eq!(query.count(&world), expected);
                    bencher.iter(|| {
                        let mut checksum = 0_u64;
                        query.for_each_chunk_with_entities(&mut world, |entities, _| {
                            for &entity in entities {
                                checksum = checksum.wrapping_add(generational_entity_key(
                                    entity.index(),
                                    entity.generation(),
                                ));
                            }
                        });
                        checksum
                    });
                },
            );
        }
    };
}

bench_random_tags!(bench_random_tag_1, 1, &TagA);
bench_random_tags!(bench_random_tag_4, 4, (&TagA, &TagB, &TagC, &TagD));
bench_random_tags!(
    bench_random_tag_8,
    8,
    (&TagA, &TagB, &TagC, &TagD, &TagE, &TagF, &TagG, &TagH)
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
