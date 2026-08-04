// === Layer format

/* Split by concern rather than kept in one file: what a category is, what a
 * manifest holds, and whether a selection is legal are three separate
 * questions, and only the last one depends on the other two. */

pub mod category;
pub mod error;
pub mod manifest;
pub mod validate;

pub use category::{ALL_CATEGORIES, Cardinality, Category, Shape};
pub use error::LayerError;
pub use manifest::{FileSpec, InstallSpec, Manifest, MergeSpec};
pub use validate::validate_selection;
