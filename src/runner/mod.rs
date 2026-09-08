// === The runner

/* r0.1 only, per docs/02-runner.md section 6: `culm run <task>`, sequential
 * execution respecting `depends_on`, no caching. Split the same way
 * `src/layer/` is: what the config file says, what a task is once resolved,
 * whether the dependencies form a legal order, and how a run is reported are
 * four separate questions. */

pub mod config;
pub mod error;
pub mod exec;
pub mod graph;
pub mod task;

pub use config::{RunnerConfig, TaskSpec};
pub use error::RunnerError;
pub use graph::TaskGraph;
pub use task::{Task, tasks_of};
