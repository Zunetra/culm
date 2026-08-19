// === Scaffolding

/* Split from `merge` on purpose. The merge engine decides what shared files
 * should contain; this decides what private files a layer copies and where they
 * land. Conflating them would put path safety and template interpolation inside
 * the one module whose correctness property is byte-for-byte determinism. */

pub mod error;
pub mod interpolate;
pub mod layout;
pub mod plan;
pub mod write;

pub use error::ScaffoldError;
pub use interpolate::Variables;
pub use layout::variables;
pub use plan::{CopyPlan, PlannedFile};
pub use write::write;
