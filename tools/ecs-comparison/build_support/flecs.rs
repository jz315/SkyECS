use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const FLECS_C_CORE_VERSION: &str = "4.1.6";
const FLECS_C_CORE_COMMIT: &str = "fb55f3c25660425cfe1bc4cf5e6bff8b3f18a9b8";
const FLECS_C_CORE_DIR: &str = "vendor/flecs-4.1.6";
const FLECS_C_SOURCES: &[&str] = &[
    "native/flecs_c/construction.cpp",
    "native/flecs_c/entity_operations.cpp",
    "native/flecs_c/fragmented_iteration.cpp",
    "native/flecs_c/gameplay_frame.cpp",
    "native/flecs_c/iteration.cpp",
    "native/flecs_c/mixed_frame.cpp",
    "native/flecs_c/random_access.cpp",
    "native/flecs_c/random_fragmentation.cpp",
    "native/flecs_c/validation.cpp",
];
const FLECS_C_HEADERS: &[&str] = &["native/flecs_c/math.hpp"];
pub fn build() {
    println!("cargo:rerun-if-changed=build_support/flecs.rs");
    println!("cargo:rerun-if-env-changed=SKY_LLVM_ROOT");
    for path in FLECS_C_SOURCES.iter().chain(FLECS_C_HEADERS) {
        println!("cargo:rerun-if-changed={path}");
    }
    println!("cargo:rerun-if-changed={FLECS_C_CORE_DIR}/flecs.c");
    println!("cargo:rerun-if-changed={FLECS_C_CORE_DIR}/flecs.h");
    let manifest_dir = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("Cargo should set CARGO_MANIFEST_DIR"),
    );
    let native_include = manifest_dir.join(FLECS_C_CORE_DIR);
    let native_core_version = flecs_core_version(&native_include);
    assert_eq!(
        native_core_version, FLECS_C_CORE_VERSION,
        "the vendored native Flecs core must match the pinned benchmark version"
    );
    println!("cargo:rustc-env=SKY_FLECS_C_CORE_VERSION={native_core_version}");
    println!("cargo:rustc-env=SKY_FLECS_C_CORE_COMMIT={FLECS_C_CORE_COMMIT}");

    let optimized = env::var("OPT_LEVEL").is_ok_and(|level| level != "0");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo should set OUT_DIR"));
    build_flecs_c_library(&native_include, &out_dir, optimized);
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

fn configure_flecs_c(build: &mut cc::Build) {
    build
        .define("FLECS_CUSTOM_BUILD", None)
        .define("FLECS_OS_API_IMPL", None)
        .define("FLECS_TERM_COUNT_MAX", "32");
}

fn build_flecs_c_library(include: &Path, out_dir: &Path, optimized: bool) {
    let target_env = env::var("CARGO_CFG_TARGET_ENV").expect("Cargo should set target env");
    let msvc_target = target_env == "msvc";
    let c_compiler = llvm_tool(if msvc_target { "clang-cl" } else { "clang" });
    let cxx_compiler = llvm_tool(if msvc_target { "clang-cl" } else { "clang++" });
    let archiver = llvm_tool(if msvc_target { "llvm-lib" } else { "llvm-ar" });

    let mut core = cc::Build::new();
    core.compiler(&c_compiler)
        .include(include)
        .warnings(false)
        .extra_warnings(false);
    configure_flecs_c(&mut core);
    configure_optimization(&mut core, optimized);
    let core_compiler = core.get_compiler();
    ensure_llvm_compiler(&core_compiler, "Flecs C core");
    let msvc_style = core_compiler.is_like_msvc();
    let object_extension = if msvc_style { "obj" } else { "o" };
    let core_object = out_dir.join(format!("flecs_c_core.{object_extension}"));
    println!(
        "cargo:rustc-env=SKY_FLECS_C_CORE_COMPILER={}",
        core_compiler.path().display()
    );
    compile_object(
        &core,
        &include.join("flecs.c"),
        &core_object,
        msvc_style,
        "Flecs C API core",
    );

    let mut adapter = cc::Build::new();
    adapter
        .compiler(&cxx_compiler)
        .cpp(true)
        .include(include)
        .flag_if_supported("-std=c++17")
        .flag_if_supported("/std:c++17")
        .warnings(true)
        .extra_warnings(true);
    configure_flecs_c(&mut adapter);
    configure_optimization(&mut adapter, optimized);
    configure_float_semantics(&mut adapter, msvc_target);
    let adapter_compiler = adapter.get_compiler();
    ensure_llvm_compiler(&adapter_compiler, "Flecs C adapter");
    ensure_matching_compilers(&core_compiler, &adapter_compiler);
    let optimization = "-O3 -flto -DNDEBUG; fp-contract=off; LLVM linker LTO";
    println!("cargo:rustc-env=SKY_FLECS_C_OPTIMIZATION={optimization}");
    println!(
        "cargo:rustc-env=SKY_FLECS_C_LLVM_VERSION={}",
        compiler_version(&adapter_compiler)
    );
    if adapter_compiler.is_like_msvc() {
        adapter.flag_if_supported("/WX");
    } else {
        adapter.flag_if_supported("-Werror");
    }
    println!(
        "cargo:rustc-env=SKY_FLECS_C_ADAPTER_COMPILER={}",
        adapter_compiler.path().display()
    );
    let adapter_objects: Vec<_> = FLECS_C_SOURCES
        .iter()
        .map(|source| {
            let source_path = Path::new(source);
            let stem = source_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .expect("Flecs C API source should have a UTF-8 file stem");
            let object = out_dir.join(format!("flecs_c_{stem}.{object_extension}"));
            compile_object(
                &adapter,
                source_path,
                &object,
                adapter_compiler.is_like_msvc(),
                &format!("Flecs C API {stem} adapter"),
            );
            object
        })
        .collect();

    // Archive the separately compiled C core and C++ adapters together. A
    // single static library avoids archive-order problems and removes the
    // dynamic-loader/function-pointer overhead from short benchmarks.
    let mut library = cc::Build::new();
    library
        .compiler(&cxx_compiler)
        .archiver(&archiver)
        .cpp(true)
        .cargo_metadata(false)
        .object(&core_object)
        .objects(&adapter_objects);
    library.compile("sky_flecs_c_native");
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    link_cpp_standard_library();
    if optimized && msvc_target {
        // rust-lld consumes the clang-cl bitcode objects from the static
        // archive and performs native LTO at the final executable link.
        println!("cargo:rustc-link-arg=/LTCG");
        println!("cargo:rustc-env=SKY_FLECS_C_LINKER=rust-lld /LTCG static archive");
    } else if optimized {
        println!("cargo:rustc-link-arg=-flto");
        println!("cargo:rustc-env=SKY_FLECS_C_LINKER=clang + lld LTO static archive");
    } else {
        println!("cargo:rustc-env=SKY_FLECS_C_LINKER=LLVM static archive link");
    }
}

