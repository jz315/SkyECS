use crate::common::{
    random_fragment_masks, random_fragment_match_count, COLD_RANDOM_ENTITY_COUNT,
    LARGE_ITERATION_ENTITY_COUNT, RANDOM_FRAGMENT_COMPONENT_COUNTS, RANDOM_FRAGMENT_ENTITY_COUNT,
    REPEATED_ITERATION_COUNT, SIMPLE_ENTITY_COUNT, WARM_RANDOM_ENTITY_COUNT,
};
use criterion::{measurement::WallTime, BatchSize, BenchmarkGroup};
use libloading::Library;
use std::ffi::c_void;
use std::hint::black_box;
use std::ptr::NonNull;
use std::sync::OnceLock;

macro_rules! native_api {
    ($(fn $name:ident($($argument:ident: $argument_type:ty),*) -> $return_type:ty;)*) => {
        struct NativeApi {
            _library: Library,
            $(
                $name: unsafe extern "C" fn($($argument_type),*) -> $return_type,
            )*
        }

        impl NativeApi {
            fn load() -> Self {
                // SAFETY: The build script creates this library from the matching
                // C++ source and exports every symbol declared by this macro. The
                // library stays owned by NativeApi for as long as any pointer is used.
                unsafe {
                    let library = Library::new(env!("SKY_FLECS_CPP_NATIVE_PATH"))
                        .expect("failed to load the isolated Flecs C++ benchmark library");
                    $(
                        let $name = *library
                            .get::<unsafe extern "C" fn($($argument_type),*) -> $return_type>(
                                concat!(stringify!($name), "\0").as_bytes(),
                            )
                            .unwrap_or_else(|error| {
                                panic!("failed to load {}: {error}", stringify!($name))
                            });
                    )*
                    Self {
                        _library: library,
                        $($name,)*
                    }
                }
            }
        }

        static NATIVE_API: OnceLock<NativeApi> = OnceLock::new();

        fn native_api() -> &'static NativeApi {
            NATIVE_API.get_or_init(NativeApi::load)
        }

        $(
            unsafe extern "C" fn $name($($argument: $argument_type),*) -> $return_type {
                // SAFETY: Callers uphold the contract of the matching native function.
                unsafe { (native_api().$name)($($argument),*) }
            }
        )*
    };
}

native_api! {
    fn sky_flecs_cpp_insert_new() -> *mut c_void;
    fn sky_flecs_cpp_insert_delete(context: *mut c_void) -> ();
    fn sky_flecs_cpp_bulk_insert(context: *mut c_void) -> u64;
    fn sky_flecs_cpp_single_insert(context: *mut c_void) -> u64;
    fn sky_flecs_cpp_simple_new(count: usize) -> *mut c_void;
    fn sky_flecs_cpp_simple_delete(context: *mut c_void) -> ();
    fn sky_flecs_cpp_simple_run(context: *mut c_void, repetitions: usize) -> u64;
    fn sky_flecs_cpp_fragmented_new() -> *mut c_void;
    fn sky_flecs_cpp_fragmented_delete(context: *mut c_void) -> ();
    fn sky_flecs_cpp_fragmented_run(context: *mut c_void) -> u64;
    fn sky_flecs_cpp_random_fragmented_new(component_count: usize, entity_count: usize) -> *mut c_void;
    fn sky_flecs_cpp_random_fragmented_delete(context: *mut c_void) -> ();
    fn sky_flecs_cpp_random_fragmented_run(context: *mut c_void) -> u64;
    fn sky_flecs_cpp_random_fragmented_count(context: *mut c_void) -> u64;
    fn sky_flecs_cpp_heavy_new() -> *mut c_void;
    fn sky_flecs_cpp_heavy_delete(context: *mut c_void) -> ();
    fn sky_flecs_cpp_heavy_run(context: *mut c_void) -> u64;
    fn sky_flecs_cpp_random_new(count: usize) -> *mut c_void;
    fn sky_flecs_cpp_random_delete(context: *mut c_void) -> ();
    fn sky_flecs_cpp_random_run(context: *mut c_void) -> u64;
    fn sky_flecs_cpp_entity_ops_new() -> *mut c_void;
    fn sky_flecs_cpp_entity_ops_delete(context: *mut c_void) -> ();
    fn sky_flecs_cpp_spawn_despawn(context: *mut c_void) -> u64;
    fn sky_flecs_cpp_add_remove_new() -> *mut c_void;
    fn sky_flecs_cpp_add_remove_delete(context: *mut c_void) -> ();
    fn sky_flecs_cpp_add_remove(context: *mut c_void) -> u64;
    fn sky_flecs_cpp_mixed_new() -> *mut c_void;
    fn sky_flecs_cpp_mixed_delete(context: *mut c_void) -> ();
    fn sky_flecs_cpp_mixed_frame(context: *mut c_void) -> u64;
    fn sky_flecs_cpp_mixed_movement(context: *mut c_void) -> u64;
    fn sky_flecs_cpp_mixed_health(context: *mut c_void) -> u64;
    fn sky_flecs_cpp_mixed_heavy(context: *mut c_void) -> u64;
    fn sky_flecs_cpp_mixed_random(context: *mut c_void) -> u64;
    fn sky_flecs_cpp_mixed_churn(context: *mut c_void) -> u64;
    fn sky_flecs_cpp_mixed_spawn(context: *mut c_void) -> u64;
    fn sky_flecs_cpp_validate() -> bool;
}

