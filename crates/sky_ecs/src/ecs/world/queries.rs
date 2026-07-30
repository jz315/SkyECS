use super::*;

impl World {
    pub(crate) fn archetype_epoch(&self) -> usize {
        self.archetype_epoch
    }

    pub(crate) fn cache_token(&self) -> &Arc<()> {
        &self.cache_token
    }

    #[inline(always)]
    pub(crate) fn resolve_chunk(&self, chunk_id: ChunkId) -> &Chunk {
        let address = self
            .chunk_directory
            .resolve(chunk_id)
            .expect("cached chunk route must remain registered");
        &self.data[address.data_index].chunks[address.chunk_index]
    }

    pub(crate) fn query_snapshot<Q, Flt>(&self) -> Arc<Vec<CachedArchetype>>
    where
        Q: QuerySpec + 'static,
        Flt: QueryFilter + 'static,
    {
        self.query_cache.snapshot::<Q, Flt>(self)
    }

    pub(crate) fn query_parallel_snapshot<Q, Flt>(
        &self,
        archetypes: &[CachedArchetype],
    ) -> ParallelJobSnapshot
    where
        Q: QuerySpec + 'static,
        Flt: QueryFilter + 'static,
    {
        self.query_cache
            .parallel_snapshot::<Q, Flt>(self, archetypes)
    }

    #[cfg(test)]
    pub(crate) fn query_cache_len(&self) -> usize {
        self.query_cache.len()
    }

    #[cfg(test)]
    pub(crate) fn query_parallel_rebuild_count<Q, Flt>(&self) -> usize
    where
        Q: QuerySpec + 'static,
        Flt: QueryFilter + 'static,
    {
        self.query_cache.parallel_rebuild_count::<Q, Flt>()
    }

    /// Creates a read-only query bound to this world.
    ///
    /// The query parameter `Q` is usually a shared reference (`&T`), an
    /// optional shared reference (`Option<&T>`), or a tuple of those.
    /// Type-level filters can be attached with [`Query::filter`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use sky_ecs::World;
    /// # #[derive(Clone, Copy)] struct Pos { x: f32, y: f32 }
    /// # #[derive(Clone, Copy)] struct Vel { x: f32, y: f32 }
    /// # let mut world = World::new();
    /// # world.spawn((Pos { x: 0.0, y: 0.0 }, Vel { x: 1.0, y: 0.0 }));
    /// let query = world.query::<(&Pos, &Vel)>();
    /// query.for_each(|pos, vel| {
    ///     let _ = (pos.x, vel.x);
    /// });
    /// ```
    ///
    /// Mutable parameters use [`World::query_mut`] instead:
    ///
    /// ```compile_fail
    /// # use sky_ecs::World;
    /// # #[derive(Clone, Copy)] struct Pos { x: f32, y: f32 }
    /// # let world = World::new();
    /// let _query = world.query::<&mut Pos>();
    /// ```
    pub fn query<Q>(&self) -> Query<'_, Q>
    where
        Q: ReadOnlyQuerySpec + 'static,
    {
        Query::new(self)
    }

    /// Creates a query with exclusive access to this world.
    ///
    /// Mutable query parameters require this entry point. Read-only
    /// parameters may be mixed into the same query.
    ///
    /// ```
    /// # use sky_ecs::World;
    /// # #[derive(Clone, Copy)] struct Pos { x: f32, y: f32 }
    /// # #[derive(Clone, Copy)] struct Vel { x: f32, y: f32 }
    /// # let mut world = World::new();
    /// # world.spawn((Pos { x: 0.0, y: 0.0 }, Vel { x: 1.0, y: 0.0 }));
    /// let mut query = world.query_mut::<(&mut Pos, &Vel)>();
    /// query.for_each(|pos, vel| {
    ///     pos.x += vel.x;
    /// });
    /// ```
    pub fn query_mut<Q>(&mut self) -> QueryMut<'_, Q>
    where
        Q: QuerySpec + 'static,
    {
        QueryMut::new(self)
    }
}
