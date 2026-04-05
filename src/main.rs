use clap::Parser;
use mdlint::{config, report, rules, workspace};
use miette::{miette, Result};
use std::path::PathBuf;
use std::process;

#[derive(Parser, Debug)]
#[command(name = "mdlint", about = "Markdown linter with backlink validation")]
struct Cli {
    /// Path to lint (file or directory)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Path to config file
    #[arg(long, short)]
    config: Option<PathBuf>,

    /// Output format
    #[arg(long, default_value = "pretty")]
    format: report::OutputFormat,

    /// Only show errors, suppress warnings
    #[arg(long, short)]
    quiet: bool,

    /// Lint specific files instead of a directory
    #[arg(long, num_args = 1..)]
    files: Vec<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let cfg = config::Config::load(cli.config.as_deref(), &cli.path)
        .map_err(|e| miette!("failed to load config: {e}"))?;

    let ws = if cli.files.is_empty() {
        workspace::Workspace::from_directory(&cli.path, &cfg)
            .map_err(|e| miette!("failed to build workspace: {e}"))?
    } else {
        workspace::Workspace::from_files(&cli.files, &cfg)
            .map_err(|e| miette!("failed to build workspace: {e}"))?
    };

    let diagnostics = rules::run_all(&ws, &cfg);

    let filtered = if cli.quiet {
        diagnostics
            .into_iter()
            .filter(|d| d.severity == rules::Severity::Error)
            .collect()
    } else {
        diagnostics
    };

    let has_errors = filtered
        .iter()
        .any(|d| d.severity == rules::Severity::Error);

    report::print_diagnostics(&filtered, cli.format);

    if has_errors {
        process::exit(1);
    }

    Ok(())
}
