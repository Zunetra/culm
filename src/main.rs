// === culm

#![forbid(unsafe_code)]

use std::io;
use std::process::ExitCode;
use std::time::Instant;

use clap::{Parser, Subcommand};
use culm::banner;
use culm::cli::{
    CacheArgs, CacheCommand, GraphArgs, GraphFormat, NewArgs, NewReport, RunArgs, RunReport,
    run_cache, run_graph, run_new_reporting, run_task_reporting,
};
use culm::runner::RunnerError;
use culm::telemetry::{self, CacheAction, CacheStats, ErrorKind, Exit, GraphStats, Outcome, codes};

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

    /// Inspect or clear the local task cache.
    Cache(CacheArgs),

    /// Print the task graph, as a list or as Graphviz dot.
    Graph(GraphArgs),
}

/* An `ExitCode` rather than `Result`, because the `Termination` impl for
 * `Result` prints the error with `Debug`, which shows a struct where the user
 * needs a sentence. */
fn main() -> ExitCode {
    let cli = Cli::parse();

    /* After parsing, so `--help` and `--version` stay clean, and so a bad
     * argument is rejected before anything is drawn. */
    banner::print();

    let started = Instant::now();
    let mut new_report = NewReport::default();
    let mut run_report = RunReport::default();

    let result = match &cli.command {
        Command::New(args) => match std::env::current_dir() {
            Ok(cwd) => run_new_reporting(args, &cwd, &mut io::stdout(), &mut new_report),
            Err(e) => Err(anyhow::Error::new(e).context("cannot read the working directory")),
        },
        Command::Run(args) => match std::env::current_dir() {
            Ok(cwd) => run_task_reporting(args, &cwd, &mut io::stdout(), &mut run_report),
            Err(e) => Err(anyhow::Error::new(e).context("cannot read the working directory")),
        },
        Command::Cache(args) => run_cache(args),
        Command::Graph(args) => match std::env::current_dir() {
            Ok(cwd) => run_graph(args, &cwd),
            Err(e) => Err(anyhow::Error::new(e).context("cannot read the working directory")),
        },
    };

    /* Here, never in the command modules: this binary is the one caller a
     * test never goes through (`tests/cli.rs` calls the commands directly
     * precisely to avoid spawning it), which is what keeps `cargo test` from
     * touching the network or this machine's real telemetry id on every run.
     * Sent for failures too, since which stage breaks is the most useful
     * thing telemetry can say. The error itself is never sent, only the
     * stage the command had reached, from a closed set. */
    let (command, stage, cancelled, scaffold, run) = match &cli.command {
        Command::New(_) => (
            "new",
            new_report.stage,
            new_report.cancelled,
            Some(new_report.scaffold),
            None,
        ),
        Command::Run(_) => ("run", run_report.stage, false, None, Some(run_report.run)),
        Command::Cache(_) => ("cache", ErrorKind::Cache, false, None, None),
        Command::Graph(_) => ("graph", ErrorKind::Graph, false, None, None),
    };
    let (exit, error_kind, error_code) = match &result {
        _ if cancelled => (Exit::Cancelled, None, None),
        Ok(()) => (Exit::Ok, None, None),
        Err(error) => (
            Exit::Error,
            Some(error_kind(error, stage)),
            Some(codes::of(error)),
        ),
    };
    let graph = match &cli.command {
        Command::Graph(args) => Some(GraphStats {
            format: match args.format {
                GraphFormat::List => telemetry::GraphFormat::List,
                GraphFormat::Dot => telemetry::GraphFormat::Dot,
            },
        }),
        _ => None,
    };
    let cache = match &cli.command {
        Command::Cache(args) => Some(CacheStats {
            action: match args.command {
                CacheCommand::Stats => CacheAction::Stats,
                CacheCommand::Clear => CacheAction::Clear,
            },
        }),
        _ => None,
    };
    telemetry::record(Outcome {
        command,
        exit,
        error_kind,
        error_code,
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        scaffold,
        run,
        graph,
        cache,
        tools: None,
    });

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

/// A task's own command failing is told apart from culm failing around it;
/// everything else is the stage the command had reached.
fn error_kind(error: &anyhow::Error, stage: ErrorKind) -> ErrorKind {
    match error.downcast_ref::<RunnerError>() {
        Some(RunnerError::TaskFailed { .. }) => ErrorKind::TaskFailed,
        _ if stage == ErrorKind::TaskFailed => ErrorKind::Other,
        _ => stage,
    }
}
