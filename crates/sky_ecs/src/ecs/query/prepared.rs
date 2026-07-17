use super::filter::QueryFilter;
use super::parallel;
use super::param::QuerySpec;
use super::sequential::{self, SequentialChunkCache};
use super::{EntityId, PreparedCache, QueryDescriptor, QueryWorld, World};
use core::marker::PhantomData;

/// A typed, cached query over component data.
///
/// `PreparedQuery` is the explicit reusable-plan API for advanced hot paths.
/// Create one directly with [`PreparedQuery::new`]. Normal code can use the
/// world-bound [`Query`](super::Query) and [`QueryMut`](super::QueryMut)
/// facades instead. A prepared query caches matching archetypes and refreshes
/// automatically when the world's archetype set changes.
///
/// The type parameter `Q` determines which components are accessed
/// (e.g. `&Position`, `(&mut Position, &Velocity)`, `Option<&Health>`).
/// The optional `Flt` restricts matching to archetypes that satisfy
/// the filter (e.g. `With<Enemy>`, `Without<Dead>`).
///
/// Iteration methods accept `&World` for read-only queries and require
/// `&mut World` for queries that yield mutable component references.
pub struct PreparedQuery<Q, Flt = ()> {
    descriptor: QueryDescriptor,
    prepared: PreparedCache,
    parallel_jobs: parallel::ParallelJobCache,
    sequential_chunks: SequentialChunkCache,
    marker: PhantomData<fn() -> (Q, Flt)>,
}

impl<Q: QuerySpec, Flt: QueryFilter> Default for PreparedQuery<Q, Flt> {
    fn default() -> Self {
        Self {
            descriptor: Q::descriptor(),
            prepared: PreparedCache::default(),
            parallel_jobs: parallel::ParallelJobCache::default(),
            sequential_chunks: SequentialChunkCache::default(),
            marker: PhantomData,
        }
    }
}

impl<Q: QuerySpec, Flt: QueryFilter> PreparedQuery<Q, Flt> {
    /// Creates a new query plan with an initially empty cache.
    #[inline(always)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of archetypes currently matching this query.
    pub fn cached_archetype_count(&self) -> usize {
        self.prepared.cached_archetype_count()
    }

    #[inline(always)]
    fn prepare(&mut self, world: &World) {
        self.prepared.prepare::<Flt>(world, &self.descriptor);
    }

