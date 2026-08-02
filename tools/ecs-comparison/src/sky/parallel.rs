use super::*;
use crate::parallel::{
    bandwidth_kernel, compute_kernel, expected_position_checksum, parallel_bundle,
    position_checksum_value, ParallelRuntime, ParallelWorkload, ENTITIES_PER_FRAGMENT,
    FRAGMENT_SHAPES,
};

enum ParallelContext {
    Bandwidth {
        world: World,
        query: PreparedQuery<(&'static mut PositionComponent, &'static VelocityComponent)>,
    },
    Compute {
        world: World,
        query: PreparedQuery<(
            &'static mut PositionComponent,
            &'static VelocityComponent,
            &'static RotationComponent,
            &'static DataComponent,
        )>,
    },
}

impl ParallelContext {
    fn new(workload: ParallelWorkload) -> Self {
        let mut world = World::new();
        match workload {
            ParallelWorkload::FragmentedBandwidth => {
                for shape in 0..FRAGMENT_SHAPES {
                    for row in 0..ENTITIES_PER_FRAGMENT {
                        let logical_index = shape * ENTITIES_PER_FRAGMENT + row;
                        let entity = world.spawn(parallel_bundle(logical_index));
                        if shape & (1 << 0) != 0 {
                            assert!(world.insert(entity, TagA));
                        }
                        if shape & (1 << 1) != 0 {
                            assert!(world.insert(entity, TagB));
                        }
                        if shape & (1 << 2) != 0 {
                            assert!(world.insert(entity, TagC));
                        }
                        if shape & (1 << 3) != 0 {
                            assert!(world.insert(entity, TagD));
                        }
                        if shape & (1 << 4) != 0 {
                            assert!(world.insert(entity, TagE));
                        }
                        if shape & (1 << 5) != 0 {
                            assert!(world.insert(entity, TagF));
                        }
                    }
                }
            }
            _ => world.spawn_batch((0..workload.entity_count()).map(parallel_bundle)),
        }

        match workload {
            ParallelWorkload::DenseCompute => Self::Compute {
                world,
                query: PreparedQuery::new(),
            },
            ParallelWorkload::DenseBandwidth | ParallelWorkload::FragmentedBandwidth => {
                Self::Bandwidth {
                    world,
                    query: PreparedQuery::new(),
                }
            }
        }
    }

    fn step(&mut self) {
        match self {
            Self::Bandwidth { world, query } => {
                query.par_for_each_chunk(world, |positions, velocities| {
                    for (position, velocity) in positions.iter_mut().zip(velocities) {
                        bandwidth_kernel(position, velocity);
                    }
                });
            }
            Self::Compute { world, query } => {
                query.par_for_each_chunk(world, |positions, velocities, rotations, data_values| {
                    for index in 0..positions.len() {
                        compute_kernel(
                            &mut positions[index],
                            &velocities[index],
                            &rotations[index],
                            &data_values[index],
                        );
                    }
                });
            }
        }
    }

    fn position_checksum(&mut self) -> u64 {
        let world = match self {
            Self::Bandwidth { world, .. } | Self::Compute { world, .. } => world,
        };
        let mut checksum = 0_u64;
        world
            .query::<&PositionComponent>()
            .for_each_chunk(|positions| {
                for position in positions {
                    checksum = checksum.wrapping_add(position_checksum_value(position));
                }
            });
        checksum
    }
}

pub fn bench_parallel_query(
    group: &mut BenchmarkGroup<'_, WallTime>,
    workload: ParallelWorkload,
    runtime: &ParallelRuntime,
) {
    group.bench_function(format!("threads_{}/sky", runtime.threads()), |bencher| {
        let mut context = ParallelContext::new(workload);
        context.step();
        assert_eq!(
            context.position_checksum(),
            expected_position_checksum(workload),
            "Sky parallel adapter must update every matching entity exactly once"
        );
        bencher.iter(|| {
            context.step();
            black_box(&context);
        });
    });
}
