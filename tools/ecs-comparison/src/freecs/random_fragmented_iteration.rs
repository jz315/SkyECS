use super::mixed_frame::warm_query;
use super::*;

pub(super) fn random_fragmented_component_world(
    component_count: usize,
    entity_count: usize,
) -> (World, Vec<u16>) {
    let masks = random_fragment_masks_for(component_count, entity_count);
    let mut world = World::default();

    for &mask in &masks {
        let entity = world
            .spawn_batch(0, 1, |_, _| {})
            .pop()
            .expect("empty entity spawn should return its ID");
        macro_rules! component {
            ($bit:expr, $setter:ident, $component:ident) => {
                if mask & (1 << $bit) != 0 {
                    world.$setter(entity, $component(10.0));
                }
            };
        }
        component!(0, set_fragment_a, FragmentA);
        component!(1, set_fragment_b, FragmentB);
        component!(2, set_fragment_c, FragmentC);
        component!(3, set_fragment_d, FragmentD);
        component!(4, set_fragment_e, FragmentE);
        component!(5, set_fragment_f, FragmentF);
        component!(6, set_fragment_g, FragmentG);
        component!(7, set_fragment_h, FragmentH);
        component!(8, set_fragment_i, FragmentI);
        component!(9, set_fragment_j, FragmentJ);
        component!(10, set_fragment_k, FragmentK);
        component!(11, set_fragment_l, FragmentL);
        component!(12, set_fragment_m, FragmentM);
        component!(13, set_fragment_n, FragmentN);
        component!(14, set_fragment_o, FragmentO);
        component!(15, set_fragment_p, FragmentP);
    }

    (world, masks)
}

pub(super) fn random_fragmented_tag_world(
    component_count: usize,
    entity_count: usize,
) -> (World, Vec<u16>) {
    let masks = random_fragment_masks_for(component_count, entity_count);
    let mut world = World::default();

    for &mask in &masks {
        let entity = world
            .spawn_batch(0, 1, |_, _| {})
            .pop()
            .expect("empty entity spawn should return its ID");
        macro_rules! tag {
            ($bit:expr, $setter:ident, $tag:ident) => {
                if mask & (1 << $bit) != 0 {
                    world.$setter(entity, $tag);
                }
            };
        }
        tag!(0, set_tag_a, TagA);
        tag!(1, set_tag_b, TagB);
        tag!(2, set_tag_c, TagC);
        tag!(3, set_tag_d, TagD);
        tag!(4, set_tag_e, TagE);
        tag!(5, set_tag_f, TagF);
        tag!(6, set_tag_g, TagG);
        tag!(7, set_tag_h, TagH);
        tag!(8, set_tag_i, TagI);
        tag!(9, set_tag_j, TagJ);
        tag!(10, set_tag_k, TagK);
        tag!(11, set_tag_l, TagL);
        tag!(12, set_tag_m, TagM);
        tag!(13, set_tag_n, TagN);
        tag!(14, set_tag_o, TagO);
        tag!(15, set_tag_p, TagP);
    }

    (world, masks)
}
fn bench_random_component_1(group: &mut BenchmarkGroup<'_, WallTime>, component_count: usize) {
    group.bench_function(
        format!("random_{component_count}_components_1_term/freecs"),
        |bencher| {
            let (mut world, masks) =
                random_fragmented_component_world(component_count, RANDOM_FRAGMENT_ENTITY_COUNT);
            let expected = random_fragment_match_count(&masks, 1);
            warm_query(&mut world, A_MASK);
            let mut initial_count = 0;
            world.for_each(A_MASK, 0, |_, _, _| initial_count += 1);
            assert_eq!(initial_count, expected);
            bencher.iter(|| {
                let mut checksum = 0_u64;
                world.for_each(A_MASK, 0, |entity, table, index| {
                    checksum = add_random_fragment_component_1_checksum(
                        checksum,
                        generational_entity_key(entity.id, entity.generation),
                        table.fragment_a[index].0,
                    );
                });
                checksum
            });
        },
    );
}

