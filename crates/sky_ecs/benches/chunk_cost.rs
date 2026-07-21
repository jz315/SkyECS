use criterion::{criterion_group, criterion_main, Criterion};
use sky_ecs::{PreparedQuery, World};
use std::hint::black_box;
use std::time::Duration;

const DENSE_ENTITY_COUNT: usize = 1_000_000;
const FRAGMENTED_ARCHETYPE_COUNT: usize = 26;
const ENTITIES_PER_FRAGMENTED_ARCHETYPE: usize = 400;

// These four components have the same sizes and alignments as the comparison
// suite's Matrix4 + three Vector3 components. Chunk capacities therefore match
// the real dense-iteration workload without adding a benchmark-only dependency.
#[derive(Clone, Copy)]
#[allow(dead_code)]
struct Transform([f32; 16]);

#[derive(Clone, Copy)]
struct Position([f32; 3]);

#[derive(Clone, Copy)]
#[allow(dead_code)]
struct Rotation([f32; 3]);

#[derive(Clone, Copy)]
struct Velocity([f32; 3]);

#[derive(Clone, Copy)]
struct Data(f32);

macro_rules! define_tags {
    ($($tag:ident),+ $(,)?) => {
        $(
            #[derive(Clone, Copy, Default)]
            #[allow(dead_code)]
            struct $tag(f32);
        )+
    };
}

define_tags!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,);

fn dense_bundle() -> (Transform, Position, Rotation, Velocity) {
    (
        Transform([
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]),
        Position([0.0, 0.0, 0.0]),
        Rotation([0.0, 0.0, 0.0]),
        Velocity([1.0, 2.0, 3.0]),
    )
}

fn dense_world() -> World {
    let mut world = World::new();
    world.spawn_batch((0..DENSE_ENTITY_COUNT).map(|_| dense_bundle()));
    world
}

fn dense_world_in_ten_batches() -> World {
    let mut world = World::new();
    for _ in 0..10 {
        world.spawn_batch((0..DENSE_ENTITY_COUNT / 10).map(|_| dense_bundle()));
    }
    world
}

fn fragmented_world() -> World {
    let mut world = World::new();

    macro_rules! spawn_archetypes {
        ($($tag:ty),+ $(,)?) => {
            $(
                for _ in 0..ENTITIES_PER_FRAGMENTED_ARCHETYPE {
                    world.spawn((<$tag>::default(), Data(1.0)));
                }
            )+
        };
    }

    spawn_archetypes!(
        A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    );
    world
}

#[inline(always)]
fn update_dense(positions: &mut [Position], velocities: &[Velocity]) {
    for (position, velocity) in positions.iter_mut().zip(velocities) {
        position.0[0] += velocity.0[0];
        position.0[1] += velocity.0[1];
        position.0[2] += velocity.0[2];
    }
}

#[inline(never)]
fn update_dense_fn(positions: &mut [Position], velocities: &[Velocity]) {
    update_dense(positions, velocities);
}

#[inline(always)]
fn update_fragmented(data: &mut [Data]) {
    for value in data {
        value.0 = -value.0;
    }
}

fn dense_chunk_lengths(world: &World) -> Vec<usize> {
    let mut query = PreparedQuery::<(&Position, &Velocity)>::new();
    let mut lengths = Vec::new();
    query.for_each_chunk(world, |(positions, velocities)| {
        assert_eq!(positions.len(), velocities.len());
        lengths.push(positions.len());
    });
    assert_eq!(lengths.iter().sum::<usize>(), DENSE_ENTITY_COUNT);
    lengths
}

fn fragmented_chunk_lengths() -> Vec<usize> {
    let world = fragmented_world();
    let mut query = PreparedQuery::<&Data>::new();
    let mut lengths = Vec::new();
    query.for_each_chunk(&world, |data| lengths.push(data.len()));
    assert_eq!(
        lengths.iter().sum::<usize>(),
        FRAGMENTED_ARCHETYPE_COUNT * ENTITIES_PER_FRAGMENTED_ARCHETYPE
    );
    lengths
}

