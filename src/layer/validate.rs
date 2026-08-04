// === Selection validation

use std::collections::BTreeMap;

use super::category::{ALL_CATEGORIES, Cardinality, Category, Shape};
use super::error::LayerError;
use super::manifest::Manifest;
use crate::merge::LayerId;

/// Checks a selection against every category's cardinality.
/// This runs before the merge engine, because a selection that breaks a cardinality rule should never reach the point of producing files.
pub fn validate_selection(selection: &[Manifest]) -> Result<(), LayerError> {
    reject_duplicates(selection)?;

    let by_category = group_by_category(selection);
    let shape = resolve_shape(by_category.get(&Category::Shape))?;

    for category in ALL_CATEGORIES {
        let chosen = by_category.get(&category).map_or(0, Vec::len);
        match category.cardinality(shape) {
            Cardinality::None if chosen > 0 => {
                return Err(LayerError::CategoryForbidden {
                    category,
                    shape,
                    names: sorted_names(&by_category, category),
                });
            }
            Cardinality::ExactlyOne | Cardinality::OneOrMore if chosen == 0 => {
                return Err(LayerError::CategoryEmpty { category });
            }
            Cardinality::ExactlyOne | Cardinality::ZeroOrOne if chosen > 1 => {
                return Err(LayerError::CategoryOverfull {
                    category,
                    chosen,
                    names: sorted_names(&by_category, category),
                });
            }
            _ => {}
        }
    }

    /* A single app with neither a frontend nor a backend is not a workspace,
     * it is a bare toolchain. Cardinality alone cannot express this, because
     * the rule spans two categories. */
    if shape == Shape::SingleApp {
        let apps = by_category.get(&Category::Frontend).map_or(0, Vec::len)
            + by_category.get(&Category::Backend).map_or(0, Vec::len);
        if apps == 0 {
            return Err(LayerError::SingleAppHasNoApp);
        }
    }

    Ok(())
}

/* Keyed on the qualified id, so a shape called `none` and a runner called
 * `none` are two different layers rather than a false duplicate. */
fn reject_duplicates(selection: &[Manifest]) -> Result<(), LayerError> {
    let mut seen: BTreeMap<LayerId, usize> = BTreeMap::new();
    for manifest in selection {
        let id = manifest.qualified_id();
        let count = seen.entry(id.clone()).or_insert(0);
        *count += 1;
        if *count > 1 {
            return Err(LayerError::DuplicateId { id });
        }
    }
    Ok(())
}

fn sorted_names(grouped: &BTreeMap<Category, Vec<String>>, category: Category) -> Vec<String> {
    let mut names = grouped.get(&category).cloned().unwrap_or_default();
    names.sort();
    names
}

fn group_by_category(selection: &[Manifest]) -> BTreeMap<Category, Vec<String>> {
    let mut grouped: BTreeMap<Category, Vec<String>> = BTreeMap::new();
    for manifest in selection {
        grouped
            .entry(manifest.category)
            .or_default()
            .push(manifest.id.clone());
    }
    grouped
}

