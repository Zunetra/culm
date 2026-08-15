// === The command line

/* The commands live in the library rather than in `main.rs` so they can be
 * driven from a test without spawning a process. A subprocess test is slower,
 * it is flaky under a loaded machine, and it has to guess the binary's path per
 * profile. `main.rs` is left with argument parsing and an exit code. */

pub mod new;

pub use new::{NewArgs, run_new, run_new_to};
