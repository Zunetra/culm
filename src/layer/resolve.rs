// === Selection resolution

/* What the user picks and what gets scaffolded are not the same list. Picking
 * `frontend/next` means Node and pnpm, which in turn means prettier and
 * eslint, and nobody should be asked to confirm any of that. This is where the
 * short answer becomes the full one. */

use std::collections::{BTreeMap, BTreeSet};

use super::catalog::{Catalog, CatalogEntry};
use super::category::{Cardinality, Category, Shape};
use super::error::LayerError;
use super::manifest::Manifest;
use super::selection::{count_in, shape_of};
use super::validate::validate_selection;
use crate::merge::LayerId;

/// Expands what the user chose into the complete, legal selection.
///
/// `chosen` is what was asked for, qualified as `<category>/<id>`. `opt_outs`
/// suppress layers that would otherwise arrive as defaults, and have no effect
/// on anything named in `chosen`, because an explicit choice outranks a rule
/// about defaults.
///
/// The result is sorted by qualified id and has already passed
/// [`validate_selection`], so a caller cannot forget to check it.
pub fn resolve<'c>(
    catalog: &'c Catalog,
    chosen: &[LayerId],
    opt_outs: &[LayerId],
) -> Result<Vec<&'c CatalogEntry>, LayerError> {
    let opted_out: BTreeSet<&LayerId> = opt_outs.iter().collect();
    let mut selected: BTreeMap<LayerId, &'c CatalogEntry> = BTreeMap::new();

    for id in chosen {
        let entry = lookup(catalog, id)?;
        selected.insert(id.clone(), entry);
    }

    close_over_requirements(catalog, &mut selected)?;
    fill_shape(catalog, &mut selected, &opted_out)?;

    let shape = shape_of(manifests_of(&selected))?;
    add_defaults(catalog, &mut selected, &opted_out, shape);
    fill_runner(catalog, &mut selected, shape)?;

    reject_conflicts(&selected)?;

    let entries: Vec<&'c CatalogEntry> = selected.into_values().collect();
    let manifests: Vec<Manifest> = entries.iter().map(|e| e.manifest.clone()).collect();
    validate_selection(&manifests)?;
    Ok(entries)
}

fn lookup<'c>(catalog: &'c Catalog, id: &LayerId) -> Result<&'c CatalogEntry, LayerError> {
    catalog
        .get(id)
        .ok_or_else(|| LayerError::UnknownLayer { id: id.clone() })
}

/* Only a toolchain is added on a layer's behalf. Node and pnpm follow any JS
 * app and Rust and cargo follow any Rust one, and both answers are forced, so
 * asking would be theatre. Everything else stays an error: `backend/actix`
 * requires `data/sea-orm`, and quietly choosing somebody's database for them
 * is a guess dressed up as an implication. */
fn close_over_requirements<'c>(
    catalog: &'c Catalog,
    selected: &mut BTreeMap<LayerId, &'c CatalogEntry>,
) -> Result<(), LayerError> {
    let mut pending: Vec<LayerId> = selected.keys().cloned().collect();

    while let Some(id) = pending.pop() {
        let entry = selected[&id];
        for required in &entry.manifest.requires {
            let required = LayerId::new(required.as_str());
            if selected.contains_key(&required) {
                continue;
            }
            let candidate = lookup(catalog, &required)?;
            if candidate.manifest.category != Category::Toolchain {
                return Err(LayerError::MissingRequirement {
                    layer: id.clone(),
                    requires: required,
                });
            }
            selected.insert(required.clone(), candidate);
            pending.push(required);
        }
    }
    Ok(())
}

/* The shape has to be settled before defaults, because cardinality depends on
 * it and a default cannot be tested against a free slot until the rule for
 * that slot is known. */
fn fill_shape<'c>(
    catalog: &'c Catalog,
    selected: &mut BTreeMap<LayerId, &'c CatalogEntry>,
    opted_out: &BTreeSet<&LayerId>,
) -> Result<(), LayerError> {
    if count_in(manifests_of(selected), Category::Shape) > 0 {
        return Ok(());
    }
    let default = catalog.by_category(Category::Shape).find(|entry| {
        entry.manifest.default && !opted_out.contains(&entry.manifest.qualified_id())
    });
    match default {
        Some(entry) => {
            selected.insert(entry.manifest.qualified_id(), entry);
            Ok(())
        }
        None => Err(LayerError::ShapeMissing),
    }
}

