use super::*;

fn discard_commands_after_panic(schedule: &mut Schedule) {
    let discard = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        schedule.discard_commands();
    }));
    if let Err(payload) = discard {
        // Never let a user payload's destructor replace the primary schedule
        // panic or turn cleanup into a double-panic abort.
        std::mem::forget(payload);
    }
}

impl World {
    fn run_schedule(
        &mut self,
        frame_delta: f32,
        raw_delta: f32,
    ) -> Result<TickReport, ScheduleError> {
        let mut schedule = self.schedule.take().expect("cannot call tick recursively");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            schedule.run(self, frame_delta, raw_delta)
        }));

        match result {
            Ok(result) => {
                self.schedule = Some(schedule);
                result
            }
            Err(payload) => {
                discard_commands_after_panic(&mut schedule);
                self.poison_after_schedule_panic();
                self.schedule = Some(schedule);
                std::panic::resume_unwind(payload);
            }
        }
    }

    /// Install an engine module plugin into this world.
    ///
    /// Plugin constructors are the configuration surface; installing a plugin
    /// records only the fact that the plugin type is present.  The app runner
    /// discovers capabilities from the resources/plugins that were installed
    /// rather than enforcing a fixed required set.
    pub fn install<P>(&mut self, plugin: P) -> PluginResult
    where
        P: Plugin + 'static,
    {
        let name = plugin.name();
        if let Some(existing) = self.plugins.get::<P>() {
            return Err(PluginError::new(
                name,
                format!("{existing} is already installed"),
            ));
        }
        plugin.install(self)?;
        if let Err(existing) = self.plugins.insert::<P>(name) {
            return Err(PluginError::new(
                name,
                format!("{existing} was installed during plugin setup"),
            ));
        }
        Ok(())
    }

    /// Returns `true` if plugin type `P` was installed through [`World::install`].
    #[inline]
    pub fn has_plugin<P: 'static>(&self) -> bool {
        self.plugins.contains::<P>()
    }

    /// Require plugin type `P` to have been installed.
    ///
    /// Plugins should use this for local hard dependencies.  The app runner
    /// does not use it to impose a universal set of required capabilities.
    pub fn require_plugin<P: 'static>(&self, plugin: &'static str) -> PluginResult {
        if self.has_plugin::<P>() {
            Ok(())
        } else {
            Err(PluginError::new(
                plugin,
                format!("requires {}", std::any::type_name::<P>()),
            ))
        }
    }

    // -----------------------------------------------------------------------
    // Schedule API
    // -----------------------------------------------------------------------

    /// Returns the builder for an installed typed schedule stage.
    ///
    /// # Panics
    ///
    /// Panics when a custom stage has not been installed explicitly with
    /// [`World::insert_stage_after`]. Use [`World::try_stage`] when the stage
    /// may be optional.
    pub fn stage<L: StageLabel>(&mut self, label: L) -> StageBuilder<'_> {
        self.try_stage(label)
            .unwrap_or_else(|error| panic!("{error}"))
    }

    /// Tries to return the builder for an already-installed typed stage.
    pub fn try_stage<L: StageLabel>(
        &mut self,
        _label: L,
    ) -> Result<StageBuilder<'_>, ScheduleBuildError> {
        let schedule = self
            .schedule
            .as_mut()
            .expect("cannot modify schedule during tick");
        let index = schedule
            .stage_index::<L>()
            .ok_or(ScheduleBuildError::UnknownStage(L::name()))?;
        Ok(StageBuilder::new(schedule, index))
    }

    /// Inserts a custom typed stage after an installed stage.
    ///
    /// The returned builder configures the newly installed stage directly.
    /// Repeated insertions after the same anchor preserve call order. A stage
    /// inserted under an earlier sibling stays with that sibling's subtree.
    pub fn insert_stage_after<Anchor, L>(
        &mut self,
        _anchor: Anchor,
        _label: L,
    ) -> Result<StageBuilder<'_>, ScheduleBuildError>
    where
        Anchor: StageLabel,
        L: StageLabel,
    {
        let schedule = self
            .schedule
            .as_mut()
            .expect("cannot modify schedule during tick");
        let index = schedule.insert_after::<Anchor, L>()?;
        Ok(StageBuilder::new(schedule, index))
    }

    /// Returns an owned snapshot of the compiled schedule and access graph.
    ///
    /// This does not initialize or run systems. It may compile dirty stage
    /// waves, so it requires exclusive access to the World.
    pub fn schedule_diagnostics(&mut self) -> ScheduleDiagnostics {
        self.schedule
            .as_mut()
            .expect("cannot inspect schedule during tick")
            .diagnostics()
    }

    /// Advances the world by one frame using wall-clock time.
    ///
    /// On the first call, delta is 0.  Subsequent calls measure elapsed time
    /// since the previous tick.  The measured delta is stored in
    /// [`Time::raw_delta`], while [`Time::frame_delta`] is affected by
    /// [`Time::time_scale`].
    ///
    /// # Panics
    ///
    /// Panics if the World is poisoned, if called recursively (e.g. from
    /// within a running system), or if a running system removes a resource
    /// required later in the same frame.
    /// Missing resources detected before the frame begins are returned as an
    /// error without advancing time or running any system.
    pub fn tick(&mut self) -> Result<TickReport, ScheduleError> {
        self.assert_schedule_tick_allowed();
        let now = std::time::Instant::now();
        let raw_delta = match self
            .schedule
            .as_ref()
            .expect("cannot call tick recursively")
            .last_tick
        {
            Some(last) => (now - last).as_secs_f32(),
            None => 0.0,
        };
        let result = self.run_schedule(raw_delta, raw_delta);
        if result.is_ok() {
            self.schedule.as_mut().unwrap().last_tick = Some(now);
        }
        result
    }

    /// Advances the world by the given delta (in seconds).
    ///
    /// Useful for deterministic tests and fixed-step simulations.  The given
    /// delta is treated as both raw and clamped frame time.
    ///
    /// # Panics
    ///
    /// Panics if the World is poisoned, if called recursively, or if a running
    /// system removes a resource required later in the same frame. Missing resources found by frame
    /// preflight are returned without advancing time or running systems.
    pub fn tick_with_delta(&mut self, delta: f32) -> Result<TickReport, ScheduleError> {
        self.tick_with_frame_delta(delta, delta)
    }

    /// Advances the world by a clamped frame delta and the raw real delta.
    ///
    /// App runners should use this when they clamp large frame deltas before
    /// simulation but still want [`Time::raw_delta`] and
    /// [`Time::raw_elapsed`] to reflect real elapsed time.
    ///
    /// # Panics
    ///
    /// Panics if the World is poisoned, if called recursively, or if a running
    /// system removes a resource required later in the same frame. Missing resources found by frame
    /// preflight are returned without advancing time or running systems.
    pub fn tick_with_frame_delta(
        &mut self,
        frame_delta: f32,
        raw_delta: f32,
    ) -> Result<TickReport, ScheduleError> {
        self.assert_schedule_tick_allowed();
        self.run_schedule(frame_delta, raw_delta)
    }

    /// Tears down all initialised systems in reverse order.
    ///
    /// Call this before dropping the world if your systems need a clean
    /// shutdown (e.g. flushing buffers, releasing external resources).
    pub fn shutdown(&mut self) {
        let mut schedule = self.schedule.take().expect("cannot shutdown during tick");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            schedule.shutdown(self);
        }));
        self.schedule = Some(schedule);
        if let Err(payload) = result {
            self.poison_after_schedule_panic();
            std::panic::resume_unwind(payload);
        }
    }
}
