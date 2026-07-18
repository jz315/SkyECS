//! Full production-path regression benchmark for first-time query preparation.
//!
//! The World reproduces Compare-ECS's 16-tag random-fragmentation history.
//! Every timed iteration creates a fresh `PreparedQuery` and calls `count`, so
//! matching, column-map materialization, cache allocation, and entity counting
//! are all included. World construction remains outside the timed loop.

use criterion::{criterion_group, criterion_main, Criterion};
use sky_ecs::dynamic::{DynamicBundle, WorldDynamicExt};
use sky_ecs::{PreparedQuery, World};
use std::hint::black_box;
use std::time::Duration;

const ENTITY_COUNT: usize = 65_536;

macro_rules! tags {
    ($($name:ident),+ $(,)?) => {
        $(#[derive(Clone, Copy)] struct $name;)+
    };
}

tags!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

fn random_fragment_masks() -> Vec<u16> {
    let mut state = 0x243F_6A88_85A3_08D3_u64;
    (0..ENTITY_COUNT)
        .map(|_| {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut value = state;
            value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            (value ^ (value >> 31)) as u16
        })
        .collect()
}

fn random_fragmented_world() -> (World, Vec<u16>) {
    let masks = random_fragment_masks();
    let mut world = World::new();
    for &mask in &masks {
        let entity = world.spawn_dynamic(DynamicBundle::new()).unwrap();
        macro_rules! insert_tag {
            ($bit:expr, $tag:ident) => {
                if mask & (1 << $bit) != 0 {
                    assert!(world.insert(entity, $tag));
                }
            };
        }
        insert_tag!(0, A);
        insert_tag!(1, B);
        insert_tag!(2, C);
        insert_tag!(3, D);
        insert_tag!(4, E);
        insert_tag!(5, F);
        insert_tag!(6, G);
        insert_tag!(7, H);
        insert_tag!(8, I);
        insert_tag!(9, J);
        insert_tag!(10, K);
        insert_tag!(11, L);
        insert_tag!(12, M);
        insert_tag!(13, N);
        insert_tag!(14, O);
        insert_tag!(15, P);
    }
    (world, masks)
}

fn expected_count(masks: &[u16], term_count: usize) -> usize {
    let query_mask = (1_u16 << term_count) - 1;
    masks
        .iter()
        .filter(|&&mask| mask & query_mask == query_mask)
        .count()
}

fn bench_fragmented_prepare(c: &mut Criterion) {
    let (world, masks) = random_fragmented_world();
    assert_eq!(world.archetype_count(), 50_908);

    let mut group = c.benchmark_group("query_prepare_fragmented");
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(3));
    group.sample_size(50);

    let expected = expected_count(&masks, 1);
    group.bench_function("random_16_tags_1_term", |b| {
        b.iter(|| {
            let mut query = PreparedQuery::<&A>::new();
            let count = query.count(&world);
            debug_assert_eq!(count, expected);
            black_box(count)
        });
    });

    let expected = expected_count(&masks, 4);
    group.bench_function("random_16_tags_4_terms", |b| {
        b.iter(|| {
            let mut query = PreparedQuery::<(&A, &B, &C, &D)>::new();
            let count = query.count(&world);
            debug_assert_eq!(count, expected);
            black_box(count)
        });
    });

    let expected = expected_count(&masks, 8);
    group.bench_function("random_16_tags_8_terms", |b| {
        b.iter(|| {
            let mut query = PreparedQuery::<(&A, &B, &C, &D, &E, &F, &G, &H)>::new();
            let count = query.count(&world);
            debug_assert_eq!(count, expected);
            black_box(count)
        });
    });
    group.finish();
}

criterion_group!(benches, bench_fragmented_prepare);
criterion_main!(benches);
