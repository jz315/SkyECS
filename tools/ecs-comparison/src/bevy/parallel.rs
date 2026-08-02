use super::*;
use crate::parallel::{
    bandwidth_kernel, compute_kernel, expected_position_checksum, parallel_bundle,
    position_checksum_value, ParallelRuntime, ParallelWorkload, ENTITIES_PER_FRAGMENT,
    FRAGMENT_SHAPES,
};
use bevy_ecs::query::QueryState;

enum ParallelContext {
    Bandwidth {
        world: World,
        query: QueryState<(&'static mut PositionComponent, &'static VelocityComponent)>,
    },
    Compute {
        world: World,
        query: QueryState<(
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
                        let mut entity = world.spawn(parallel_bundle(logical_index));
                        if shape & (1 << 0) != 0 {
                            entity.insert(TagA);
                        }
                        if shape & (1 << 1) != 0 {
                            entity.insert(TagB);
                        }
                        if shape & (1 << 2) != 0 {
                            entity.insert(TagC);
                        }
                        if shape & (1 << 3) != 0 {
                            entity.insert(TagD);
                        }
                        if shape & (1 << 4) != 0 {
                            entity.insert(TagE);
                        }
                        if shape & (1 << 5) != 0 {
                            entity.insert(TagF);
                        }
                    }
                }
            }
            _ => {
                world.spawn_batch((0..workload.entity_count()).map(parallel_bundle));
            }
        }

        match workload {
            ParallelWorkload::DenseCompute => {
                let query = world.query();
                Self::Compute { world, query }
            }
            ParallelWorkload::DenseBandwidth | ParallelWorkload::FragmentedBandwidth => {
                let query = world.query();
                Self::Bandwidth { world, query }
            }
        }
    }

    fn step(&mut self) {
        match self {
            Self::Bandwidth { world, query } => {
                query
                    .par_iter_mut(world)
                    .for_each(|(mut position, velocity)| {
                        bandwidth_kernel(&mut position, velocity);
                    });
            }
            Self::Compute { world, query } => {
                query
                    .par_iter_mut(world)
                    .for_each(|(mut position, velocity, rotation, data)| {
                        compute_kernel(&mut position, velocity, rotation, data);
                    });
            }
        }
    }

    fn position_checksum(&mut self) -> u64 {
        let world = match self {
            Self::Bandwidth { world, .. } | Self::Compute { world, .. } => world,
        };
        world
            .query::<&PositionComponent>()
            .iter(world)
            .fold(0_u64, |checksum, position| {
                checksum.wrapping_add(position_checksum_value(position))
            })
    }
}

pub fn bench_parallel_query(
    group: &mut BenchmarkGroup<'_, WallTime>,
    workload: ParallelWorkload,
    runtime: &ParallelRuntime,
) {
    group.bench_function(format!("threads_{}/bevy", runtime.threads()), |bencher| {
        let mut context = ParallelContext::new(workload);
        context.step();
        assert_eq!(
            context.position_checksum(),
            expected_position_checksum(workload),
            "Bevy parallel adapter must update every matching entity exactly once"
        );
        bencher.iter(|| {
            context.step();
            black_box(&context);
        });
    });
}
