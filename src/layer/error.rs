// === Errors

use std::path::PathBuf;

use thiserror::Error;

use super::category::{Category, Shape};
use crate::merge::LayerId;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LayerError {
    #[error("{path}: {message}")]
    Malformed { path: String, message: String },

    #[error("layer `{layer}` declares a json-deep merge whose value is not a table")]
    NonTableMergeValue { layer: LayerId },

    #[error("category `{category}` requires at least one layer, but the selection has none")]
    CategoryEmpty { category: Category },

    #[error("shape `{shape}` allows no {category} layers, but the selection has {}", .names.join(", "))]
    CategoryForbidden {
        category: Category,
        shape: Shape,
        names: Vec<String>,
    },

    #[error("category `{category}` allows at most one layer, but the selection has {chosen}: {}", .names.join(", "))]
    CategoryOverfull {
        category: Category,
        chosen: usize,
        names: Vec<String>,
    },

    #[error("layer id `{id}` appears more than once in the selection")]
    DuplicateId { id: LayerId },

    #[error("the selection has no shape layer, so it is neither a single app nor a monorepo")]
    ShapeMissing,

    #[error("`{id}` is not a known shape, expected `single-app`, `monorepo`, or `tooling-only`")]
    UnknownShape { id: String },

    #[error("a single-app workspace needs at least one frontend or backend layer")]
    SingleAppHasNoApp,

    #[error("`{id}` is not a layer in the catalog")]
    UnknownLayer { id: LayerId },

    #[error("layer `{layer}` requires `{requires}`, which nothing in the selection provides")]
    MissingRequirement { layer: LayerId, requires: LayerId },

    #[error("layers `{a}` and `{b}` conflict and cannot be selected together")]
    Conflicting { a: LayerId, b: LayerId },

    #[error(
        "layer `{layer}` names `{id}` in `{field}`, which is not a qualified `<category>/<id>`"
    )]
    UnqualifiedReference {
        layer: LayerId,
        field: &'static str,
        id: String,
    },

    #[error("{}: {message}", .path.display())]
    Unreadable { path: PathBuf, message: String },
}