fn print_layout(label: &str, lengths: &[usize]) {
    let smallest = lengths.iter().copied().min().unwrap_or(0);
    let largest = lengths.iter().copied().max().unwrap_or(0);
    eprintln!(
        "{label}: {} chunks, {} rows, chunk rows {}..={}",
        lengths.len(),
        lengths.iter().sum::<usize>(),
        smallest,
        largest
    );
}

#[derive(Clone, Copy)]
struct ChunkDescriptor {
    first_column: usize,
    second_column: usize,
    rows: usize,
}

#[inline(always)]
fn include_descriptor(checksum: &mut usize, descriptor: ChunkDescriptor) {
    *checksum = checksum.rotate_left(7)
        ^ descriptor.first_column
        ^ descriptor.second_column.rotate_left(13)
        ^ descriptor.rows;
}

#[inline(always)]
fn walk_descriptors(descriptors: &[ChunkDescriptor]) -> usize {
    let mut checksum = 0;
    for &descriptor in descriptors {
        include_descriptor(&mut checksum, descriptor);
    }
    checksum
}

fn bench_dense(c: &mut Criterion) {
    let chunk_lengths = dense_chunk_lengths(&dense_world());
    let ten_batch_chunk_lengths = dense_chunk_lengths(&dense_world_in_ten_batches());
    print_layout("dense_1m_one_batch", &chunk_lengths);
    print_layout("dense_1m_ten_batches", &ten_batch_chunk_lengths);

    let mut group = c.benchmark_group("chunk_cost/dense_1m");

    group.bench_function("flat_compute", |b| {
        let mut positions = vec![Position([0.0; 3]); DENSE_ENTITY_COUNT];
        let velocities = vec![Velocity([1.0, 2.0, 3.0]); DENSE_ENTITY_COUNT];
        b.iter(|| {
            update_dense(&mut positions, &velocities);
            black_box(&positions);
        });
    });

    group.bench_function("segmented_compute", |b| {
        let mut positions = vec![Position([0.0; 3]); DENSE_ENTITY_COUNT];
        let velocities = vec![Velocity([1.0, 2.0, 3.0]); DENSE_ENTITY_COUNT];
        b.iter(|| {
            let mut start = 0;
            for &length in &chunk_lengths {
                let end = start + length;
                update_dense(&mut positions[start..end], &velocities[start..end]);
                start = end;
            }
            black_box(&positions);
        });
    });

    group.bench_function("descriptor_walk", |b| {
        let mut world = dense_world();
        let mut query = PreparedQuery::<(&mut Position, &Velocity)>::new();
        let mut descriptors = Vec::new();
        query.for_each_chunk(&mut world, |(positions, velocities)| {
            descriptors.push(ChunkDescriptor {
                first_column: positions.as_ptr() as usize,
                second_column: velocities.as_ptr() as usize,
                rows: positions.len(),
            });
        });
        assert_eq!(descriptors.len(), chunk_lengths.len());
        b.iter(|| black_box(walk_descriptors(&descriptors)));
    });

    group.bench_function("prepared_dispatch", |b| {
        let mut world = dense_world();
        let mut query = PreparedQuery::<(&mut Position, &Velocity)>::new();
        assert_eq!(query.count(&world), DENSE_ENTITY_COUNT);
        b.iter(|| {
            let mut checksum = 0;
            query.for_each_chunk(&mut world, |(positions, velocities)| {
                include_descriptor(
                    &mut checksum,
                    ChunkDescriptor {
                        first_column: positions.as_ptr() as usize,
                        second_column: velocities.as_ptr() as usize,
                        rows: positions.len(),
                    },
                );
            });
            black_box(checksum);
        });
    });

    group.bench_function("full_ecs", |b| {
        let mut world = dense_world();
        let mut query = PreparedQuery::<(&mut Position, &Velocity)>::new();
        assert_eq!(query.count(&world), DENSE_ENTITY_COUNT);
        b.iter(|| {
            query.for_each_chunk(&mut world, |(positions, velocities)| {
                update_dense(positions, velocities);
            });
            black_box(&world);
        });
    });

    group.bench_function("full_ecs_fn", |b| {
        let mut world = dense_world();
        let mut query = PreparedQuery::<(&mut Position, &Velocity)>::new();
        assert_eq!(query.count(&world), DENSE_ENTITY_COUNT);
        b.iter(|| {
            query.for_each_chunk_fn(&mut world, update_dense_fn);
            black_box(&world);
        });
    });

    group.bench_function("full_ecs_fn_24_chunks", |b| {
        let mut world = dense_world_in_ten_batches();
        let mut query = PreparedQuery::<(&mut Position, &Velocity)>::new();
        assert_eq!(query.count(&world), DENSE_ENTITY_COUNT);
        b.iter(|| {
            query.for_each_chunk_fn(&mut world, update_dense_fn);
            black_box(&world);
        });
    });

    group.finish();
}

