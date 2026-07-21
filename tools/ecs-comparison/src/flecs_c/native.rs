use std::ffi::c_void;
use std::ptr::NonNull;

#[link(name = "sky_flecs_c_native", kind = "static", modifiers = "-bundle")]
unsafe extern "C" {
    pub(super) fn sky_flecs_c_insert_new() -> *mut c_void;
    pub(super) fn sky_flecs_c_insert_delete(context: *mut c_void);
    pub(super) fn sky_flecs_c_bulk_insert(context: *mut c_void) -> u64;
    pub(super) fn sky_flecs_c_single_insert(context: *mut c_void) -> u64;
    pub(super) fn sky_flecs_c_simple_new(count: usize) -> *mut c_void;
    pub(super) fn sky_flecs_c_simple_delete(context: *mut c_void);
    pub(super) fn sky_flecs_c_simple_run(context: *mut c_void, repetitions: usize) -> u64;
    pub(super) fn sky_flecs_c_fragmented_new() -> *mut c_void;
    pub(super) fn sky_flecs_c_fragmented_delete(context: *mut c_void);
    pub(super) fn sky_flecs_c_fragmented_run(context: *mut c_void) -> u64;
    pub(super) fn sky_flecs_c_random_fragmented_new(
        storage: usize,
        component_count: usize,
        term_count: usize,
        masks: *const u16,
        entity_count: usize,
    ) -> *mut c_void;
    pub(super) fn sky_flecs_c_random_fragmented_delete(context: *mut c_void);
    pub(super) fn sky_flecs_c_random_fragmented_run(context: *mut c_void) -> u64;
    pub(super) fn sky_flecs_c_random_fragmented_count(context: *mut c_void) -> u64;
    pub(super) fn sky_flecs_c_heavy_new() -> *mut c_void;
    pub(super) fn sky_flecs_c_heavy_delete(context: *mut c_void);
    pub(super) fn sky_flecs_c_heavy_run(context: *mut c_void) -> u64;
    pub(super) fn sky_flecs_c_random_new(count: usize) -> *mut c_void;
    pub(super) fn sky_flecs_c_random_delete(context: *mut c_void);
    pub(super) fn sky_flecs_c_random_run(context: *mut c_void) -> u64;
    pub(super) fn sky_flecs_c_entity_ops_new() -> *mut c_void;
    pub(super) fn sky_flecs_c_entity_ops_delete(context: *mut c_void);
    pub(super) fn sky_flecs_c_spawn_despawn(context: *mut c_void) -> u64;
    pub(super) fn sky_flecs_c_add_remove_new() -> *mut c_void;
    pub(super) fn sky_flecs_c_add_remove_delete(context: *mut c_void);
    pub(super) fn sky_flecs_c_add_remove(context: *mut c_void) -> u64;
    pub(super) fn sky_flecs_c_mixed_new() -> *mut c_void;
    pub(super) fn sky_flecs_c_mixed_delete(context: *mut c_void);
    pub(super) fn sky_flecs_c_mixed_frame(context: *mut c_void) -> u64;
    pub(super) fn sky_flecs_c_mixed_movement(context: *mut c_void) -> u64;
    pub(super) fn sky_flecs_c_mixed_health(context: *mut c_void) -> u64;
    pub(super) fn sky_flecs_c_mixed_heavy(context: *mut c_void) -> u64;
    pub(super) fn sky_flecs_c_mixed_random(context: *mut c_void) -> u64;
    pub(super) fn sky_flecs_c_mixed_churn(context: *mut c_void) -> u64;
    pub(super) fn sky_flecs_c_mixed_spawn(context: *mut c_void) -> u64;
    pub(super) fn sky_flecs_c_gameplay_new() -> *mut c_void;
    pub(super) fn sky_flecs_c_gameplay_delete(context: *mut c_void);
    pub(super) fn sky_flecs_c_gameplay_frame(context: *mut c_void) -> u64;
    pub(super) fn sky_flecs_c_gameplay_run_trace(
        context: *mut c_void,
        digest: *mut NativeGameplayDigest,
    ) -> bool;
    pub(super) fn sky_flecs_c_validate() -> bool;
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct NativeGameplayDigest {
    pub entity_count: u64,
    pub moving_count: u64,
    pub health_count: u64,
    pub lifetime_count: u64,
    pub stunned_count: u64,
    pub position_checksum: u64,
    pub health_checksum: u64,
    pub lifetime_checksum: u64,
    pub generation_checksum: u64,
    pub ai_lookup_checksum: u64,
}

pub(super) struct Context {
    pointer: NonNull<c_void>,
    delete: unsafe extern "C" fn(*mut c_void),
}

impl Context {
    pub(super) fn new(pointer: *mut c_void, delete: unsafe extern "C" fn(*mut c_void)) -> Self {
        Self {
            pointer: NonNull::new(pointer).expect("Flecs C API context allocation failed"),
            delete,
        }
    }

    pub(super) fn pointer(&mut self) -> *mut c_void {
        self.pointer.as_ptr()
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // SAFETY: The pointer came from the constructor paired with this deleter.
        unsafe { (self.delete)(self.pointer.as_ptr()) };
    }
}
