// === culm

#![forbid(unsafe_code)]

use std::process::ExitCode;

use clap::{Parser, Subcommand};
use culm::banner;
use culm::cli::{NewArgs, run_new};

#[derive(Parser)]
#[command(name = "culm", version, about = "Scaffold a workspace from layers")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new workspace.
    New(NewArgs),
}

/* An `ExitCode` rather than `Result`, because the `Termination` impl for
 * `Result` prints the error with `Debug`, which shows a struct where the user
 * needs a sentence. */
fn main() -> ExitCode {
    let cli = Cli::parse();

    /* After parsing, so `--help` and `--version` stay clean, and so a bad
     * argument is rejected before anything is drawn. */
    banner::print();

    let result = match &cli.command {
        Command::New(args) => match std::env::current_dir() {
            Ok(cwd) => run_new(args, &cwd),
            Err(e) => Err(anyhow::Error::new(e).context("cannot read the working directory")),
        },
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}
