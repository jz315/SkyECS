use std::env;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

fn native_optimized_flags() -> &'static str {
    env!("SKY_FLECS_C_OPTIMIZATION")
}

pub(super) fn write_metadata(root: &Path, report_dir: &Path) -> io::Result<()> {
    let mut output = File::create(report_dir.join("environment.txt"))?;
    writeln!(output, "os={} arch={}", env::consts::OS, env::consts::ARCH)?;
    writeln!(output, "criterion_estimator=median")?;
    writeln!(output, "rust_bench_lto=fat codegen_units=1 opt_level=3")?;
    writeln!(output, "native_c_flags={}", native_optimized_flags())?;
    writeln!(
        output,
        "flecs_c_core_compiler={}",
        env!("SKY_FLECS_C_CORE_COMPILER")
    )?;
    writeln!(
        output,
        "flecs_c_adapter_compiler={}",
        env!("SKY_FLECS_C_ADAPTER_COMPILER")
    )?;
    writeln!(output, "flecs_c_linker={}", env!("SKY_FLECS_C_LINKER"))?;
    writeln!(
        output,
        "flecs_c_llvm_version={}",
        env!("SKY_FLECS_C_LLVM_VERSION")
    )?;
    writeln!(
        output,
        "rust_linker={}",
        env::var("CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER").unwrap_or_else(|_| {
            if cfg!(target_os = "windows") {
                "rust-lld (workspace target config)".to_owned()
            } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
                "clang -fuse-ld=lld (workspace target config)".to_owned()
            } else {
                "target default".to_owned()
            }
        })
    )?;
    writeln!(
        output,
        "flecs_c_core_version={} commit={}",
        env!("SKY_FLECS_C_CORE_VERSION"),
        env!("SKY_FLECS_C_CORE_COMMIT")
    )?;
    writeln!(
        output,
        "processor={}",
        env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "unknown".to_owned())
    )?;
    for (program, args) in [
        ("rustc", vec!["-Vv"]),
        ("git", vec!["rev-parse", "HEAD"]),
        ("git", vec!["status", "--short"]),
        (
            "cargo",
            vec!["tree", "-p", "sky_ecs_comparison", "--depth", "1"],
        ),
    ] {
        let result = Command::new(program)
            .current_dir(root)
            .args(args)
            .output()?;
        output.write_all(&result.stdout)?;
        output.write_all(&result.stderr)?;
        writeln!(output)?;
    }
    Ok(())
}