    /// Iterates *chunk-by-chunk*, providing full slices of each component
    /// column.
    ///
    /// This is the fastest iteration path — ideal for SIMD-style batch
    /// processing.
    #[inline(always)]
    pub fn for_each_chunk<W, F>(&mut self, world: W, mut f: F)
    where
        W: QueryWorld<Q>,
        F: for<'w> FnMut(Q::Chunk<'w>),
    {
        let world = world.as_world();
        self.prepare(world);
        if let Some(chunks) = self
            .sequential_chunks
            .get_or_prepare(&self.prepared.archetypes, world)
        {
            sequential::run_for_each_chunk::<Q, _>(chunks, f);
            return;
        }
        for cached in self.prepared.archetypes.iter() {
            let data = &world.data[cached.data_index];

            for chunk in &data.chunks {
                debug_assert!(chunk.entity_count != 0);
                unsafe {
                    f(Q::chunk_from_raw(chunk, &cached.component_indices));
                }
            }
        }
    }

    /// Iterates *chunk-by-chunk* in parallel using Rayon.
    ///
    /// The closure may run on multiple worker threads and chunk execution
    /// order is unspecified.  Do not perform structural changes, access
    /// [`World`], or mutate shared state without explicit synchronization.
    ///
    /// This API is intended for systems that first gather any resource data
    /// they need, then process component slices in parallel, and finally
    /// apply any queued structural changes after the parallel phase.
    ///
    /// ```rust
    /// use sky_ecs::World;
    ///
    /// #[derive(Clone, Copy)]
    /// struct Position {
    ///     x: f32,
    ///     y: f32,
    /// }
    ///
    /// #[derive(Clone, Copy)]
    /// struct Velocity {
    ///     x: f32,
    ///     y: f32,
    /// }
    ///
    /// let mut world = World::new();
    /// world.spawn((
    ///     Position { x: 0.0, y: 0.0 },
    ///     Velocity { x: 1.0, y: 2.0 },
    /// ));
    ///
    /// let mut query = sky_ecs::PreparedQuery::<(&mut Position, &Velocity)>::new();
    /// query.par_for_each_chunk(&mut world, |(positions, velocities)| {
    ///     for index in 0..positions.len() {
    ///         positions[index].x += velocities[index].x;
    ///         positions[index].y += velocities[index].y;
    ///     }
    /// });
    /// ```
    ///
    /// ```compile_fail
    /// use std::cell::Cell;
    /// use sky_ecs::World;
    ///
    /// let mut world = World::new();
    /// world.spawn((Cell::new(1u32),));
    ///
    /// let mut query = sky_ecs::PreparedQuery::<&Cell<u32>>::new();
    /// query.par_for_each_chunk(&mut world, |_cells| {});
    /// ```
    ///
    /// ```compile_fail
    /// use std::rc::Rc;
    /// use sky_ecs::World;
    ///
    /// let mut world = World::new();
    /// world.spawn((Rc::new(1u32),));
    ///
    /// let mut query = sky_ecs::PreparedQuery::<&mut Rc<u32>>::new();
    /// query.par_for_each_chunk(&mut world, |_values| {});
    /// ```
    pub fn par_for_each_chunk<W, F>(&mut self, world: W, f: F)
    where
        W: QueryWorld<Q>,
        for<'w> Q::Chunk<'w>: Send,
        F: for<'w> Fn(Q::Chunk<'w>) + Send + Sync,
    {
        let world = world.as_world();
        self.prepare(world);
        let jobs = parallel::prepare_job_snapshot(
            &mut self.parallel_jobs,
            &self.prepared.archetypes,
            world,
        );
        parallel::par_for_each_chunk::<Q, _>(&self.prepared.archetypes, world, jobs, f);
    }

    /// Iterates *entity-by-entity*, providing individual component
    /// references.
    #[inline(always)]
    pub fn for_each<W, F>(&mut self, world: W, mut f: F)
    where
        W: QueryWorld<Q>,
        F: for<'w> FnMut(Q::Item<'w>),
    {
        let world = world.as_world();
        self.prepare(world);
        if let Some(chunks) = self
            .sequential_chunks
            .get_or_prepare(&self.prepared.archetypes, world)
        {
            sequential::run_for_each::<Q, _>(chunks, f);
            return;
        }
        for cached in self.prepared.archetypes.iter() {
            let data = &world.data[cached.data_index];

            for chunk in &data.chunks {
                debug_assert!(chunk.entity_count != 0);
                unsafe {
                    Q::for_each_entity(chunk, &cached.component_indices, &mut f);
                }
            }
        }
    }

    /// Iterates matching entities in parallel using stripe-sized Rayon jobs.
    ///
    /// This is the ergonomic entity-level counterpart to
    /// [`par_for_each_chunk`](Self::par_for_each_chunk). Jobs are still split
    /// by contiguous storage ranges rather than scheduled once per entity.
    pub fn par_for_each<W, F>(&mut self, world: W, f: F)
    where
        W: QueryWorld<Q>,
        for<'w> Q::Item<'w>: Send,
        F: for<'w> Fn(Q::Item<'w>) + Send + Sync,
    {
        let world = world.as_world();
        self.prepare(world);
        let jobs = parallel::prepare_job_snapshot(
            &mut self.parallel_jobs,
            &self.prepared.archetypes,
            world,
        );
        parallel::par_for_each::<Q, _>(&self.prepared.archetypes, world, jobs, f);
    }

    /// Like [`for_each`](Self::for_each), but also provides the
    /// [`EntityId`] for each matched entity.
    pub fn for_each_with_entity<W, F>(&mut self, world: W, mut f: F)
    where
        W: QueryWorld<Q>,
        F: for<'w> FnMut(EntityId, Q::Item<'w>),
    {
        let world = world.as_world();
        self.prepare(world);
        if let Some(chunks) = self
            .sequential_chunks
            .get_or_prepare(&self.prepared.archetypes, world)
        {
            sequential::run_for_each_with_entity::<Q, _>(chunks, f);
            return;
        }
        for cached in self.prepared.archetypes.iter() {
            let data = &world.data[cached.data_index];

            for chunk in &data.chunks {
                let entities = chunk.entities();
                let mut entity_index = 0usize;
                unsafe {
                    Q::for_each_entity(chunk, &cached.component_indices, &mut |item| {
                        f(entities[entity_index], item);
                        entity_index += 1;
                    });
                }
            }
        }
    }

    /// Parallel counterpart to [`for_each_with_entity`](Self::for_each_with_entity).
    pub fn par_for_each_with_entity<W, F>(&mut self, world: W, f: F)
    where
        W: QueryWorld<Q>,
        for<'w> Q::Item<'w>: Send,
        F: for<'w> Fn(EntityId, Q::Item<'w>) + Send + Sync,
    {
        let world = world.as_world();
        self.prepare(world);
        let jobs = parallel::prepare_job_snapshot(
            &mut self.parallel_jobs,
            &self.prepared.archetypes,
            world,
        );
        parallel::par_for_each_with_entity::<Q, _>(&self.prepared.archetypes, world, jobs, f);
    }

    /// Like [`for_each_chunk`](Self::for_each_chunk), but also provides
    /// the entity ID slice for each chunk.
    pub fn for_each_chunk_with_entities<W, F>(&mut self, world: W, mut f: F)
    where
        W: QueryWorld<Q>,
        F: for<'w> FnMut(&[EntityId], Q::Chunk<'w>),
    {
        let world = world.as_world();
        self.prepare(world);
        if let Some(chunks) = self
            .sequential_chunks
            .get_or_prepare(&self.prepared.archetypes, world)
        {
            sequential::run_for_each_chunk_with_entities::<Q, _>(chunks, f);
            return;
        }
        for cached in self.prepared.archetypes.iter() {
            let data = &world.data[cached.data_index];

            for chunk in &data.chunks {
                debug_assert!(chunk.entity_count != 0);
                unsafe {
                    f(
                        chunk.entities(),
                        Q::chunk_from_raw(chunk, &cached.component_indices),
                    );
                }
            }
        }
    }

    /// Like [`par_for_each_chunk`](Self::par_for_each_chunk), but also
    /// provides the entity ID slice for each chunk.
    ///
    /// Entity slices line up with the component slices for that chunk, but
    /// chunk visitation order remains unspecified.
    pub fn par_for_each_chunk_with_entities<W, F>(&mut self, world: W, f: F)
    where
        W: QueryWorld<Q>,
        for<'w> Q::Chunk<'w>: Send,
        F: for<'w> Fn(&'w [EntityId], Q::Chunk<'w>) + Send + Sync,
    {
        let world = world.as_world();
        self.prepare(world);
        let jobs = parallel::prepare_job_snapshot(
            &mut self.parallel_jobs,
            &self.prepared.archetypes,
            world,
        );
        parallel::par_for_each_chunk_with_entities::<Q, _>(
            &self.prepared.archetypes,
            world,
            jobs,
            f,
        );
    }

    /// Returns the total number of entities matched by this query.
    pub fn count(&mut self, world: &World) -> usize {
        self.prepare(world);
        self.prepared
            .archetypes
            .iter()
            .map(|cached| {
                world.data[cached.data_index]
                    .chunks
                    .iter()
                    .map(|c| c.entity_count)
                    .sum::<usize>()
            })
            .sum()
    }

    /// Returns `true` if no entities match this query.
    pub fn is_empty(&mut self, world: &World) -> bool {
        self.prepare(world);
        !self.prepared.archetypes.iter().any(|cached| {
            world.data[cached.data_index]
                .chunks
                .iter()
                .any(|c| c.entity_count > 0)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::World;
    use super::super::{QueryFilter, With, Without};
    use super::PreparedQuery;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Clone, Copy, Default)]
    struct Position {
        x: f32,
        y: f32,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct MatchA(u8);
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct MatchB(u8);
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct MatchC(u8);
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct MatchD(u8);
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct MatchE(u8);
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct MatchF(u8);
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct MatchG(u8);
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct MatchH(u8);
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct MatchMissing;

    #[derive(Clone, Copy, Default)]
    struct Velocity {
        x: f32,
        y: f32,
    }

    #[derive(Clone, Copy, Default)]
    struct Extra {
        value: f32,
    }

    #[derive(Clone, Copy, Default)]
    struct Mass {
        value: f32,
    }

    struct CountingFilter;

    impl super::super::QueryFilterSealed for CountingFilter {}

    static FILTER_MATCH_CALLS: AtomicUsize = AtomicUsize::new(0);

    impl QueryFilter for CountingFilter {
        fn matches_archetype(_: &super::super::super::InternalArchetype) -> bool {
            FILTER_MATCH_CALLS.fetch_add(1, Ordering::Relaxed);
            true
        }
    }

    #[test]
    fn typed_single_component_query_reads_and_writes() {
        let mut world = World::new();
        world.spawn_batch((0..4).map(|_| (Velocity::default(),)));

        let mut init = PreparedQuery::<&mut Velocity>::new();
        let mut expected = 1.0;
        init.for_each(&mut world, |velocity| {
            velocity.x = expected;
            expected += 1.0;
        });

        let mut sum = 0.0;
        let mut read = PreparedQuery::<&Velocity>::new();
        read.for_each(&mut world, |velocity| {
            sum += velocity.x;
        });

        assert_eq!(sum, 10.0);
    }

    #[test]
    fn typed_two_component_query_updates_positions() {
        let mut world = World::new();
        world.spawn_batch((0..8).map(|_| (Position::default(), Velocity::default())));

        let mut init = PreparedQuery::<&mut Velocity>::new();
        init.for_each(&mut world, |velocity| {
            velocity.x = 1.0;
            velocity.y = 2.0;
        });

        let mut query = PreparedQuery::<(&mut Position, &Velocity)>::new();
        query.for_each_chunk(&mut world, |(positions, velocities)| {
            for index in 0..positions.len() {
                positions[index].x += velocities[index].x * 0.5;
                positions[index].y += velocities[index].y * 0.5;
            }
        });

        let mut check = PreparedQuery::<&Position>::new();
        check.for_each(&mut world, |position| {
            assert_eq!(position.x, 0.5);
            assert_eq!(position.y, 1.0);
        });
    }

    #[test]
    fn typed_four_component_query_runs_across_matching_archetypes() {
        let mut world = World::new();
        world.spawn_batch((0..6).map(|_| {
            (
                Position::default(),
                Velocity::default(),
                Extra::default(),
                Mass::default(),
            )
        }));

        let mut init = PreparedQuery::<(&mut Velocity, &mut Extra, &mut Mass)>::new();
        init.for_each(&mut world, |(velocity, extra, mass)| {
            velocity.x = 2.0;
            velocity.y = 4.0;
            extra.value = 1.0;
            mass.value = 3.0;
        });

        let mut query = PreparedQuery::<(&mut Position, &Velocity, &mut Extra, &Mass)>::new();
        query.for_each_chunk(&mut world, |(positions, velocities, extras, masses)| {
            for index in 0..positions.len() {
                positions[index].x += velocities[index].x * masses[index].value;
                positions[index].y += velocities[index].y * masses[index].value;
                extras[index].value += masses[index].value;
            }
        });

        let mut position_check = PreparedQuery::<&Position>::new();
        position_check.for_each(&mut world, |position| {
            assert_eq!(position.x, 6.0);
            assert_eq!(position.y, 12.0);
        });

        let mut extra_check = PreparedQuery::<&Extra>::new();
        extra_check.for_each(&mut world, |extra| {
            assert_eq!(extra.value, 4.0);
        });
    }

    #[test]
    fn required_posting_intersections_match_one_four_and_eight_components() {
        let mut world = World::new();
        world.spawn((MatchA(1),));
        world.spawn((MatchA(1), MatchB(2), MatchC(3), MatchD(4)));
        world.spawn((
            MatchA(1),
            MatchB(2),
            MatchC(3),
            MatchD(4),
            MatchE(5),
            MatchF(6),
            MatchG(7),
            MatchH(8),
        ));
        world.spawn((MatchB(20), MatchC(30), MatchD(40)));

        let mut one = PreparedQuery::<&MatchA>::new();
        assert_eq!(one.count(&world), 3);
        assert_eq!(one.cached_archetype_count(), 3);

        let mut four = PreparedQuery::<(&MatchA, &MatchB, &MatchC, &MatchD)>::new();
        assert_eq!(four.count(&world), 2);
        assert_eq!(four.cached_archetype_count(), 2);

        let mut eight = PreparedQuery::<(
            &MatchA,
            &MatchB,
            &MatchC,
            &MatchD,
            &MatchE,
            &MatchF,
            &MatchG,
            &MatchH,
        )>::new();
        assert_eq!(eight.count(&world), 1);
        eight.for_each(&world, |(a, b, c, d, e, f, g, h)| {
            assert_eq!(
                (a.0, b.0, c.0, d.0, e.0, f.0, g.0, h.0),
                (1, 2, 3, 4, 5, 6, 7, 8)
            );
        });
    }

    #[test]
    fn query_only_matches_archetypes_with_all_components() {
        let mut world = World::new();
        world.spawn_batch((0..2).map(|_| (Position::default(), Velocity::default())));
        world.spawn_batch((0..3).map(|_| (Position::default(),)));

        let mut init = PreparedQuery::<&mut Velocity>::new();
        init.for_each(&mut world, |velocity| {
            velocity.x = 1.0;
        });

        let mut query = PreparedQuery::<(&mut Position, &Velocity)>::new();
        query.for_each(&mut world, |(position, velocity)| {
            position.x += velocity.x;
        });

        let mut changed = 0;
        let mut unchanged = 0;
        let mut positions = PreparedQuery::<&Position>::new();
        positions.for_each(&mut world, |position| {
            if position.x == 1.0 {
                changed += 1;
            } else {
                unchanged += 1;
            }
        });

        assert_eq!(changed, 2);
        assert_eq!(unchanged, 3);
    }

    #[test]
    fn prepared_query_refreshes_when_new_matching_archetype_appears() {
        let mut world = World::new();
        world.spawn((Position::default(), Velocity::default()));

        let mut init = PreparedQuery::<&mut Velocity>::new();
        init.for_each(&mut world, |velocity| {
            velocity.x = 1.0;
        });

        let mut prepared = PreparedQuery::<(&mut Position, &Velocity)>::new();
        prepared.for_each(&mut world, |(position, velocity)| {
            position.x += velocity.x;
        });
        assert_eq!(prepared.cached_archetype_count(), 1);

        world.spawn((Position::default(), Velocity::default(), Extra::default()));
        let mut init_new = PreparedQuery::<(&mut Velocity, &mut Extra)>::new();
        init_new.for_each(&mut world, |(velocity, extra)| {
            velocity.x = 1.0;
            extra.value = 2.0;
        });

        prepared.for_each(&mut world, |(position, velocity)| {
            position.x += velocity.x;
        });
        assert_eq!(prepared.cached_archetype_count(), 2);

        let mut sum = 0.0;
        let mut positions = PreparedQuery::<&Position>::new();
        positions.for_each(&mut world, |position| {
            sum += position.x;
        });

        assert_eq!(sum, 3.0);
    }

    #[test]
    fn sequential_chunk_plan_is_built_after_layout_reuse() {
        let mut world = World::new();
        world.spawn((Position::default(),));
        world.spawn((Position::default(), MatchA(1)));
        world.spawn((Position::default(), MatchB(2)));
        world.spawn((Position::default(), MatchC(3)));
        world.spawn((Position::default(), MatchD(4)));
        world.spawn((Position::default(), MatchE(5)));
        world.spawn((Position::default(), MatchF(6)));
        world.spawn((Position::default(), MatchG(7)));

        let mut query = PreparedQuery::<&mut Position>::new();
        query.for_each(&mut world, |position| position.x += 1.0);
        assert_eq!(query.sequential_chunks.rebuild_count(), 0);

        query.for_each_chunk(&mut world, |positions| {
            for position in positions {
                position.x += 1.0;
            }
        });
        assert_eq!(query.sequential_chunks.rebuild_count(), 1);

        let mut entity_ids = std::collections::HashSet::new();
        query.for_each_with_entity(&mut world, |entity, position| {
            assert!(entity_ids.insert(entity));
            position.x += 1.0;
        });
        assert_eq!(query.sequential_chunks.rebuild_count(), 1);

        let mut chunk_entity_count = 0;
        query.for_each_chunk_with_entities(&mut world, |entities, positions| {
            assert_eq!(entities.len(), positions.len());
            chunk_entity_count += entities.len();
            for position in positions {
                position.x += 1.0;
            }
        });

        assert_eq!(entity_ids.len(), 8);
        assert_eq!(chunk_entity_count, 8);
        let mut seen = 0;
        let mut check = PreparedQuery::<&Position>::new();
        check.for_each(&world, |position| {
            assert_eq!(position.x, 4.0);
            seen += 1;
        });
        assert_eq!(seen, 8);
    }

    #[test]
    fn sequential_chunk_plan_keeps_dense_layout_on_direct_path() {
        let mut world = World::new();
        world.spawn_batch((0..1_024).map(|_| (Position::default(), Velocity::default())));

        let mut query = PreparedQuery::<(&mut Position, &Velocity)>::new();
        for _ in 0..3 {
            query.for_each_chunk(&mut world, |(positions, velocities)| {
                for index in 0..positions.len() {
                    positions[index].x += velocities[index].x;
                }
            });
        }

        assert_eq!(query.sequential_chunks.rebuild_count(), 0);
    }

    #[test]
    fn sequential_chunk_plan_rebuilds_after_structural_change() {
        let mut world = World::new();
        world.spawn((Position::default(),));
        world.spawn((Position::default(), MatchA(1)));
        world.spawn((Position::default(), MatchB(2)));
        world.spawn((Position::default(), MatchC(3)));
        world.spawn((Position::default(), MatchD(4)));
        world.spawn((Position::default(), MatchE(5)));
        world.spawn((Position::default(), MatchF(6)));
        world.spawn((Position::default(), MatchG(7)));

        let mut query = PreparedQuery::<&Position>::new();
        query.for_each(&world, |_| {});
        query.for_each(&world, |_| {});
        assert_eq!(query.sequential_chunks.rebuild_count(), 1);

        world.spawn((Position::default(), MatchH(8)));

        let mut seen = 0;
        query.for_each(&world, |_| seen += 1);
        assert_eq!(seen, 9);
        assert_eq!(query.sequential_chunks.rebuild_count(), 1);

        seen = 0;
        query.for_each(&world, |_| seen += 1);
        assert_eq!(seen, 9);
        assert_eq!(query.sequential_chunks.rebuild_count(), 2);
    }

    #[test]
    fn sequential_chunk_plan_never_reuses_pointers_across_worlds() {
        fn fragmented_world(x: f32) -> World {
            let mut world = World::new();
            world.spawn((Position { x, y: 0.0 },));
            world.spawn((Position { x, y: 0.0 }, MatchA(1)));
            world.spawn((Position { x, y: 0.0 }, MatchB(2)));
            world.spawn((Position { x, y: 0.0 }, MatchC(3)));
            world.spawn((Position { x, y: 0.0 }, MatchD(4)));
            world.spawn((Position { x, y: 0.0 }, MatchE(5)));
            world.spawn((Position { x, y: 0.0 }, MatchF(6)));
            world.spawn((Position { x, y: 0.0 }, MatchG(7)));
            world
        }

        let first = fragmented_world(1.0);
        let second = fragmented_world(2.0);
        let mut query = PreparedQuery::<&Position>::new();

        query.for_each(&first, |_| {});
        query.for_each(&first, |_| {});
        assert_eq!(query.sequential_chunks.rebuild_count(), 1);

        for expected_sum in [16.0, 16.0] {
            let mut sum = 0.0;
            query.for_each(&second, |position| sum += position.x);
            assert_eq!(sum, expected_sum);
        }
        assert_eq!(query.sequential_chunks.rebuild_count(), 2);
    }

    #[test]
    fn required_posting_refresh_finds_a_component_absent_from_the_initial_world() {
        let mut world = World::new();
        world.spawn((Position::default(),));

        let mut prepared = PreparedQuery::<(&Position, &Velocity)>::new();
        assert_eq!(prepared.count(&world), 0);
        assert_eq!(prepared.cached_archetype_count(), 0);

        world.spawn((Position::default(), Velocity::default()));
        assert_eq!(prepared.count(&world), 1);
        assert_eq!(prepared.cached_archetype_count(), 1);
    }

    #[test]
    fn prepared_query_scans_only_appended_archetypes_and_resets_after_clear() {
        let mut world = World::new();
        world.spawn((Position::default(),));

        FILTER_MATCH_CALLS.store(0, Ordering::Relaxed);
        let mut prepared = PreparedQuery::<&Position, CountingFilter>::new();
        assert_eq!(prepared.count(&world), 1);
        assert_eq!(FILTER_MATCH_CALLS.load(Ordering::Relaxed), 1);

        world.spawn((Position::default(), Velocity::default()));
        assert_eq!(prepared.count(&world), 2);
        assert_eq!(FILTER_MATCH_CALLS.load(Ordering::Relaxed), 2);

        world.clear();
        world.spawn((Position::default(),));
        world.spawn((Position::default(), Velocity::default()));
        let calls_before_reset = FILTER_MATCH_CALLS.load(Ordering::Relaxed);
        assert_eq!(prepared.count(&world), 2);
        assert_eq!(
            FILTER_MATCH_CALLS.load(Ordering::Relaxed) - calls_before_reset,
            2
        );
    }

    #[test]
    fn required_posting_query_rebuilds_after_world_clear() {
        let mut world = World::new();
        world.spawn((Position::default(),));
        world.spawn((Position::default(), Velocity::default()));

        let mut prepared = PreparedQuery::<&Position>::new();
        assert_eq!(prepared.count(&world), 2);
        assert_eq!(prepared.cached_archetype_count(), 2);

        world.clear();
        world.spawn((Velocity::default(),));
        world.spawn((Position::default(), Extra::default()));

        assert_eq!(prepared.count(&world), 1);
        assert_eq!(prepared.cached_archetype_count(), 1);
    }

    #[test]
    fn prepared_query_rebuilds_when_used_with_another_world_at_the_same_epoch() {
        let mut positions = World::new();
        positions.spawn((Position { x: 1.0, y: 2.0 },));

        let mut velocities = World::new();
        velocities.spawn((Velocity { x: 3.0, y: 4.0 },));

        let mut prepared = PreparedQuery::<&Position>::new();
        assert_eq!(prepared.count(&positions), 1);
        assert_eq!(prepared.cached_archetype_count(), 1);

        assert_eq!(prepared.count(&velocities), 0);
        assert_eq!(prepared.cached_archetype_count(), 0);

        assert_eq!(prepared.count(&positions), 1);
        assert_eq!(prepared.cached_archetype_count(), 1);
    }

    #[test]
    #[should_panic(expected = "duplicate component type")]
    fn typed_query_rejects_duplicate_types() {
        let mut world = World::new();
        world.spawn((Position::default(),));

        let mut query = PreparedQuery::<(&mut Position, &mut Position)>::new();
        query.for_each(&mut world, |_| {});
    }

    #[test]
    fn count_returns_matching_entity_total() {
        let mut world = World::new();
        world.spawn((Position { x: 0.0, y: 0.0 }, Velocity { x: 1.0, y: 1.0 }));
        world.spawn((Position { x: 0.0, y: 0.0 }, Velocity { x: 1.0, y: 1.0 }));
        world.spawn((Position { x: 0.0, y: 0.0 },)); // no Velocity

        let mut query = PreparedQuery::<(&Position, &Velocity)>::new();
        assert_eq!(query.count(&world), 2);

        let mut all_positions = PreparedQuery::<&Position>::new();
        assert_eq!(all_positions.count(&world), 3);
    }

    #[test]
    fn is_empty_reflects_matching_state() {
        let mut world = World::new();
        let mut query = PreparedQuery::<(&Position, &Velocity)>::new();
        assert!(query.is_empty(&world));

        world.spawn((Position { x: 0.0, y: 0.0 }, Velocity { x: 1.0, y: 1.0 }));
        assert!(!query.is_empty(&world));
    }

    #[test]
    fn for_each_with_entity_provides_correct_ids() {
        let mut world = World::new();
        let e1 = world.spawn((Position { x: 1.0, y: 0.0 },));
        let e2 = world.spawn((Position { x: 2.0, y: 0.0 },));

        let mut seen = std::collections::HashMap::new();
        let mut query = PreparedQuery::<&Position>::new();
        query.for_each_with_entity(&mut world, |entity, position| {
            seen.insert(entity, position.x);
        });

        assert_eq!(seen.len(), 2);
        assert_eq!(seen[&e1], 1.0);
        assert_eq!(seen[&e2], 2.0);
    }

    #[test]
    fn for_each_chunk_with_entities_provides_entity_slices() {
        let mut world = World::new();
        let e1 = world.spawn((Position { x: 1.0, y: 0.0 }, Velocity { x: 0.0, y: 0.0 }));
        let e2 = world.spawn((Position { x: 2.0, y: 0.0 }, Velocity { x: 0.0, y: 0.0 }));

        let mut entity_ids = Vec::new();
        let mut query = PreparedQuery::<(&Position, &Velocity)>::new();
        query.for_each_chunk_with_entities(&mut world, |entities, (positions, _velocities)| {
            assert_eq!(entities.len(), positions.len());
            entity_ids.extend_from_slice(entities);
        });

        assert!(entity_ids.contains(&e1));
        assert!(entity_ids.contains(&e2));
    }

    #[test]
    fn optional_query_returns_some_when_present_none_when_absent() {
        let mut world = World::new();
        world.spawn((Position { x: 1.0, y: 0.0 }, Velocity { x: 10.0, y: 0.0 }));
        world.spawn((Position { x: 2.0, y: 0.0 },)); // no Velocity

        let mut with_vel = 0;
        let mut without_vel = 0;
        let mut query = PreparedQuery::<(&Position, Option<&Velocity>)>::new();
        query.for_each(&mut world, |(pos, vel)| {
            if let Some(v) = vel {
                assert_eq!(pos.x, 1.0);
                assert_eq!(v.x, 10.0);
                with_vel += 1;
            } else {
                assert_eq!(pos.x, 2.0);
                without_vel += 1;
            }
        });

        assert_eq!(with_vel, 1);
        assert_eq!(without_vel, 1);
    }

    #[test]
    fn positive_filter_posting_drives_an_all_optional_query() {
        let mut world = World::new();
        world.spawn((Position { x: 1.0, y: 0.0 },));
        world.spawn((Velocity { x: 2.0, y: 0.0 },));

        let mut query =
            PreparedQuery::<(Option<&Position>, Option<&Velocity>), With<Velocity>>::new();
        assert_eq!(query.count(&world), 1);

        world.spawn((Position { x: 3.0, y: 0.0 }, Velocity { x: 4.0, y: 0.0 }));

        let mut sum = 0.0;
        query.for_each(&world, |(position, velocity)| {
            sum += position.map_or(0.0, |position| position.x);
            sum += velocity
                .expect("With<Velocity> guarantees the optional value")
                .x;
        });
        assert_eq!(query.cached_archetype_count(), 2);
        assert_eq!(sum, 9.0);
    }

    #[test]
    fn all_optional_query_without_a_positive_term_still_scans_every_archetype() {
        let mut world = World::new();
        world.spawn((Position { x: 1.0, y: 0.0 },));
        world.spawn((Velocity { x: 2.0, y: 0.0 },));
        world.spawn((Extra { value: 3.0 },));

        let mut query = PreparedQuery::<(Option<&Position>, Option<&Velocity>)>::new();
        assert_eq!(query.count(&world), 3);
        assert_eq!(query.cached_archetype_count(), 3);
    }

    #[test]
    fn optional_query_chunk_returns_some_slice_or_none() {
        let mut world = World::new();
        world.spawn((Position { x: 1.0, y: 0.0 }, Velocity { x: 5.0, y: 0.0 }));
        world.spawn((Position { x: 2.0, y: 0.0 },));

        let mut chunks_with_vel = 0;
        let mut chunks_without_vel = 0;
        let mut query = PreparedQuery::<(&Position, Option<&Velocity>)>::new();
        query.for_each_chunk(&mut world, |(positions, opt_velocities)| {
            assert!(!positions.is_empty());
            if opt_velocities.is_some() {
                chunks_with_vel += 1;
            } else {
                chunks_without_vel += 1;
            }
        });

        assert_eq!(chunks_with_vel, 1);
        assert_eq!(chunks_without_vel, 1);
    }

    #[test]
    fn optional_mut_query_writes_when_present() {
        let mut world = World::new();
        world.spawn((Position { x: 0.0, y: 0.0 }, Velocity { x: 1.0, y: 2.0 }));
        world.spawn((Position { x: 0.0, y: 0.0 },));

        let mut query = PreparedQuery::<(&mut Position, Option<&Velocity>)>::new();
        query.for_each(&mut world, |(pos, vel)| {
            if let Some(v) = vel {
                pos.x += v.x;
                pos.y += v.y;
            } else {
                pos.x = -1.0;
            }
        });

        let mut moved = 0;
        let mut stayed = 0;
        let mut check = PreparedQuery::<&Position>::new();
        check.for_each(&mut world, |pos| {
            if pos.x == 1.0 && pos.y == 2.0 {
                moved += 1;
            } else if pos.x == -1.0 {
                stayed += 1;
            }
        });

        assert_eq!(moved, 1);
        assert_eq!(stayed, 1);
    }

    #[test]
    fn with_filter_includes_only_matching_archetypes() {
        let mut world = World::new();
        world.spawn((Position { x: 1.0, y: 0.0 }, Velocity { x: 0.0, y: 0.0 }));
        world.spawn((Position { x: 2.0, y: 0.0 },)); // no Velocity

        let mut query = PreparedQuery::<&Position, With<Velocity>>::new();
        let mut count = 0;
        query.for_each(&mut world, |pos| {
            assert_eq!(pos.x, 1.0);
            count += 1;
        });

        assert_eq!(count, 1);
    }

    #[test]
    fn without_filter_excludes_matching_archetypes() {
        let mut world = World::new();
        world.spawn((Position { x: 1.0, y: 0.0 }, Velocity { x: 0.0, y: 0.0 }));
        world.spawn((Position { x: 2.0, y: 0.0 },));

        let mut query = PreparedQuery::<&Position, Without<Velocity>>::new();
        let mut count = 0;
        query.for_each(&mut world, |pos| {
            assert_eq!(pos.x, 2.0);
            count += 1;
        });

        assert_eq!(count, 1);
    }

    #[test]
    fn combined_filters_intersect() {
        let mut world = World::new();
        world.spawn((
            Position { x: 1.0, y: 0.0 },
            Velocity { x: 0.0, y: 0.0 },
            Extra { value: 0.0 },
        ));
        world.spawn((Position { x: 2.0, y: 0.0 }, Velocity { x: 0.0, y: 0.0 }));
        world.spawn((Position { x: 3.0, y: 0.0 },));

        // Want: has Velocity but NOT Extra
        let mut query = PreparedQuery::<&Position, (With<Velocity>, Without<Extra>)>::new();
        let mut count = 0;
        query.for_each(&mut world, |pos| {
            assert_eq!(pos.x, 2.0);
            count += 1;
        });

        assert_eq!(count, 1);
    }

    #[test]
    fn compiled_conjunctive_filters_simplify_duplicates_and_contradictions() {
        let mut world = World::new();
        world.spawn((Position { x: 1.0, y: 0.0 }, Velocity { x: 0.0, y: 0.0 }));
        world.spawn((Position { x: 2.0, y: 0.0 },));

        let mut redundant = PreparedQuery::<
            (&Position, &Velocity),
            (With<Position>, With<Velocity>, With<Velocity>),
        >::new();
        assert_eq!(redundant.count(&world), 1);

        let mut impossible = PreparedQuery::<&Position, (With<Velocity>, Without<Velocity>)>::new();
        assert_eq!(impossible.count(&world), 0);

        let mut required_but_excluded =
            PreparedQuery::<&Position, (With<Position>, Without<Position>)>::new();
        assert_eq!(required_but_excluded.count(&world), 0);
    }

    #[test]
    fn adaptive_match_preserves_declared_query_order() {
        let mut world = World::new();
        world.spawn((
            MatchA(1),
            MatchB(2),
            MatchC(3),
            MatchD(4),
            MatchE(5),
            MatchF(6),
            MatchG(7),
            MatchH(8),
        ));

        let mut query = PreparedQuery::<(
            &MatchH,
            &MatchB,
            &MatchF,
            &MatchA,
            &MatchG,
            &MatchC,
            &MatchE,
            &MatchD,
        )>::new();
        let mut seen = 0;
        query.for_each(&world, |(h, b, f, a, g, c, e, d)| {
            assert_eq!(
                (h.0, b.0, f.0, a.0, g.0, c.0, e.0, d.0),
                (8, 2, 6, 1, 7, 3, 5, 4)
            );
            seen += 1;
        });
        assert_eq!(seen, 1);
    }

    #[test]
    fn adaptive_match_handles_optional_gaps_and_required_misses() {
        let mut world = World::new();
        world.spawn((
            MatchA(1),
            MatchB(2),
            MatchC(3),
            MatchD(4),
            MatchE(5),
            MatchF(6),
            MatchG(7),
        ));

        let mut optional = PreparedQuery::<(
            &MatchG,
            Option<&MatchMissing>,
            &MatchA,
            &MatchF,
            &MatchB,
            &MatchE,
            &MatchC,
            &MatchD,
        )>::new();
        let mut seen = 0;
        optional.for_each(&world, |(g, missing, a, f, b, e, c, d)| {
            assert!(missing.is_none());
            assert_eq!((g.0, a.0, f.0, b.0, e.0, c.0, d.0), (7, 1, 6, 2, 5, 3, 4));
            seen += 1;
        });
        assert_eq!(seen, 1);

        let mut required = PreparedQuery::<(&MatchA, &MatchMissing)>::new();
        assert_eq!(required.count(&world), 0);
    }
}
