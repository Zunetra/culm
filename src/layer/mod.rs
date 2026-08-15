// === Layer format

/* Split by concern rather than kept in one file: what a category is, what a
 * manifest holds, and whether a selection is legal are three separate
 * questions, and only the last one depends on the other two. */

pub mod catalog;
pub mod category;
pub mod error;
pub mod manifest;
pub mod resolve;
pub mod selection;
pub mod source;
pub mod validate;

pub use catalog::{Catalog, CatalogEntry};
pub use category::{ALL_CATEGORIES, Cardinality, Category, Shape};
pub use error::LayerError;
pub use manifest::{FileSpec, InstallSpec, Manifest, MergeSpec};
pub use resolve::resolve;
pub use selection::{count_in, names_in, shape_of};
pub use source::LayerSource;
pub use validate::validate_selection;
