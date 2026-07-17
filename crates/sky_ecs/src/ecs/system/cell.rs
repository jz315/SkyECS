use crate::ecs::{CommandBuffer, World};
use std::marker::PhantomData;
use std::ptr::NonNull;

/// Scheduler-only capability to a structurally frozen World.
#[derive(Clone, Copy)]
pub(crate) struct UnsafeWorldCell<'w> {
    ptr: NonNull<World>,
    marker: PhantomData<&'w World>,
}

// Safety: this capability is created only for a compiled wave whose access
// sets are pairwise disjoint. Ordinary systems cannot mutate World structure.
unsafe impl Send for UnsafeWorldCell<'_> {}
unsafe impl Sync for UnsafeWorldCell<'_> {}

impl<'w> UnsafeWorldCell<'w> {
    /// # Safety
    ///
    /// `world` must point to a live World for `'w`. The scheduler must enforce
    /// the declared access sets for every capability derived from this cell.
    pub(crate) unsafe fn new(world: *mut World) -> Self {
        Self {
            ptr: NonNull::new(world).expect("scheduler world pointer must not be null"),
            marker: PhantomData,
        }
    }

    pub(crate) unsafe fn world(self) -> &'w World {
        // SAFETY: `new` requires a live World for `'w`; copying the capability
        // does not extend that lifetime or grant structural mutation.
        unsafe { self.ptr.as_ref() }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct SystemParamContext<'w> {
    commands: NonNull<CommandBuffer>,
    marker: PhantomData<&'w mut CommandBuffer>,
}

impl<'w> SystemParamContext<'w> {
    /// # Safety
    ///
    /// `commands` must point to the live, invocation-private command buffer
    /// that is exclusively available for `'w`.
    pub(crate) unsafe fn new(commands: *mut CommandBuffer) -> Self {
        Self {
            commands: NonNull::new(commands).expect("system command buffer must not be null"),
            marker: PhantomData,
        }
    }

    pub(crate) fn commands(self) -> *mut CommandBuffer {
        self.commands.as_ptr()
    }
}