fn bench_random_component_4(group: &mut BenchmarkGroup<'_, WallTime>, component_count: usize) {
    let query_mask = A_MASK | B_MASK | C_MASK | D_MASK;
    group.bench_function(
        format!("random_{component_count}_components_4_terms/freecs"),
        |bencher| {
            let (mut world, masks) =
                random_fragmented_component_world(component_count, RANDOM_FRAGMENT_ENTITY_COUNT);
            let expected = random_fragment_match_count(&masks, 4);
            warm_query(&mut world, query_mask);
            let mut initial_count = 0;
            world.for_each(query_mask, 0, |_, _, _| initial_count += 1);
            assert_eq!(initial_count, expected);
            bencher.iter(|| {
                let mut checksum = 0_u64;
                world.for_each(query_mask, 0, |entity, table, index| {
                    checksum = add_random_fragment_checksum(
                        checksum,
                        generational_entity_key(entity.id, entity.generation),
                        table.fragment_a[index].0,
                        table.fragment_b[index].0,
                        table.fragment_c[index].0,
                        table.fragment_d[index].0,
                    );
                });
                checksum
            });
        },
    );
}

fn bench_random_component_8(group: &mut BenchmarkGroup<'_, WallTime>, component_count: usize) {
    let query_mask = A_MASK | B_MASK | C_MASK | D_MASK | E_MASK | F_MASK | G_MASK | H_MASK;
    group.bench_function(
        format!("random_{component_count}_components_8_terms/freecs"),
        |bencher| {
            let (mut world, masks) =
                random_fragmented_component_world(component_count, RANDOM_FRAGMENT_ENTITY_COUNT);
            let expected = random_fragment_match_count(&masks, 8);
            warm_query(&mut world, query_mask);
            let mut initial_count = 0;
            world.for_each(query_mask, 0, |_, _, _| initial_count += 1);
            assert_eq!(initial_count, expected);
            bencher.iter(|| {
                let mut checksum = 0_u64;
                world.for_each(query_mask, 0, |entity, table, index| {
                    checksum = add_random_fragment_component_8_checksum(
                        checksum,
                        generational_entity_key(entity.id, entity.generation),
                        table.fragment_a[index].0,
                        table.fragment_b[index].0,
                        table.fragment_c[index].0,
                        table.fragment_d[index].0,
                        table.fragment_e[index].0,
                        table.fragment_f[index].0,
                        table.fragment_g[index].0,
                        table.fragment_h[index].0,
                    );
                });
                checksum
            });
        },
    );
}

fn bench_random_tag(
    group: &mut BenchmarkGroup<'_, WallTime>,
    component_count: usize,
    term_count: usize,
) {
    let query_mask = match term_count {
        1 => TAG_A_MASK,
        4 => TAG_A_MASK | TAG_B_MASK | TAG_C_MASK | TAG_D_MASK,
        8 => {
            TAG_A_MASK
                | TAG_B_MASK
                | TAG_C_MASK
                | TAG_D_MASK
                | TAG_E_MASK
                | TAG_F_MASK
                | TAG_G_MASK
                | TAG_H_MASK
        }
        _ => unreachable!(),
    };
    let suffix = if term_count == 1 { "term" } else { "terms" };
    group.bench_function(
        format!("random_{component_count}_tags_{term_count}_{suffix}/freecs"),
        |bencher| {
            let (mut world, masks) =
                random_fragmented_tag_world(component_count, RANDOM_FRAGMENT_ENTITY_COUNT);
            let expected = random_fragment_match_count(&masks, term_count);
            warm_query(&mut world, query_mask);
            let mut initial_count = 0;
            world.for_each(query_mask, 0, |_, _, _| initial_count += 1);
            assert_eq!(initial_count, expected);
            bencher.iter(|| {
                let mut checksum = 0_u64;
                world.for_each(query_mask, 0, |entity, _, _| {
                    checksum = checksum
                        .wrapping_add(generational_entity_key(entity.id, entity.generation));
                });
                checksum
            });
        },
    );
}

pub fn bench_random_fragmented_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    // FreeCS 3.13.0 wires every new table against every existing table when
    // it builds archetype transition edges. The 16-bit matrix creates tens
    // of thousands of tables, making setup quadratic before Criterion can
    // start timing. Keep those six canonical cells explicitly unsupported
    // instead of changing their entity count or construction history.
    for (component_count, term_count) in RANDOM_FRAGMENT_WORKLOADS
        .into_iter()
        .filter(|(component_count, _)| *component_count < 16)
    {
        bench_random_tag(group, component_count, term_count);
    }
    for (component_count, term_count) in RANDOM_FRAGMENT_WORKLOADS
        .into_iter()
        .filter(|(component_count, _)| *component_count < 16)
    {
        match term_count {
            1 => bench_random_component_1(group, component_count),
            4 => bench_random_component_4(group, component_count),
            8 => bench_random_component_8(group, component_count),
            _ => unreachable!(),
        }
    }
}
