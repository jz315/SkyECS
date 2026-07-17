use super::*;

pub(super) fn random_fragmented_component_world(
    component_count: usize,
    entity_count: usize,
) -> (World, Vec<u16>) {
    let masks = random_fragment_masks_for(component_count, entity_count);
    let mut world = World::new();

    for &mask in &masks {
        let entity = world.add_entity(());
        macro_rules! component {
            ($bit:expr, $component:ident) => {
                if mask & (1 << $bit) != 0 {
                    world.add_component(entity, ($component(10.0),));
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
        let entity = world.add_entity(());
        macro_rules! tag {
            ($bit:expr, $tag:ident) => {
                if mask & (1 << $bit) != 0 {
                    world.add_component(entity, ($tag,));
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
        format!("random_{component_count}_components_1_term/shipyard"),
        |bencher| {
            let (world, masks) =
                random_fragmented_component_world(component_count, RANDOM_FRAGMENT_ENTITY_COUNT);
            let expected = random_fragment_match_count(&masks, 1);
            let a = world.borrow::<View<A>>().unwrap();
            assert_eq!((&a).iter().count(), expected);
            bencher.iter(|| {
                let mut checksum = 0_u64;
                (&a).iter().with_id().for_each(|(entity, a)| {
                    checksum =
                        add_random_fragment_component_1_checksum(checksum, entity.inner(), a.0);
                });
                checksum
            });
        },
    );
}

fn bench_random_component_4(group: &mut BenchmarkGroup<'_, WallTime>, component_count: usize) {
    group.bench_function(
        format!("random_{component_count}_components_4_terms/shipyard"),
        |bencher| {
            let (world, masks) =
                random_fragmented_component_world(component_count, RANDOM_FRAGMENT_ENTITY_COUNT);
            let expected = random_fragment_match_count(&masks, 4);
            let (a, b, c, d) = world
                .borrow::<(View<A>, View<B>, View<C>, View<D>)>()
                .unwrap();
            assert_eq!((&a, &b, &c, &d).iter().count(), expected);
            bencher.iter(|| {
                let mut checksum = 0_u64;
                (&a, &b, &c, &d)
                    .iter()
                    .with_id()
                    .for_each(|(entity, (a, b, c, d))| {
                        checksum = add_random_fragment_checksum(
                            checksum,
                            entity.inner(),
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
        format!("random_{component_count}_components_8_terms/shipyard"),
        |bencher| {
            let (world, masks) =
                random_fragmented_component_world(component_count, RANDOM_FRAGMENT_ENTITY_COUNT);
            let expected = random_fragment_match_count(&masks, 8);
            let (a, b, c, d, e, f, g, h) = world
                .borrow::<(
                    View<A>,
                    View<B>,
                    View<C>,
                    View<D>,
                    View<E>,
                    View<F>,
                    View<G>,
                    View<H>,
                )>()
                .unwrap();
            assert_eq!((&a, &b, &c, &d, &e, &f, &g, &h).iter().count(), expected);
            bencher.iter(|| {
                let mut checksum = 0_u64;
                (&a, &b, &c, &d, &e, &f, &g, &h).iter().with_id().for_each(
                    |(entity, (a, b, c, d, e, f, g, h))| {
                        checksum = add_random_fragment_component_8_checksum(
                            checksum,
                            entity.inner(),
                            a.0,
                            b.0,
                            c.0,
                            d.0,
                            e.0,
                            f.0,
                            g.0,
                            h.0,
                        );
                    },
                );
                checksum
            });
        },
    );
}

fn bench_random_tag_1(group: &mut BenchmarkGroup<'_, WallTime>, component_count: usize) {
    group.bench_function(
        format!("random_{component_count}_tags_1_term/shipyard"),
        |bencher| {
            let (world, masks) =
                random_fragmented_tag_world(component_count, RANDOM_FRAGMENT_ENTITY_COUNT);
            let expected = random_fragment_match_count(&masks, 1);
            let a = world.borrow::<View<TagA>>().unwrap();
            assert_eq!((&a).iter().count(), expected);
            bencher.iter(|| {
                let mut checksum = 0_u64;
                (&a).iter().with_id().for_each(|(entity, _)| {
                    checksum = checksum.wrapping_add(entity.inner());
                });
                checksum
            });
        },
    );
}

fn bench_random_tag_4(group: &mut BenchmarkGroup<'_, WallTime>, component_count: usize) {
    group.bench_function(
        format!("random_{component_count}_tags_4_terms/shipyard"),
        |bencher| {
            let (world, masks) =
                random_fragmented_tag_world(component_count, RANDOM_FRAGMENT_ENTITY_COUNT);
            let expected = random_fragment_match_count(&masks, 4);
            let (a, b, c, d) = world
                .borrow::<(View<TagA>, View<TagB>, View<TagC>, View<TagD>)>()
                .unwrap();
            assert_eq!((&a, &b, &c, &d).iter().count(), expected);
            bencher.iter(|| {
                let mut checksum = 0_u64;
                (&a, &b, &c, &d).iter().with_id().for_each(|(entity, _)| {
                    checksum = checksum.wrapping_add(entity.inner());
                });
                checksum
            });
        },
    );
}

fn bench_random_tag_8(group: &mut BenchmarkGroup<'_, WallTime>, component_count: usize) {
    group.bench_function(
        format!("random_{component_count}_tags_8_terms/shipyard"),
        |bencher| {
            let (world, masks) =
                random_fragmented_tag_world(component_count, RANDOM_FRAGMENT_ENTITY_COUNT);
            let expected = random_fragment_match_count(&masks, 8);
            let (a, b, c, d, e, f, g, h) = world
                .borrow::<(
                    View<TagA>,
                    View<TagB>,
                    View<TagC>,
                    View<TagD>,
                    View<TagE>,
                    View<TagF>,
                    View<TagG>,
                    View<TagH>,
                )>()
                .unwrap();
            assert_eq!((&a, &b, &c, &d, &e, &f, &g, &h).iter().count(), expected);
            bencher.iter(|| {
                let mut checksum = 0_u64;
                (&a, &b, &c, &d, &e, &f, &g, &h)
                    .iter()
                    .with_id()
                    .for_each(|(entity, _)| {
                        checksum = checksum.wrapping_add(entity.inner());
                    });
                checksum
            });
        },
    );
}

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
