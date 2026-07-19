use super::*;

impl World {
    /// Inserts a singleton resource, returning the previous value if one existed.
    ///
    /// [`Time`] is a permanent built-in resource and cannot be inserted or
    /// replaced.
    ///
    /// # Panics
    ///
    /// Panics if `R` is [`Time`].
    pub fn insert_resource<R: 'static>(&mut self, resource: R) -> Option<R> {
        assert_ne!(
            std::any::TypeId::of::<R>(),
            std::any::TypeId::of::<Time>(),
            "Time is permanent World frame state and cannot be inserted"
        );
        self.bump_resource_epoch();
        self.resources.insert(resource)
    }

    /// Returns an immutable reference to resource `R`, or `None`.
    ///
    /// [`Time`] is built in and is therefore always present.
    pub fn get_resource<R: 'static>(&self) -> Option<&R> {
        if std::any::TypeId::of::<R>() == std::any::TypeId::of::<Time>() {
            // Safety: equal TypeIds prove that `R` is exactly `Time`.
            return Some(unsafe { &*std::ptr::from_ref(&self.time).cast::<R>() });
        }
        self.resources.get::<R>()
    }

    /// Returns a mutable reference to resource `R`, or `None`.
    ///
    /// Exclusive code may use this to configure built-in [`Time`] between
    /// ticks. Ordinary systems must use `Res<Time>` instead of `ResMut<Time>`.
    pub fn get_resource_mut<R: 'static>(&mut self) -> Option<&mut R> {
        if std::any::TypeId::of::<R>() == std::any::TypeId::of::<Time>() {
            // Safety: equal TypeIds prove that `R` is exactly `Time`.
            return Some(unsafe { &mut *std::ptr::from_mut(&mut self.time).cast::<R>() });
        }
        self.resources.get_mut::<R>()
    }

    /// Returns `true` if the world contains resource `R`.
    ///
    /// This always returns `true` for the permanent built-in [`Time`].
    pub fn contains_resource<R: 'static>(&self) -> bool {
        if std::any::TypeId::of::<R>() == std::any::TypeId::of::<Time>() {
            return true;
        }
        self.resources.contains::<R>()
    }

    pub(crate) fn contains_resource_id(&self, id: std::any::TypeId) -> bool {
        id == std::any::TypeId::of::<Time>() || self.resources.contains_id(id)
    }

    /// Removes and returns resource `R`, or `None` if not present.
    ///
    /// [`Time`] is a permanent built-in resource and cannot be removed.
    ///
    /// # Panics
    ///
    /// Panics if `R` is [`Time`].
    pub fn remove_resource<R: 'static>(&mut self) -> Option<R> {
        assert_ne!(
            std::any::TypeId::of::<R>(),
            std::any::TypeId::of::<Time>(),
            "Time is permanent World frame state and cannot be removed"
        );
        if !self.resources.contains::<R>() {
            return None;
        }
        self.bump_resource_epoch();
        self.resources.remove::<R>()
    }

    pub(crate) fn resource_epoch(&self) -> u64 {
        self.resource_epoch
    }

    /// Returns a stable resource pointer while `resource_epoch` is unchanged.
    /// Scheduler access validation determines whether it may be dereferenced as
    /// shared or exclusive during a prepared system wave.
    pub(crate) fn resource_ptr<R: 'static>(&self) -> Option<*mut R> {
        if std::any::TypeId::of::<R>() == std::any::TypeId::of::<Time>() {
            return Some(std::ptr::from_ref(&self.time).cast::<R>().cast_mut());
        }
        self.resources
            .get::<R>()
            .map(|resource| std::ptr::from_ref(resource).cast_mut())
    }
}
