// === culm

#![forbid(unsafe_code)]

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

/// Copying a layer's own files into the generated workspace.
/// Shared files are the merge engine's business; this handles the files one layer alone owns.
pub mod scaffold;

/// The merge engine.
/// Layers contribute fragments to shared files, and this module resolves them into one deterministic file set.
pub mod merge;