fn resolve_shape(chosen: Option<&Vec<String>>) -> Result<Shape, LayerError> {
    let names = chosen.ok_or(LayerError::ShapeMissing)?;
    if names.len() > 1 {
        let mut names = names.clone();
        names.sort();
        return Err(LayerError::CategoryOverfull {
            category: Category::Shape,
            chosen: names.len(),
            names,
        });
    }
    let id = names.first().ok_or(LayerError::ShapeMissing)?;
    Shape::from_id(id).ok_or_else(|| LayerError::UnknownShape { id: id.clone() })
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(id: &str, category: &str) -> Manifest {
        Manifest::parse(
            "layer.toml",
            &format!("id = \"{id}\"\ncategory = \"{category}\"\n"),
        )
        .unwrap()
    }

    #[test]
    fn a_single_app_rejects_two_frontends() {
        let selection = vec![
            manifest("base", "base"),
            manifest("single-app", "shape"),
            manifest("nx", "runner"),
            manifest("nuxt", "frontend"),
            manifest("next", "frontend"),
        ];
        match validate_selection(&selection).unwrap_err() {
            LayerError::CategoryOverfull {
                category, names, ..
            } => {
                assert_eq!(category, Category::Frontend);
                assert_eq!(names, vec!["next", "nuxt"]);
            }
            other => panic!("expected an overfull category, got {other:?}"),
        }
    }

    /* The case that drove the shape-dependent model: a Nuxt site beside a React
     * admin panel, or a Rust service beside a Node BFF, is the reason someone
     * reaches for a monorepo. */
    #[test]
    fn a_monorepo_accepts_several_frontends_and_backends() {
        let selection = vec![
            manifest("base", "base"),
            manifest("monorepo", "shape"),
            manifest("nx", "runner"),
            manifest("nuxt", "frontend"),
            manifest("react-vite", "frontend"),
            manifest("axum", "backend"),
            manifest("nestjs", "backend"),
        ];
        assert!(validate_selection(&selection).is_ok());
    }

    #[test]
    fn a_runner_is_always_required() {
        let selection = vec![
            manifest("base", "base"),
            manifest("monorepo", "shape"),
            manifest("nuxt", "frontend"),
        ];
        assert_eq!(
            validate_selection(&selection),
            Err(LayerError::CategoryEmpty {
                category: Category::Runner
            })
        );
    }

    #[test]
    fn a_single_app_with_no_app_is_rejected() {
        let selection = vec![
            manifest("base", "base"),
            manifest("single-app", "shape"),
            manifest("nx", "runner"),
        ];
        assert_eq!(
            validate_selection(&selection),
            Err(LayerError::SingleAppHasNoApp)
        );
    }

    #[test]
    fn a_selection_without_a_shape_is_rejected() {
        let selection = vec![manifest("nuxt", "frontend")];
        assert_eq!(
            validate_selection(&selection),
            Err(LayerError::ShapeMissing)
        );
    }

    #[test]
    fn an_unknown_shape_is_rejected() {
        let selection = vec![manifest("polyrepo", "shape"), manifest("nx", "runner")];
        assert_eq!(
            validate_selection(&selection),
            Err(LayerError::UnknownShape {
                id: "polyrepo".to_owned()
            })
        );
    }

    /* Tooling only, dropped onto a repository that already exists. The
     * toolchain is the answer to "tooling for which ecosystem", since with no
     * framework there is nothing left to imply it. */
    #[test]
    fn tooling_only_takes_tooling_without_any_app() {
        let js = vec![
            manifest("base", "base"),
            manifest("tooling-only", "shape"),
            manifest("none", "runner"),
            manifest("node-pnpm", "toolchain"),
            manifest("prettier", "quality"),
            manifest("eslint", "quality"),
        ];
        assert!(validate_selection(&js).is_ok());

        let rust = vec![
            manifest("base", "base"),
            manifest("tooling-only", "shape"),
            manifest("none", "runner"),
            manifest("rust-cargo", "toolchain"),
            manifest("rustfmt", "quality"),
            manifest("clippy", "quality"),
        ];
        assert!(validate_selection(&rust).is_ok());
    }

    #[test]
    fn tooling_only_rejects_a_frontend() {
        let selection = vec![
            manifest("base", "base"),
            manifest("tooling-only", "shape"),
            manifest("none", "runner"),
            manifest("node-pnpm", "toolchain"),
            manifest("nuxt", "frontend"),
        ];
        assert_eq!(
            validate_selection(&selection),
            Err(LayerError::CategoryForbidden {
                category: Category::Frontend,
                shape: Shape::ToolingOnly,
                names: vec!["nuxt".to_owned()],
            })
        );
    }

    #[test]
    fn tooling_only_needs_a_toolchain_to_know_which_tooling() {
        let selection = vec![
            manifest("base", "base"),
            manifest("tooling-only", "shape"),
            manifest("none", "runner"),
        ];
        assert_eq!(
            validate_selection(&selection),
            Err(LayerError::CategoryEmpty {
                category: Category::Toolchain
            })
        );
    }

    /* Every workspace gets one, so that a selection is never left without the
     * ignores that belong to no framework. */
    #[test]
    fn a_selection_without_a_base_layer_is_rejected() {
        let selection = vec![
            manifest("monorepo", "shape"),
            manifest("nx", "runner"),
            manifest("nuxt", "frontend"),
        ];
        assert_eq!(
            validate_selection(&selection),
            Err(LayerError::CategoryEmpty {
                category: Category::Base
            })
        );
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let selection = vec![manifest("nuxt", "frontend"), manifest("nuxt", "frontend")];
        assert_eq!(
            validate_selection(&selection),
            Err(LayerError::DuplicateId {
                id: LayerId::new("frontend/nuxt")
            })
        );
    }

    /* `none` is a legitimate id in more than one category, so identity has to
     * be qualified or the two would collide as a false duplicate. */
    #[test]
    fn the_same_bare_id_in_two_categories_is_not_a_duplicate() {
        let selection = vec![
            manifest("base", "base"),
            manifest("tooling-only", "shape"),
            manifest("none", "runner"),
            manifest("none", "infra"),
            manifest("node-pnpm", "toolchain"),
        ];
        assert!(validate_selection(&selection).is_ok());
    }
}
