use super::model::ContractVerification;
use sky_ecs_comparison::common::is_canonical_group;
use sky_ecs_comparison::Engine;
use std::env;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub(super) fn workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let output = Command::new(cargo())
        .args(["locate-project", "--workspace", "--message-format", "plain"])
        .output()?;
    if !output.status.success() {
        return Err("cargo locate-project failed".into());
    }
    let manifest = PathBuf::from(String::from_utf8(output.stdout)?.trim());
    Ok(manifest
        .parent()
        .ok_or("workspace manifest has no parent")?
        .to_path_buf())
}

fn cargo() -> String {
    env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned())
}

pub(super) fn run_contracts(
    root: &Path,
    report_dir: &Path,
) -> Result<ContractVerification, Box<dyn std::error::Error>> {
    let log_path = report_dir.join("contracts.log");
    let log = File::create(&log_path)?;
    let error_log = log.try_clone()?;
    let status = Command::new(cargo())
        .current_dir(root)
        .args([
            "test",
            "--release",
            "-p",
            "sky_ecs_comparison",
            "--test",
            "contracts",
            "--",
            "--test-threads=1",
        ])
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(error_log))
        .status()?;
    if !status.success() {
        return Err(format!(
            "release comparison contracts failed; inspect {}",
            log_path.display()
        )
        .into());
    }
    let commit = Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "HEAD"])
        .output()?;
    if !commit.status.success() {
        return Err("git rev-parse HEAD failed after contracts".into());
    }
    Ok(ContractVerification {
        status: "passed".to_owned(),
        profile: "release".to_owned(),
        commit: String::from_utf8(commit.stdout)?.trim().to_owned(),
        log: "contracts.log".to_owned(),
    })
}

pub(super) fn rotated_order(offset: usize) -> String {
    (0..Engine::ALL.len())
        .map(|index| Engine::ALL[(index + offset) % Engine::ALL.len()].name())
        .collect::<Vec<_>>()
        .join(",")
}

pub(super) fn clear_results(benchmark_target: &Path) -> io::Result<()> {
    let criterion = benchmark_target.join("criterion");
    let Ok(entries) = fs::read_dir(criterion) else {
        return Ok(());
    };
    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_dir() && is_canonical_group(&entry.file_name().to_string_lossy()) {
            fs::remove_dir_all(entry.path())?;
        }
    }
    Ok(())
}

pub(super) fn run_bench(
    root: &Path,
    benchmark_target: &Path,
    report_dir: &Path,
    run: usize,
    order: &str,
    filter: Option<&str>,
) -> io::Result<()> {
    let log_path = report_dir.join(format!("run-{}.log", run + 1));
    let log = File::create(log_path)?;
    let error_log = log.try_clone()?;
    let mut command = Command::new(cargo());
    command.current_dir(root).args([
        "bench",
        "-p",
        "sky_ecs_comparison",
        "--bench",
        "comparison",
        "--",
    ]);
    if let Some(filter) = filter {
        command.arg(filter);
    }
    let status = command
        .arg("--noplot")
        .env("CARGO_TARGET_DIR", benchmark_target)
        .env("SKY_ECS_ORDER", order)
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(error_log))
        .status()?;
    if !status.success() {
        return Err(io::Error::other(format!(
            "publication run {} failed; inspect its log",
            run + 1
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn rotations_put_every_engine_in_every_position() {
        let mut positions: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
        for offset in 0..Engine::ALL.len() {
            for (position, engine) in rotated_order(offset).split(',').enumerate() {
                positions
                    .entry(engine.to_owned())
                    .or_default()
                    .insert(position);
            }
        }
        for engine in Engine::ALL {
            assert_eq!(positions[engine.name()].len(), Engine::ALL.len());
        }
    }
}
