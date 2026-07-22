use std::env;
use std::path::PathBuf;

const DEFAULT_RUN_COUNT: usize = 4;

pub(super) struct Options {
    pub(super) runs: usize,
    pub(super) filter: Option<String>,
    pub(super) reanalyze: Option<PathBuf>,
    pub(super) allow_dirty: bool,
}

pub(super) fn options() -> Result<Options, Box<dyn std::error::Error>> {
    parse_options(env::args().skip(1))
}

fn parse_options(
    arguments: impl IntoIterator<Item = String>,
) -> Result<Options, Box<dyn std::error::Error>> {
    let mut args = arguments.into_iter();
    let mut runs_explicit = false;
    let mut options = Options {
        runs: DEFAULT_RUN_COUNT,
        filter: None,
        reanalyze: None,
        allow_dirty: false,
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
            "--allow-dirty" => options.allow_dirty = true,
            _ => return Err(format!("unknown argument `{argument}`").into()),
        }
    }
    if options.reanalyze.is_some() && (options.filter.is_some() || runs_explicit) {
        return Err("--reanalyze cannot be combined with --runs or --filter".into());
    }
    Ok(options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirty_override_is_explicit_and_off_by_default() {
        assert!(!parse_options(Vec::new()).unwrap().allow_dirty);
        assert!(
            parse_options(vec!["--allow-dirty".to_owned()])
                .unwrap()
                .allow_dirty
        );
    }
}
