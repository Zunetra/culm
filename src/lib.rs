// === ipile

#![forbid(unsafe_code)]

/// The merge engine.
/// Layers contribute fragments to shared files, and this module resolves them into one deterministic file set.
pub mod merge;