/* A default arrives only if its own requirements are already met, which is
 * what makes quality tooling follow the language instead of the wizard.
 * prettier requires node-pnpm, so it appears in a JS workspace and stays away
 * from a Rust-only one without a single rule naming either. */
fn add_defaults<'c>(
    catalog: &'c Catalog,
    selected: &mut BTreeMap<LayerId, &'c CatalogEntry>,
    opted_out: &BTreeSet<&LayerId>,
    shape: Shape,
) {
    for entry in catalog.entries() {
        let id = entry.manifest.qualified_id();
        if !entry.manifest.default || selected.contains_key(&id) || opted_out.contains(&id) {
            continue;
        }
        let category = entry.manifest.category;
        match category.cardinality(shape) {
            Cardinality::None => continue,
            Cardinality::ExactlyOne | Cardinality::ZeroOrOne
                if count_in(manifests_of(selected), category) > 0 =>
            {
                continue;
            }
            _ => {}
        }
        let satisfied = entry
            .manifest
            .requires
            .iter()
            .all(|r| selected.contains_key(&LayerId::new(r.as_str())));
        if !satisfied || conflicts_with_selection(entry, selected) {
            continue;
        }
        selected.insert(id, entry);
    }
}

/* Runner is exactly one and no runner layer is a default, because the right
 * answer is not a property of the runner. `nx` requires node-pnpm, so a
 * Rust-only workspace has exactly one legal runner and it is `none`. Decision
 * 3 keeps our own runner out of this until it has earned the default. */
fn fill_runner<'c>(
    catalog: &'c Catalog,
    selected: &mut BTreeMap<LayerId, &'c CatalogEntry>,
    shape: Shape,
) -> Result<(), LayerError> {
    if Category::Runner.cardinality(shape) == Cardinality::None
        || count_in(manifests_of(selected), Category::Runner) > 0
    {
        return Ok(());
    }
    let wanted = if selected.contains_key(&LayerId::new("toolchain/node-pnpm")) {
        LayerId::new("runner/nx")
    } else {
        LayerId::new("runner/none")
    };
    let entry = lookup(catalog, &wanted)?;
    selected.insert(wanted, entry);
    Ok(())
}

/* Checked symmetrically, so one layer declaring the conflict is enough and the
 * pair cannot drift by somebody editing only one of the two manifests. */
fn reject_conflicts(selected: &BTreeMap<LayerId, &CatalogEntry>) -> Result<(), LayerError> {
    for (id, entry) in selected {
        for declared in &entry.manifest.conflicts {
            let other = LayerId::new(declared.as_str());
            if selected.contains_key(&other) {
                let (a, b) = if *id <= other {
                    (id.clone(), other)
                } else {
                    (other, id.clone())
                };
                return Err(LayerError::Conflicting { a, b });
            }
        }
    }
    Ok(())
}

fn conflicts_with_selection(
    candidate: &CatalogEntry,
    selected: &BTreeMap<LayerId, &CatalogEntry>,
) -> bool {
    let id = candidate.manifest.qualified_id();
    candidate
        .manifest
        .conflicts
        .iter()
        .any(|c| selected.contains_key(&LayerId::new(c.as_str())))
        || selected.values().any(|entry| {
            entry
                .manifest
                .conflicts
                .iter()
                .any(|c| LayerId::new(c.as_str()) == id)
        })
}

/// Borrows the manifests out of a selection, so the shared selection helpers can be asked the same questions validation asks.
fn manifests_of<'s>(
    selected: &'s BTreeMap<LayerId, &CatalogEntry>,
) -> impl Iterator<Item = &'s Manifest> {
    selected.values().map(|entry| &entry.manifest)
}

