// === culm

#![forbid(unsafe_code)]

use std::process::ExitCode;

use clap::{Parser, Subcommand};
use culm::banner;
use culm::cli::{NewArgs, RunArgs, run_new, run_task};
use culm::telemetry;

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

    /// Run a task from `culm.toml`, respecting `depends_on`.
    Run(RunArgs),
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
        Command::Run(args) => match std::env::current_dir() {
            Ok(cwd) => run_task(args, &cwd),
            Err(e) => Err(anyhow::Error::new(e).context("cannot read the working directory")),
        },
    };

    /* Here, never in `cli::new`: this binary is the one caller a test never
     * goes through (`tests/cli.rs` calls `run_new` directly precisely to
     * avoid spawning it), which is what keeps `cargo test` from touching the
     * network or this machine's real telemetry id on every run. Fired
     * whether or not the confirmation prompt was accepted: a run without
     * `--yes` still named its layers before asking, and that naming is what
     * decision 9 calls the selection. */
    /* `Run` sends nothing: decision 9 explicitly refuses task names and
     * command strings, and a `run` payload has no other field worth
     * reporting yet. */
    if result.is_ok() {
        match &cli.command {
            Command::New(args) => telemetry::record("new", &args.layers, args.preset.as_deref()),
            Command::Run(_) => {}
        }
    }

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}
