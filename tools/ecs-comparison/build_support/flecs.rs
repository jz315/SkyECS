use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const FLECS_RUST_SYS_VERSION: &str = "0.2.1";
const FLECS_CPP_CORE_VERSION: &str = "4.1.6";
const FLECS_CPP_CORE_COMMIT: &str = "fb55f3c25660425cfe1bc4cf5e6bff8b3f18a9b8";
const FLECS_CPP_CORE_DIR: &str = "vendor/flecs-4.1.6";
const FLECS_CPP_EXPORTS: &[&str] = &[
    "sky_flecs_cpp_insert_new",
    "sky_flecs_cpp_insert_delete",
    "sky_flecs_cpp_bulk_insert",
    "sky_flecs_cpp_single_insert",
    "sky_flecs_cpp_simple_new",
    "sky_flecs_cpp_simple_delete",
    "sky_flecs_cpp_simple_run",
    "sky_flecs_cpp_fragmented_new",
    "sky_flecs_cpp_fragmented_delete",
    "sky_flecs_cpp_fragmented_run",
    "sky_flecs_cpp_random_fragmented_new",
    "sky_flecs_cpp_random_fragmented_delete",
    "sky_flecs_cpp_random_fragmented_run",
    "sky_flecs_cpp_random_fragmented_count",
    "sky_flecs_cpp_heavy_new",
    "sky_flecs_cpp_heavy_delete",
    "sky_flecs_cpp_heavy_run",
    "sky_flecs_cpp_random_new",
    "sky_flecs_cpp_random_delete",
    "sky_flecs_cpp_random_run",
    "sky_flecs_cpp_entity_ops_new",
    "sky_flecs_cpp_entity_ops_delete",
    "sky_flecs_cpp_spawn_despawn",
    "sky_flecs_cpp_add_remove_new",
    "sky_flecs_cpp_add_remove_delete",
    "sky_flecs_cpp_add_remove",
    "sky_flecs_cpp_mixed_new",
    "sky_flecs_cpp_mixed_delete",
    "sky_flecs_cpp_mixed_frame",
    "sky_flecs_cpp_mixed_movement",
    "sky_flecs_cpp_mixed_health",
    "sky_flecs_cpp_mixed_heavy",
    "sky_flecs_cpp_mixed_random",
    "sky_flecs_cpp_mixed_churn",
    "sky_flecs_cpp_mixed_spawn",
    "sky_flecs_cpp_validate",
];

