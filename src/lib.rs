// === ipile

#![forbid(unsafe_code)]

/// The layer format.
/// Parses `layer.toml`, and enforces each category's cardinality before a selection reaches the merge engine.
pub mod layer;

/// Copying a layer's own files into the generated workspace.
/// Shared files are the merge engine's business; this handles the files one layer alone owns.
pub mod scaffold;

/// The merge engine.
/// Layers contribute fragments to shared files, and this module resolves them into one deterministic file set.
pub mod merge;