struct Context {
    pointer: NonNull<c_void>,
    delete: unsafe extern "C" fn(*mut c_void),
}

impl Context {
    fn new(pointer: *mut c_void, delete: unsafe extern "C" fn(*mut c_void)) -> Self {
        Self {
            pointer: NonNull::new(pointer).expect("Flecs C++ context allocation failed"),
            delete,
        }
    }

    fn pointer(&mut self) -> *mut c_void {
        self.pointer.as_ptr()
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        // SAFETY: `pointer` was created by the matching C++ constructor and is
        // owned by this context. Drop runs exactly once and uses its paired deleter.
        unsafe { (self.delete)(self.pointer.as_ptr()) };
    }
}

fn insert_context() -> Context {
    // SAFETY: The constructor has no preconditions and returns an owning pointer.
    unsafe { Context::new(sky_flecs_cpp_insert_new(), sky_flecs_cpp_insert_delete) }
}

fn simple_context(count: usize) -> Context {
    // SAFETY: `count` is a finite benchmark size and the returned pointer is owning.
    unsafe { Context::new(sky_flecs_cpp_simple_new(count), sky_flecs_cpp_simple_delete) }
}

fn fragmented_context() -> Context {
    // SAFETY: The constructor has no preconditions and returns an owning pointer.
    unsafe {
        Context::new(
            sky_flecs_cpp_fragmented_new(),
            sky_flecs_cpp_fragmented_delete,
        )
    }
}

fn random_fragmented_context(component_count: usize) -> Context {
    // SAFETY: All callers pass a supported component count and the pointer is owning.
    unsafe {
        Context::new(
            sky_flecs_cpp_random_fragmented_new(component_count, RANDOM_FRAGMENT_ENTITY_COUNT),
            sky_flecs_cpp_random_fragmented_delete,
        )
    }
}

fn heavy_context() -> Context {
    // SAFETY: The constructor has no preconditions and returns an owning pointer.
    unsafe { Context::new(sky_flecs_cpp_heavy_new(), sky_flecs_cpp_heavy_delete) }
}

fn random_context(count: usize) -> Context {
    // SAFETY: `count` is non-zero for all registered workloads.
    unsafe { Context::new(sky_flecs_cpp_random_new(count), sky_flecs_cpp_random_delete) }
}