pub fn build() {
    println!("cargo:rerun-if-changed=build_support/flecs.rs");
    println!("cargo:rerun-if-changed=native/flecs_cpp_adapter.cpp");
    println!("cargo:rerun-if-changed={FLECS_CPP_CORE_DIR}/flecs.c");
    println!("cargo:rerun-if-changed={FLECS_CPP_CORE_DIR}/flecs.h");
    println!("cargo:rerun-if-env-changed=FLECS_RUST_INCLUDE");

    let rust_include = env::var_os("FLECS_RUST_INCLUDE")
        .map(PathBuf::from)
        .or_else(find_registry_include)
        .unwrap_or_else(|| {
            panic!(
                "could not locate flecs_ecs_sys {FLECS_RUST_SYS_VERSION}; set FLECS_RUST_INCLUDE to the directory containing flecs.h and flecs_rust.c"
            )
        });
    let manifest_dir = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("Cargo should set CARGO_MANIFEST_DIR"),
    );
    let cpp_include = manifest_dir.join(FLECS_CPP_CORE_DIR);

    println!(
        "cargo:rerun-if-changed={}",
        rust_include.join("flecs.h").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        rust_include.join("flecs_rust.c").display()
    );
    let rust_core_version = flecs_core_version(&rust_include);
    let cpp_core_version = flecs_core_version(&cpp_include);
    assert_eq!(
        cpp_core_version, FLECS_CPP_CORE_VERSION,
        "the vendored native Flecs core must match the pinned benchmark version"
    );
    println!("cargo:rustc-env=SKY_FLECS_RUST_CORE_VERSION={rust_core_version}");
    println!("cargo:rustc-env=SKY_FLECS_CPP_CORE_VERSION={cpp_core_version}");
    println!("cargo:rustc-env=SKY_FLECS_CPP_CORE_COMMIT={FLECS_CPP_CORE_COMMIT}");

    let optimized = env::var("OPT_LEVEL").is_ok_and(|level| level != "0");
    let msvc = env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo should set OUT_DIR"));
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("Cargo should set target OS");

    // The Rust binding uses its default runtime alias checks. Keep that ABI in
    // the statically linked core instead of silently benchmarking a less-safe
    // configuration than normal Rust users receive.
    let mut core = cc::Build::new();
    core.file(rust_include.join("flecs_rust.c"))
        .include(&rust_include);
    configure_flecs_rust(&mut core);
    configure_optimization(&mut core, optimized, msvc);
    core.warnings(true).extra_warnings(true);
    core.compile("sky_flecs_c_core");

    // Native C++ Flecs uses its own release-configured core. A dynamic-library
    // boundary prevents its normal lock-free C++ ABI from being interposed by
    // the Rust binding's safety-lock core in the benchmark executable.
    let cpp_library = build_flecs_cpp_library(&cpp_include, &out_dir, optimized, msvc, &target_os);
    println!(
        "cargo:rustc-env=SKY_FLECS_CPP_NATIVE_PATH={}",
        cpp_library.display()
    );

    if optimized {
        if msvc {
            println!("cargo:rustc-link-arg=/LTCG");
            println!("cargo:rustc-link-arg=/WHOLEARCHIVE:sky_flecs_c_core.lib");
        } else {
            println!("cargo:rustc-link-arg=-flto");
        }
    }
}