// === Tests

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::super::source::LayerSource;
    use super::*;

    fn shipped() -> Catalog {
        Catalog::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("layers")).unwrap()
    }

    fn ids(entries: &[&CatalogEntry]) -> Vec<String> {
        entries
            .iter()
            .map(|e| e.manifest.qualified_id().as_str().to_owned())
            .collect()
    }

    fn resolved(chosen: &[&str]) -> Vec<String> {
        let catalog = shipped();
        let chosen: Vec<LayerId> = chosen.iter().map(|id| LayerId::new(*id)).collect();
        ids(&resolve(&catalog, &chosen, &[]).unwrap())
    }

    // == Toolchains are derived, never asked

    #[test]
    fn a_javascript_frontend_brings_node_and_pnpm() {
        let selection = resolved(&["frontend/next"]);
        assert!(selection.contains(&"toolchain/node-pnpm".to_owned()));
        assert!(!selection.contains(&"toolchain/rust-cargo".to_owned()));
    }

    #[test]
    fn a_rust_backend_brings_rust_and_cargo() {
        let selection = resolved(&["backend/axum"]);
        assert!(selection.contains(&"toolchain/rust-cargo".to_owned()));
        assert!(!selection.contains(&"toolchain/node-pnpm".to_owned()));
    }

    /* The case the whole design exists for: one workspace, two languages, both
     * toolchains, and each language's tooling following its own. */
    #[test]
    fn a_polyglot_workspace_derives_both_toolchains_and_both_sets_of_tooling() {
        let selection = resolved(&["shape/monorepo", "frontend/next", "backend/axum"]);
        for expected in [
            "toolchain/node-pnpm",
            "toolchain/rust-cargo",
            "quality/prettier",
            "quality/eslint",
            "quality/rustfmt",
            "quality/clippy",
        ] {
            assert!(
                selection.contains(&expected.to_owned()),
                "{expected} missing from {selection:?}"
            );
        }
    }

    // == Quality tooling follows the derived toolchain

    #[test]
    fn a_rust_only_workspace_gets_no_javascript_tooling() {
        let selection = resolved(&["backend/axum"]);
        assert!(selection.contains(&"quality/rustfmt".to_owned()));
        assert!(selection.contains(&"quality/clippy".to_owned()));
        assert!(!selection.contains(&"quality/prettier".to_owned()));
        assert!(!selection.contains(&"quality/eslint".to_owned()));
    }

    #[test]
    fn a_javascript_only_workspace_gets_no_rust_tooling() {
        let selection = resolved(&["frontend/next"]);
        assert!(selection.contains(&"quality/prettier".to_owned()));
        assert!(!selection.contains(&"quality/rustfmt".to_owned()));
    }

    /* Tooling-only is the one shape where nothing implies the toolchain, so it
     * is the explicit answer to "tooling for which ecosystem". */
    #[test]
    fn tooling_only_takes_the_toolchain_it_was_given() {
        let selection = resolved(&["shape/tooling-only", "toolchain/node-pnpm"]);
        assert!(selection.contains(&"quality/prettier".to_owned()));
        assert!(!selection.contains(&"quality/clippy".to_owned()));
        assert_eq!(
            selection
                .iter()
                .filter(|id| id.starts_with("frontend/"))
                .count(),
            0
        );
    }

    // == Defaults and opt-outs

    #[test]
    fn the_shape_defaults_to_a_single_app() {
        assert!(resolved(&["frontend/next"]).contains(&"shape/single-app".to_owned()));
    }

    #[test]
    fn base_is_always_present() {
        assert!(resolved(&["frontend/next"]).contains(&"base/base".to_owned()));
    }

    #[test]
    fn an_opted_out_default_stays_out() {
        let catalog = shipped();
        let selection = ids(&resolve(
            &catalog,
            &[LayerId::new("frontend/next")],
            &[LayerId::new("quality/prettier")],
        )
        .unwrap());
        assert!(!selection.contains(&"quality/prettier".to_owned()));
        assert!(selection.contains(&"quality/eslint".to_owned()));
    }

    /* An explicit choice outranks a rule about defaults, so opting out of
     * something you also asked for is not a contradiction worth an error. */
    #[test]
    fn an_explicit_choice_survives_being_opted_out() {
        let catalog = shipped();
        let selection = ids(&resolve(
            &catalog,
            &[
                LayerId::new("frontend/next"),
                LayerId::new("quality/prettier"),
            ],
            &[LayerId::new("quality/prettier")],
        )
        .unwrap());
        assert!(selection.contains(&"quality/prettier".to_owned()));
    }

    // == The runner

    #[test]
    fn a_javascript_workspace_falls_back_to_nx() {
        assert!(resolved(&["frontend/next"]).contains(&"runner/nx".to_owned()));
    }

    /* nx requires node-pnpm and runner is exactly one, so a Rust-only
     * workspace has one legal answer rather than a choice. */
    #[test]
    fn a_rust_only_workspace_falls_back_to_no_runner() {
        let selection = resolved(&["backend/axum"]);
        assert!(selection.contains(&"runner/none".to_owned()));
        assert!(!selection.contains(&"runner/nx".to_owned()));
    }

    #[test]
    fn an_explicit_runner_is_not_overridden() {
        let selection = resolved(&["frontend/next", "runner/none"]);
        assert!(selection.contains(&"runner/none".to_owned()));
        assert!(!selection.contains(&"runner/nx".to_owned()));
    }

    /* Asking for nx without a JS app is legal and pulls Node in, because nx
     * genuinely needs it. The toolchain is implied by the runner exactly as it
     * would be by a framework. */
    #[test]
    fn choosing_nx_alone_derives_node_and_pnpm() {
        let selection = resolved(&["backend/axum", "runner/nx"]);
        assert!(selection.contains(&"toolchain/node-pnpm".to_owned()));
        assert!(selection.contains(&"quality/prettier".to_owned()));
    }

    // == Failures

    #[test]
    fn an_unknown_layer_is_named() {
        let catalog = shipped();
        assert_eq!(
            resolve(&catalog, &[LayerId::new("frontend/ember")], &[]),
            Err(LayerError::UnknownLayer {
                id: LayerId::new("frontend/ember")
            })
        );
    }

    fn manifest(source: &str, dir: &str) -> CatalogEntry {
        CatalogEntry {
            manifest: Manifest::parse("layer.toml", source).unwrap(),
            source: LayerSource::Disk(PathBuf::from(dir)),
        }
    }

    /* A missing toolchain is filled in. A missing anything else is an error,
     * because choosing somebody's database for them is a guess. */
    #[test]
    fn a_non_toolchain_requirement_is_never_invented() {
        let catalog = Catalog::from_entries(vec![
            manifest(
                "id = \"base\"\ncategory = \"base\"\ndefault = true\n",
                "base",
            ),
            manifest(
                "id = \"single-app\"\ncategory = \"shape\"\ndefault = true\n",
                "shape",
            ),
            manifest("id = \"none\"\ncategory = \"runner\"\n", "runner"),
            manifest("id = \"sea-orm\"\ncategory = \"data\"\n", "data"),
            manifest(
                "id = \"actix\"\ncategory = \"backend\"\nrequires = [\"data/sea-orm\"]\n",
                "backend",
            ),
        ])
        .unwrap();
        assert_eq!(
            resolve(&catalog, &[LayerId::new("backend/actix")], &[]),
            Err(LayerError::MissingRequirement {
                layer: LayerId::new("backend/actix"),
                requires: LayerId::new("data/sea-orm"),
            })
        );
    }

    #[test]
    fn a_declared_conflict_is_rejected_in_either_direction() {
        let entries = vec![
            manifest(
                "id = \"base\"\ncategory = \"base\"\ndefault = true\n",
                "base",
            ),
            manifest(
                "id = \"monorepo\"\ncategory = \"shape\"\ndefault = true\n",
                "shape",
            ),
            manifest("id = \"none\"\ncategory = \"runner\"\n", "runner"),
            manifest(
                "id = \"one\"\ncategory = \"infra\"\nconflicts = [\"infra/two\"]\n",
                "infra-one",
            ),
            manifest("id = \"two\"\ncategory = \"infra\"\n", "infra-two"),
        ];
        let catalog = Catalog::from_entries(entries).unwrap();
        let expected = Err(LayerError::Conflicting {
            a: LayerId::new("infra/one"),
            b: LayerId::new("infra/two"),
        });
        assert_eq!(
            resolve(
                &catalog,
                &[LayerId::new("infra/one"), LayerId::new("infra/two")],
                &[]
            ),
            expected
        );
        assert_eq!(
            resolve(
                &catalog,
                &[LayerId::new("infra/two"), LayerId::new("infra/one")],
                &[]
            ),
            expected
        );
    }

    /* Resolution completes a selection, it does not launder an impossible one.
     * Two frontends in a single app stays an error. */
    #[test]
    fn resolution_does_not_rescue_a_selection_that_breaks_cardinality() {
        let catalog = shipped();
        let chosen = [LayerId::new("frontend/next"), LayerId::new("frontend/nuxt")];
        match resolve(&catalog, &chosen, &[]).unwrap_err() {
            LayerError::CategoryOverfull { category, .. } => {
                assert_eq!(category, Category::Frontend)
            }
            other => panic!("expected an overfull category, got {other:?}"),
        }
    }

    // == Determinism

    #[test]
    fn the_order_layers_are_chosen_in_does_not_change_the_result() {
        let forwards = resolved(&["shape/monorepo", "frontend/next", "backend/axum"]);
        let backwards = resolved(&["backend/axum", "frontend/next", "shape/monorepo"]);
        assert_eq!(forwards, backwards);
    }

    #[test]
    fn the_result_is_sorted_by_qualified_id() {
        let selection = resolved(&["shape/monorepo", "frontend/next", "backend/axum"]);
        let mut sorted = selection.clone();
        sorted.sort();
        assert_eq!(selection, sorted);
    }
}
