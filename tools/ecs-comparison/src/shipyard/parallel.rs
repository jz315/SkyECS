use super::*;
use crate::parallel::{
    bandwidth_kernel, compute_kernel, expected_position_checksum, parallel_bundle,
    position_checksum_value, ParallelRuntime, ParallelWorkload, ENTITIES_PER_FRAGMENT,
    FRAGMENT_SHAPES,
};
use rayon::prelude::*;

struct ParallelContext {
    world: World,
    workload: ParallelWorkload,
}

impl ParallelContext {
    fn new(workload: ParallelWorkload) -> Self {
        let mut world = World::new();
        match workload {
            ParallelWorkload::FragmentedBandwidth => {
                for shape in 0..FRAGMENT_SHAPES {
                    for row in 0..ENTITIES_PER_FRAGMENT {
                        let logical_index = shape * ENTITIES_PER_FRAGMENT + row;
                        let entity = world.add_entity(parallel_bundle(logical_index));
                        if shape & (1 << 0) != 0 {
                            world.add_component(entity, TagA);
                        }
                        if shape & (1 << 1) != 0 {
                            world.add_component(entity, TagB);
                        }
                        if shape & (1 << 2) != 0 {
                            world.add_component(entity, TagC);
                        }
                        if shape & (1 << 3) != 0 {
                            world.add_component(entity, TagD);
                        }
                        if shape & (1 << 4) != 0 {
                            world.add_component(entity, TagE);
                        }
                        if shape & (1 << 5) != 0 {
                            world.add_component(entity, TagF);
                        }
                    }
                }
            }
            _ => {
                let _entities =
                    world.bulk_add_entity((0..workload.entity_count()).map(parallel_bundle));
            }
        }
        Self { world, workload }
    }

    fn step(&self) {
        match self.workload {
            ParallelWorkload::DenseCompute => {
                let (mut positions, velocities, rotations, data) = self
                    .world
                    .borrow::<(
                        ViewMut<PositionComponent>,
                        View<VelocityComponent>,
                        View<RotationComponent>,
                        View<DataComponent>,
                    )>()
                    .expect("Shipyard parallel component views should borrow");
                (&mut positions, &velocities, &rotations, &data)
                    .par_iter()
                    .for_each(|(position, velocity, rotation, data)| {
                        compute_kernel(position, velocity, rotation, data);
                    });
            }
            ParallelWorkload::DenseBandwidth | ParallelWorkload::FragmentedBandwidth => {
                let (mut positions, velocities) = self
                    .world
                    .borrow::<(ViewMut<PositionComponent>, View<VelocityComponent>)>()
                    .expect("Shipyard parallel component views should borrow");
                (&mut positions, &velocities)
                    .par_iter()
                    .for_each(|(position, velocity)| {
                        bandwidth_kernel(position, velocity);
                    });
            }
        }
    }

    fn position_checksum(&self) -> u64 {
        let positions = self
            .world
            .borrow::<View<PositionComponent>>()
            .expect("Shipyard position view should borrow");
        positions.iter().fold(0_u64, |checksum, position| {
            checksum.wrapping_add(position_checksum_value(position))
        })
    }
}

pub fn bench_parallel_query(
    group: &mut BenchmarkGroup<'_, WallTime>,
    workload: ParallelWorkload,
    runtime: &ParallelRuntime,
) {
    group.bench_function(
        format!("threads_{}/shipyard", runtime.threads()),
        |bencher| {
            let context = ParallelContext::new(workload);
            context.step();
            assert_eq!(
                context.position_checksum(),
                expected_position_checksum(workload),
                "Shipyard parallel adapter must update every matching entity exactly once"
            );
            bencher.iter(|| {
                context.step();
                black_box(&context);
            });
        },
    );
}