fn bench_fragmented(c: &mut Criterion) {
    let entity_count = FRAGMENTED_ARCHETYPE_COUNT * ENTITIES_PER_FRAGMENTED_ARCHETYPE;
    let chunk_lengths = fragmented_chunk_lengths();
    let previous_layout_lengths =
        vec![ENTITIES_PER_FRAGMENTED_ARCHETYPE; FRAGMENTED_ARCHETYPE_COUNT];
    print_layout("fragmented_26x400", &chunk_lengths);

    let mut group = c.benchmark_group("chunk_cost/fragmented_26x400");

    group.bench_function("flat_compute", |b| {
        let mut data = vec![Data(1.0); entity_count];
        b.iter(|| {
            update_fragmented(&mut data);
            black_box(&data);
        });
    });

    group.bench_function("previous_16k_layout_compute", |b| {
        let mut data = vec![Data(1.0); entity_count];
        b.iter(|| {
            let mut start = 0;
            for &length in &previous_layout_lengths {
                let end = start + length;
                update_fragmented(&mut data[start..end]);
                start = end;
            }
            black_box(&data);
        });
    });

    group.bench_function("segmented_compute", |b| {
        let mut data = vec![Data(1.0); entity_count];
        b.iter(|| {
            let mut start = 0;
            for &length in &chunk_lengths {
                let end = start + length;
                update_fragmented(&mut data[start..end]);
                start = end;
            }
            black_box(&data);
        });
    });

    group.bench_function("descriptor_walk", |b| {
        let mut world = fragmented_world();
        let mut query = PreparedQuery::<&mut Data>::new();
        let mut descriptors = Vec::new();
        query.for_each_chunk(&mut world, |data| {
            descriptors.push(ChunkDescriptor {
                first_column: data.as_ptr() as usize,
                second_column: 0,
                rows: data.len(),
            });
        });
        assert_eq!(descriptors.len(), chunk_lengths.len());
        b.iter(|| black_box(walk_descriptors(&descriptors)));
    });

    group.bench_function("prepared_dispatch", |b| {
        let mut world = fragmented_world();
        let mut query = PreparedQuery::<&mut Data>::new();
        assert_eq!(query.count(&world), entity_count);
        b.iter(|| {
            let mut checksum = 0;
            query.for_each_chunk(&mut world, |data| {
                include_descriptor(
                    &mut checksum,
                    ChunkDescriptor {
                        first_column: data.as_ptr() as usize,
                        second_column: 0,
                        rows: data.len(),
                    },
                );
            });
            black_box(checksum);
        });
    });

    group.bench_function("full_ecs", |b| {
        let mut world = fragmented_world();
        let mut query = PreparedQuery::<&mut Data>::new();
        assert_eq!(query.count(&world), entity_count);
        b.iter(|| {
            query.for_each_chunk(&mut world, update_fragmented);
            black_box(&world);
        });
    });

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(5))
        .sample_size(100);
    targets = bench_dense, bench_fragmented
}
criterion_main!(benches);