fn flecs_core_version(include: &Path) -> String {
    let header_path = include.join("flecs.h");
    let header = fs::read_to_string(&header_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", header_path.display()));
    let value = |name: &str| {
        header
            .lines()
            .find_map(|line| {
                let mut fields = line.split_whitespace();
                (fields.next() == Some("#define") && fields.next() == Some(name))
                    .then(|| fields.next().map(str::to_owned))
                    .flatten()
            })
            .unwrap_or_else(|| panic!("{name} is missing from {}", header_path.display()))
    };
    format!(
        "{}.{}.{}",
        value("FLECS_VERSION_MAJOR"),
        value("FLECS_VERSION_MINOR"),
        value("FLECS_VERSION_PATCH")
    )
}

fn configure_flecs_rust(build: &mut cc::Build) {
    build
        .define("FLECS_CUSTOM_BUILD", None)
        .define("FLECS_CPP", None)
        .define("FLECS_MODULE", None)
        .define("FLECS_SCRIPT", None)
        .define("FLECS_METRICS", None)
        .define("FLECS_ALERTS", None)
        .define("FLECS_SYSTEM", None)
        .define("FLECS_PIPELINE", None)
        .define("FLECS_TIMER", None)
        .define("FLECS_META", None)
        .define("FLECS_META_C", None)
        .define("FLECS_UNITS", None)
        .define("FLECS_DOC", None)
        .define("FLECS_OS_API_IMPL", None)
        .define("FLECS_MUT_ALIAS_LOCKS", None)
        .define("FLECS_TERM_COUNT_MAX", "32");
}

fn configure_flecs_cpp(build: &mut cc::Build) {
    build
        .define("FLECS_CUSTOM_BUILD", None)
        .define("FLECS_CPP", None)
        .define("FLECS_OS_API_IMPL", None)
        .define("FLECS_TERM_COUNT_MAX", "32")
        .define("flecs_STATIC", None);
}

fn build_flecs_cpp_library(
    include: &Path,
    out_dir: &Path,
    optimized: bool,
    msvc: bool,
    target_os: &str,
) -> PathBuf {
    let object_extension = if msvc { "obj" } else { "o" };
    let core_object = out_dir.join(format!("flecs_cpp_core.{object_extension}"));
    let adapter_object = out_dir.join(format!("flecs_cpp_adapter.{object_extension}"));

    let mut core = cc::Build::new();
    core.include(include).warnings(false).extra_warnings(false);
    configure_flecs_cpp(&mut core);
    configure_optimization(&mut core, optimized, msvc);
    if !msvc {
        core.flag_if_supported("-fPIC")
            .flag_if_supported("-fvisibility=hidden");
    }
    compile_object(
        &core,
        &include.join("flecs.c"),
        &core_object,
        msvc,
        "Flecs C++ core",
    );

    let mut adapter = cc::Build::new();
    adapter
        .cpp(true)
        .include(include)
        .flag_if_supported("-std=c++17")
        .flag_if_supported("/std:c++17")
        .warnings(true)
        .extra_warnings(true);
    configure_flecs_cpp(&mut adapter);
    configure_optimization(&mut adapter, optimized, msvc);
    if !msvc {
        adapter.flag_if_supported("-fPIC");
    }
    compile_object(
        &adapter,
        Path::new("native/flecs_cpp_adapter.cpp"),
        &adapter_object,
        msvc,
        "Flecs C++ adapter",
    );

    let file_name = match target_os {
        "windows" => "sky_flecs_cpp_native.dll",
        "macos" => "libsky_flecs_cpp_native.dylib",
        _ => "libsky_flecs_cpp_native.so",
    };
    let library = out_dir.join(file_name);
    let mut command = adapter.get_compiler().to_command();
    if msvc {
        let import_library = out_dir.join("sky_flecs_cpp_native.lib");
        command
            .arg("/LD")
            .arg(&core_object)
            .arg(&adapter_object)
            .arg("/link")
            .arg("/NOLOGO")
            .arg(format!("/OUT:{}", library.display()))
            .arg(format!("/IMPLIB:{}", import_library.display()));
        if optimized {
            command.arg("/LTCG");
        }
        for symbol in FLECS_CPP_EXPORTS {
            command.arg(format!("/EXPORT:{symbol}"));
        }
    } else {
        command.arg(if target_os == "macos" {
            "-dynamiclib"
        } else {
            "-shared"
        });
        command
            .arg(&core_object)
            .arg(&adapter_object)
            .arg("-o")
            .arg(&library);
        if target_os != "macos" {
            command.arg("-Wl,-Bsymbolic");
        }
    }
    run_command(command, "link isolated Flecs C++ library");
    library
}

fn compile_object(build: &cc::Build, source: &Path, output: &Path, msvc: bool, description: &str) {
    let mut command = build.get_compiler().to_command();
    if msvc {
        command
            .arg("/c")
            .arg(source)
            .arg(format!("/Fo{}", output.display()));
    } else {
        command.arg("-c").arg(source).arg("-o").arg(output);
    }
    run_command(command, &format!("compile {description}"));
}

fn run_command(mut command: Command, description: &str) {
    let output = command
        .output()
        .unwrap_or_else(|error| panic!("failed to {description}: {error}"));
    if !output.status.success() {
        panic!(
            "failed to {description}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn configure_optimization(build: &mut cc::Build, optimized: bool, msvc: bool) {
    if !optimized {
        return;
    }
    build.opt_level(3).define("NDEBUG", None);
    if msvc {
        build.flag_if_supported("/GL");
    } else {
        build.flag_if_supported("-flto");
    }
}

fn find_registry_include() -> Option<PathBuf> {
    let cargo_home = env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".cargo")))
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")))?;
    let registry_sources = cargo_home.join("registry").join("src");
    let package = format!("flecs_ecs_sys-{FLECS_RUST_SYS_VERSION}");

    find_package_include(&registry_sources, &package)
}

fn find_package_include(registry_sources: &Path, package: &str) -> Option<PathBuf> {
    let registries = std::fs::read_dir(registry_sources).ok()?;
    for registry in registries.filter_map(Result::ok) {
        let include = registry.path().join(package).join("src");
        if include.join("flecs.h").is_file() && include.join("flecs_rust.c").is_file() {
            return Some(include);
        }
    }
    None
}