fn llvm_tool(name: &str) -> PathBuf {
    let executable = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };
    let Some(root) = env::var_os("SKY_LLVM_ROOT").filter(|root| !root.is_empty()) else {
        return PathBuf::from(executable);
    };
    let root = PathBuf::from(root);
    let in_bin = root.join("bin").join(&executable);
    let path = if in_bin.is_file() {
        in_bin
    } else {
        root.join(&executable)
    };
    assert!(
        path.is_file(),
        "{name} was not found under SKY_LLVM_ROOT={} (expected {})",
        root.display(),
        path.display()
    );
    path
}

fn ensure_llvm_compiler(compiler: &cc::Tool, description: &str) {
    assert!(
        compiler.is_like_clang() || compiler.is_like_clang_cl(),
        "{description} must use Clang/LLVM, got {}",
        compiler.path().display()
    );
}

fn compiler_version(compiler: &cc::Tool) -> String {
    let output = compiler
        .to_command()
        .arg("--version")
        .output()
        .unwrap_or_else(|error| panic!("failed to query Clang version: {error}"));
    assert!(output.status.success(), "failed to query Clang version");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or("unknown Clang version")
        .to_owned()
}

fn link_cpp_standard_library() {
    let target_env = env::var("CARGO_CFG_TARGET_ENV").expect("Cargo should set target env");
    if target_env == "msvc" {
        return;
    }

    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("Cargo should set target OS");
    let library = env::var("CXXSTDLIB").unwrap_or_else(|_| {
        if matches!(
            target_os.as_str(),
            "macos" | "ios" | "tvos" | "watchos" | "freebsd" | "openbsd"
        ) {
            "c++".to_owned()
        } else {
            "stdc++".to_owned()
        }
    });
    if !library.is_empty() {
        println!("cargo:rustc-link-lib=dylib={library}");
    }
}

fn ensure_matching_compilers(core: &cc::Tool, adapter: &cc::Tool) {
    let family = |compiler: &cc::Tool| {
        if compiler.is_like_clang_cl() {
            "clang-cl"
        } else if compiler.is_like_msvc() {
            "msvc"
        } else if compiler.is_like_clang() {
            "clang"
        } else if compiler.is_like_gnu() {
            "gnu"
        } else {
            "unknown"
        }
    };
    assert_eq!(
        family(core),
        family(adapter),
        "Flecs C core and adapter must use the same compiler family (core: {}, adapter: {})",
        core.path().display(),
        adapter.path().display()
    );
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

fn configure_float_semantics(build: &mut cc::Build, msvc_target: bool) {
    if msvc_target {
        build.flag("/clang:-ffp-contract=off");
    } else {
        build.flag("-ffp-contract=off");
    }
}

fn configure_optimization(build: &mut cc::Build, optimized: bool) {
    if !optimized {
        return;
    }
    build.opt_level(3).define("NDEBUG", None);
    build.flag_if_supported("-O3").flag_if_supported("-flto");
}
