use std::env;
use std::path::PathBuf;

const DEFAULT_RUN_COUNT: usize = 4;

pub(super) struct Options {
    pub(super) runs: usize,
    pub(super) filter: Option<String>,
    pub(super) reanalyze: Option<PathBuf>,
}

pub(super) fn options() -> Result<Options, Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let mut runs_explicit = false;
    let mut options = Options {
        runs: DEFAULT_RUN_COUNT,
        filter: None,
        reanalyze: None,
    };
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--runs" => {
                runs_explicit = true;
                options.runs = args.next().ok_or("--runs requires a value")?.parse()?;
                if options.runs == 0 {
                    return Err("--runs must be at least 1".into());
                }
            }
            "--filter" => options.filter = Some(args.next().ok_or("--filter requires a value")?),
            "--reanalyze" => {
                options.reanalyze = Some(PathBuf::from(
                    args.next().ok_or("--reanalyze requires a report path")?,
                ));
            }
            _ => return Err(format!("unknown argument `{argument}`").into()),
        }
    }
    if options.reanalyze.is_some() && (options.filter.is_some() || runs_explicit) {
        return Err("--reanalyze cannot be combined with --runs or --filter".into());
    }
    Ok(options)
}
