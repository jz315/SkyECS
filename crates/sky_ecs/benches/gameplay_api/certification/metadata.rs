use serde::Serialize;
use std::fs;
use std::process::Command;

#[derive(Debug, Serialize)]
pub(super) struct Metadata {
    source_revision: String,
    working_tree_dirty: bool,
    cpu: String,
    os: String,
    rustc: String,
    rustflags: String,
    cargo_profile: &'static str,
    command: &'static str,
}

pub(super) fn collect() -> Metadata {
    Metadata {
        source_revision: command_output("git", &["rev-parse", "HEAD"]),
        working_tree_dirty: !command_output("git", &["status", "--porcelain"]).is_empty(),
        cpu: cpu_name(),
        os: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        rustc: command_output("rustc", &["-Vv"]),
        rustflags: std::env::var("RUSTFLAGS").unwrap_or_default(),
        cargo_profile: "bench",
        command: "SKY_ECS_CERTIFY_GAMEPLAY_API=1 cargo bench -p sky_ecs --bench gameplay_api",
    }
}

fn cpu_name() -> String {
    if let Ok(value) = std::env::var("PROCESSOR_IDENTIFIER") {
        return value;
    }
    if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo") {
        if let Some(model) = cpuinfo.lines().find_map(|line| {
            line.strip_prefix("model name")
                .and_then(|line| line.split_once(':'))
                .map(|(_, value)| value.trim())
        }) {
            return model.to_owned();
        }
    }
    "unknown".to_owned()
}

fn command_output(program: &str, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_else(|| "unknown".to_owned())
}
