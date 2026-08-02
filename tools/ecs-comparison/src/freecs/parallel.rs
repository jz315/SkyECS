use super::*;
use crate::parallel::{
    bandwidth_kernel, compute_kernel, expected_position_checksum, parallel_bundle,
    position_checksum_value, ParallelRuntime, ParallelWorkload, ENTITIES_PER_FRAGMENT,
    FRAGMENT_SHAPES,
};

const PARALLEL_MASK: u64 = POSITION_MASK | VELOCITY_MASK | ROTATION_MASK | DATA_MASK;

struct ParallelContext {
    world: World,
    workload: ParallelWorkload,
}

impl ParallelContext {
    fn new(workload: ParallelWorkload) -> Self {
        let mut world = World::default();
        match workload {
            ParallelWorkload::FragmentedBandwidth => {
                for shape in 0..FRAGMENT_SHAPES {
                    let mut mask = PARALLEL_MASK;
                    if shape & (1 << 0) != 0 {
                        mask |= TAG_A_MASK;
                    }
                    if shape & (1 << 1) != 0 {
                        mask |= TAG_B_MASK;
                    }
                    if shape & (1 << 2) != 0 {
                        mask |= TAG_C_MASK;
                    }
                    if shape & (1 << 3) != 0 {
                        mask |= TAG_D_MASK;
                    }
                    if shape & (1 << 4) != 0 {
                        mask |= TAG_E_MASK;
                    }
                    if shape & (1 << 5) != 0 {
                        mask |= TAG_F_MASK;
                    }
                    let logical_start = shape * ENTITIES_PER_FRAGMENT;
                    let mut next = logical_start;
                    let _entities = world.spawn_batch(mask, ENTITIES_PER_FRAGMENT, |table, row| {
                        let (position, velocity, rotation, data) = parallel_bundle(next);
                        next += 1;
                        table.position[row] = position;
                        table.velocity[row] = velocity;
                        table.rotation[row] = rotation;
                        table.data[row] = data;
                    });
                }
            }
            _ => {
                let mut next = 0_usize;
                let _entities =
                    world.spawn_batch(PARALLEL_MASK, workload.entity_count(), |table, row| {
                        let (position, velocity, rotation, data) = parallel_bundle(next);
                        next += 1;
                        table.position[row] = position;
                        table.velocity[row] = velocity;
                        table.rotation[row] = rotation;
                        table.data[row] = data;
                    });
            }
        }
        Self { world, workload }
    }

    fn step(&mut self) {
        match self.workload {
            ParallelWorkload::DenseCompute => {
                self.world
                    .par_for_each_mut(PARALLEL_MASK, 0, |_entity, table, row| {
                        compute_kernel(
                            &mut table.position[row],
                            &table.velocity[row],
                            &table.rotation[row],
                            &table.data[row],
                        );
                    });
            }
            ParallelWorkload::DenseBandwidth | ParallelWorkload::FragmentedBandwidth => {
                self.world
                    .par_for_each_mut(MOVE_MASK, 0, |_entity, table, row| {
                        bandwidth_kernel(&mut table.position[row], &table.velocity[row]);
                    });
            }
        }
    }

    fn position_checksum(&self) -> u64 {
        let mut checksum = 0_u64;
        self.world
            .for_each(POSITION_MASK, 0, |_entity, table, row| {
                let position = &table.position[row];
                checksum = checksum.wrapping_add(position_checksum_value(position));
            });
        checksum
    }
}

pub fn bench_parallel_query(
    group: &mut BenchmarkGroup<'_, WallTime>,
    workload: ParallelWorkload,
    runtime: &ParallelRuntime,
) {
    group.bench_function(format!("threads_{}/freecs", runtime.threads()), |bencher| {
        let mut context = ParallelContext::new(workload);
        context.step();
        assert_eq!(
            context.position_checksum(),
            expected_position_checksum(workload),
            "FreeCS parallel adapter must update every matching entity exactly once"
        );
        bencher.iter(|| {
            context.step();
            black_box(&context);
        });
    });
}