fn entity_ops_context() -> Context {
    // SAFETY: The constructor has no preconditions and returns an owning pointer.
    unsafe {
        Context::new(
            sky_flecs_cpp_entity_ops_new(),
            sky_flecs_cpp_entity_ops_delete,
        )
    }
}

fn add_remove_context() -> Context {
    // SAFETY: The constructor has no preconditions and returns an owning pointer.
    unsafe {
        Context::new(
            sky_flecs_cpp_add_remove_new(),
            sky_flecs_cpp_add_remove_delete,
        )
    }
}

fn mixed_context() -> Context {
    // SAFETY: The constructor has no preconditions and returns an owning pointer.
    unsafe { Context::new(sky_flecs_cpp_mixed_new(), sky_flecs_cpp_mixed_delete) }
}

pub fn bench_insert(group: &mut BenchmarkGroup<'_, WallTime>) {
    group.bench_function("bulk_insert_10k/flecs_cpp", |bencher| {
        bencher.iter_batched_ref(
            insert_context,
            |context| {
                // SAFETY: The context kind matches the C++ operation.
                black_box(unsafe { sky_flecs_cpp_bulk_insert(context.pointer()) });
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("single_insert_10k/flecs_cpp", |bencher| {
        bencher.iter_batched_ref(
            insert_context,
            |context| {
                // SAFETY: The context kind matches the C++ operation.
                black_box(unsafe { sky_flecs_cpp_single_insert(context.pointer()) });
            },
            BatchSize::SmallInput,
        );
    });
}

pub fn bench_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    let mut context = simple_context(SIMPLE_ENTITY_COUNT);
    group.bench_function("simple_10k/flecs_cpp", |bencher| {
        bencher.iter(|| {
            // SAFETY: The context remains alive and has the expected concrete type.
            black_box(unsafe { sky_flecs_cpp_simple_run(context.pointer(), 1) });
        });
    });
}

pub fn bench_iteration_repeated(group: &mut BenchmarkGroup<'_, WallTime>) {
    let mut context = simple_context(SIMPLE_ENTITY_COUNT);
    group.bench_function("simple_x32/flecs_cpp", |bencher| {
        bencher.iter(|| {
            // SAFETY: The context remains alive and has the expected concrete type.
            black_box(unsafe {
                sky_flecs_cpp_simple_run(context.pointer(), REPEATED_ITERATION_COUNT)
            });
        });
    });
}

pub fn bench_iteration_large(group: &mut BenchmarkGroup<'_, WallTime>) {
    let mut context = simple_context(LARGE_ITERATION_ENTITY_COUNT);
    group.bench_function("simple_100k/flecs_cpp", |bencher| {
        bencher.iter(|| {
            // SAFETY: The context remains alive and has the expected concrete type.
            black_box(unsafe { sky_flecs_cpp_simple_run(context.pointer(), 1) });
        });
    });
}

pub fn bench_fragmented_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    let mut context = fragmented_context();
    group.bench_function("fragmented_26x400/flecs_cpp", |bencher| {
        bencher.iter(|| {
            // SAFETY: The context remains alive and has the expected concrete type.
            black_box(unsafe { sky_flecs_cpp_fragmented_run(context.pointer()) });
        });
    });
}

pub fn bench_random_fragmented_iteration(group: &mut BenchmarkGroup<'_, WallTime>) {
    for component_count in RANDOM_FRAGMENT_COMPONENT_COUNTS {
        let expected = random_fragment_match_count(&random_fragment_masks(component_count)) as u64;
        let mut context = random_fragmented_context(component_count);
        // SAFETY: The context remains alive and has the expected concrete type.
        assert_eq!(
            unsafe { sky_flecs_cpp_random_fragmented_count(context.pointer()) },
            expected
        );

        group.bench_function(
            format!("random_{component_count}_components_4_terms/flecs_cpp"),
            |bencher| {
                bencher.iter(|| {
                    // SAFETY: The context remains alive and has the expected concrete type.
                    black_box(unsafe { sky_flecs_cpp_random_fragmented_run(context.pointer()) });
                });
            },
        );
    }
}

pub fn bench_heavy_compute(group: &mut BenchmarkGroup<'_, WallTime>) {
    let mut context = heavy_context();
    group.bench_function("heavy/flecs_cpp", |bencher| {
        bencher.iter(|| {
            // SAFETY: The context remains alive and has the expected concrete type.
            black_box(unsafe { sky_flecs_cpp_heavy_run(context.pointer()) });
        });
    });
}

pub fn bench_random_access(group: &mut BenchmarkGroup<'_, WallTime>) {
    for (name, count) in [
        ("hot_10k", SIMPLE_ENTITY_COUNT),
        ("warm_100k", WARM_RANDOM_ENTITY_COUNT),
        ("cold_1m", COLD_RANDOM_ENTITY_COUNT),
    ] {
        let mut context = random_context(count);
        group.bench_function(format!("{name}/flecs_cpp"), |bencher| {
            bencher.iter(|| {
                // SAFETY: The context remains alive and has the expected concrete type.
                black_box(unsafe { sky_flecs_cpp_random_run(context.pointer()) });
            });
        });
    }
}

pub fn bench_entity_ops(group: &mut BenchmarkGroup<'_, WallTime>) {
    let mut spawn_context = entity_ops_context();
    group.bench_function("spawn_despawn_1k/flecs_cpp", |bencher| {
        bencher.iter(|| {
            // SAFETY: The context remains alive and has the expected concrete type.
            black_box(unsafe { sky_flecs_cpp_spawn_despawn(spawn_context.pointer()) });
        });
    });

    let mut component_context = add_remove_context();
    group.bench_function("add_remove_component_1k/flecs_cpp", |bencher| {
        bencher.iter(|| {
            // SAFETY: The context remains alive and has the expected concrete type.
            black_box(unsafe { sky_flecs_cpp_add_remove(component_context.pointer()) });
        });
    });
}

pub fn bench_mixed_frame(group: &mut BenchmarkGroup<'_, WallTime>) {
    let mut context = mixed_context();
    group.bench_function("frame/flecs_cpp", |bencher| {
        bencher.iter(|| {
            // SAFETY: The context remains alive and has the expected concrete type.
            black_box(unsafe { sky_flecs_cpp_mixed_frame(context.pointer()) });
        });
    });
}

pub fn bench_mixed_frame_phases(group: &mut BenchmarkGroup<'_, WallTime>) {
    bench_mixed_phase(group, "movement/flecs_cpp", sky_flecs_cpp_mixed_movement);
    bench_mixed_phase(group, "health/flecs_cpp", sky_flecs_cpp_mixed_health);
    bench_mixed_phase(group, "heavy/flecs_cpp", sky_flecs_cpp_mixed_heavy);
    bench_mixed_phase(group, "random_access/flecs_cpp", sky_flecs_cpp_mixed_random);
    bench_mixed_phase(
        group,
        "structural_churn/flecs_cpp",
        sky_flecs_cpp_mixed_churn,
    );
    bench_mixed_phase(group, "spawn_despawn/flecs_cpp", sky_flecs_cpp_mixed_spawn);
}

fn bench_mixed_phase(
    group: &mut BenchmarkGroup<'_, WallTime>,
    name: &str,
    operation: unsafe extern "C" fn(*mut c_void) -> u64,
) {
    let mut context = mixed_context();
    group.bench_function(name, |bencher| {
        bencher.iter(|| {
            // SAFETY: Every operation accepted here is declared for MixedContext.
            black_box(unsafe { operation(context.pointer()) });
        });
    });
}

pub fn validate_contract() {
    // SAFETY: Validation owns all native state internally and returns a boolean.
    assert!(unsafe { sky_flecs_cpp_validate() });
}
