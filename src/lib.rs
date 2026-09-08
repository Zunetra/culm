// === culm

#![forbid(unsafe_code)]

/// The startup banner.
/// Prints nothing when stdout is not a terminal, so a scripted run and a captured one stay clean.
pub mod banner;

/// The commands, as functions.
/// `main.rs` parses arguments and calls in here, so every command is reachable from a test without a subprocess.
pub mod cli;

/// Running the package manager in the generated workspace.
/// Decision 5 keeps versions out of templates, which only works if something later asks the package manager to write them.
pub mod install;

/// The layer format.
/// Parses `layer.toml`, and enforces each category's cardinality before a selection reaches the merge engine.
pub mod layer;

/// Named selections of layers, shipped in the binary.
/// A preset is a starting point rather than a template of its own: it resolves to layers and then takes the ordinary path.
pub mod preset;

/// The task runner.
/// Parses `culm.toml`, builds the dependency graph, and runs tasks in topological order. r0.1: sequential, no caching.
pub mod runner;

/// Copying a layer's own files into the generated workspace.
/// Shared files are the merge engine's business; this handles the files one layer alone owns.
pub mod scaffold;

/// The merge engine.
/// Layers contribute fragments to shared files, and this module resolves them into one deterministic file set.
pub mod merge;

/// Anonymous, opt-out usage reporting.
/// Decision 9 is LOCKED: this is the one module that owns what leaves the machine, and no other module talks to the network.
pub mod telemetry;

/// The interactive picker `culm new` falls back to when no `--layer` or `--preset` is given on a real terminal.
/// Builds the same `Vec<LayerId>` a flag-driven run would, then hands off to the ordinary path in `cli::new`, so nothing downstream knows a human was involved.
pub mod wizard;
