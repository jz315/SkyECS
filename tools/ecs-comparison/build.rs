use std::env;
use std::path::{Path, PathBuf};

const FLECS_SYS_VERSION: &str = "0.2.1";

fn main() {
    println!("cargo:rerun-if-changed=native/flecs_cpp.cpp");
    println!("cargo:rerun-if-env-changed=FLECS_CPP_INCLUDE");

    let include = env::var_os("FLECS_CPP_INCLUDE")
        .map(PathBuf::from)
        .or_else(find_registry_include)
        .unwrap_or_else(|| {
            panic!(
                "could not locate flecs_ecs_sys {FLECS_SYS_VERSION}; set FLECS_CPP_INCLUDE to the directory containing flecs.h"
            )
        });

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .file("native/flecs_cpp.cpp")
        .include(include)
        .define("FLECS_CUSTOM_BUILD", None)
        .define("FLECS_CPP", None)
        .define("FLECS_MODULE", None)
        .define("FLECS_SCRIPT", None)
        .define("FLECS_STATS", None)
        .define("FLECS_METRICS", None)
        .define("FLECS_ALERTS", None)
        .define("FLECS_SYSTEM", None)
        .define("FLECS_PIPELINE", None)
        .define("FLECS_TIMER", None)
        .define("FLECS_META", None)
        .define("FLECS_META_C", None)
        .define("FLECS_UNITS", None)
        .define("FLECS_JSON", None)
        .define("FLECS_DOC", None)
        .define("FLECS_LOG", None)
        .define("FLECS_APP", None)
        .define("FLECS_HTTP", None)
        .define("FLECS_REST", None)
        .define("FLECS_OS_API_IMPL", None)
        .define("FLECS_MUT_ALIAS_LOCKS", None)
        .define("FLECS_TERM_COUNT_MAX", "32")
        .flag_if_supported("-std=c++17")
        .flag_if_supported("/std:c++17")
        .warnings(true)
        .extra_warnings(true);

    if env::var("PROFILE").as_deref() == Ok("release") {
        build.opt_level(3).define("NDEBUG", None);
    }
    build.compile("sky_flecs_cpp_adapter");
}

fn find_registry_include() -> Option<PathBuf> {
    let cargo_home = env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".cargo")))
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")))?;
    let registry_sources = cargo_home.join("registry").join("src");
    let package = format!("flecs_ecs_sys-{FLECS_SYS_VERSION}");

    find_package_include(&registry_sources, &package)
}

fn find_package_include(registry_sources: &Path, package: &str) -> Option<PathBuf> {
    let registries = std::fs::read_dir(registry_sources).ok()?;
    for registry in registries.filter_map(Result::ok) {
        let include = registry.path().join(package).join("src");
        if include.join("flecs.h").is_file() {
            return Some(include);
        }
    }
    None
}
