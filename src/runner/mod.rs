// === The runner

/* Split the same way `src/layer/` is: what the config file says, what a task
 * is once resolved, whether the dependencies form a legal order, and how a
 * run is reported are four separate questions. r0.1 covered config, error,
 * exec, graph, and task. r0.2 through r1.0 (docs/02-runner.md section 6) add
 * caching (`crate::cas`), `affected`, `parallel`, and `remote_cache` here,
 * plus the three real backends (`turborepo`, `moon`, `nx`) decision 3
 * requires the scaffolder be able to emit, and `registration`, which writes
 * each backend's root config into a freshly scaffolded workspace. */

pub mod affected;
pub mod config;
pub mod error;
pub mod exec;
pub mod graph;
pub mod moon;
pub mod nx;
pub mod parallel;
pub mod registration;
pub mod remote_cache;
pub mod task;
pub mod turborepo;

pub use config::{RunnerConfig, TaskSpec};
pub use error::RunnerError;
pub use graph::TaskGraph;
pub use registration::write_root_config;
pub use task::{Task, tasks_of};
