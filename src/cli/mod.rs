// === The command line

/* The commands live in the library rather than in `main.rs` so they can be
 * driven from a test without spawning a process. A subprocess test is slower,
 * it is flaky under a loaded machine, and it has to guess the binary's path per
 * profile. `main.rs` is left with argument parsing and an exit code. */

pub mod cache;
pub mod error;
pub mod graph;
pub mod new;
pub mod run;

pub use cache::{CacheArgs, CacheCommand, run_cache, run_cache_to};
pub use error::CliError;
pub use graph::{GraphArgs, GraphFormat, run_graph, run_graph_to};
pub use new::{NewArgs, NewReport, run_new, run_new_reporting, run_new_to};
pub use run::{RunArgs, RunReport, run_task, run_task_reporting, run_task_to};
