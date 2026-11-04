// === The interactive wizard

/* Deliberately last, per docs/CONTEXT.md: flags have to work anyway, and a
 * tool whose only entry point is prompts cannot be scripted. This module
 * never becomes a second way to scaffold - it only builds the same
 * `Vec<LayerId>` a `--layer` list would, then hands off to the ordinary path
 * in `cli::new`, which is what keeps the merge engine, validation, and
 * install logic from needing to know a human was involved.
 *
 * Order follows docs/DESIGN.md section 2.4, minus the project name step:
 * that is still a required positional argument, so by the time this runs it
 * is already known. Toolchain is asked only under `tooling-only`, the one
 * shape where nothing else implies it (decision 4); every other shape never
 * asks, the same as a flag-driven run never names a toolchain directly. Data
 * follows backend, and offers only the data layers whose toolchain an app
 * already chosen brings: `sqlx` beside a Rust service, `prisma` and
 * `drizzle` beside a Node one. Nothing chosen offers nothing, and the screen
 * is skipped. */

mod picker;

use std::collections::HashSet;

use anyhow::{Context, Result};

use crate::layer::{Cardinality, Catalog, CatalogEntry, Category, Shape, shape_of};
use crate::merge::LayerId;

/// How a wizard session ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Every screen was answered; the qualified layer ids chosen.
    Chosen(Vec<String>),
    /// Esc/q on the screen for this category.
    /// Named so telemetry can say where people give up, never what they had picked.
    Cancelled(Category),
}

/// Runs the wizard. Never partially commits: cancelling at the last screen
/// throws away every earlier answer, the same as cancelling at the first,
/// since nothing is scaffolded until the whole selection exists.
pub fn run(catalog: &Catalog) -> Result<Outcome> {
    let mut terminal = ratatui::init();
    let outcome = run_inner(&mut terminal, catalog);
    ratatui::restore();
    outcome
}

fn run_inner(terminal: &mut ratatui::DefaultTerminal, catalog: &Catalog) -> Result<Outcome> {
    let mut chosen: Vec<String> = Vec::new();

    let Some((shape_id, shape)) = pick_shape(terminal, catalog)? else {
        return Ok(Outcome::Cancelled(Category::Shape));
    };
    chosen.push(shape_id);

    for category in [Category::Frontend, Category::Backend] {
        let Some(mut picked) = pick_category(terminal, catalog, category, shape, "app", &|_| true)?
        else {
            return Ok(Outcome::Cancelled(category));
        };
        chosen.append(&mut picked);
    }

    let toolchains = implied_toolchains(catalog, &chosen);
    let offer_data = |entry: &CatalogEntry| data_fits(entry, &toolchains);
    let Some(mut picked) = pick_category(
        terminal,
        catalog,
        Category::Data,
        shape,
        "database",
        &offer_data,
    )?
    else {
        return Ok(Outcome::Cancelled(Category::Data));
    };
    chosen.append(&mut picked);

    if matches!(shape, Shape::ToolingOnly) {
        let Some(mut picked) = pick_category(
            terminal,
            catalog,
            Category::Toolchain,
            shape,
            "toolchain",
            &|_| true,
        )?
        else {
            return Ok(Outcome::Cancelled(Category::Toolchain));
        };
        chosen.append(&mut picked);
    }

    for (category, noun) in [
        (Category::Quality, "quality tool"),
        (Category::Infra, "infra layer"),
        (Category::Editor, "editor"),
    ] {
        let Some(mut picked) = pick_category(terminal, catalog, category, shape, noun, &|_| true)?
        else {
            return Ok(Outcome::Cancelled(category));
        };
        chosen.append(&mut picked);
    }

    let Some(runner) = pick_single_category(terminal, catalog, Category::Runner, shape, "runner")?
    else {
        return Ok(Outcome::Cancelled(Category::Runner));
    };
    chosen.push(runner);

    Ok(Outcome::Chosen(chosen))
}

/// Every shape is a real choice: it is `ExactlyOne` in every case, so a plain
/// single-select list is the whole interaction, no "none" option needed.
/// Returns both the qualified id (to push onto the chosen list) and the
/// resolved `Shape` (to decide every later screen's cardinality), derived
/// through the same `shape_of` the flag-driven path uses rather than a
/// hand-rolled parse of the id string.
fn pick_shape(
    terminal: &mut ratatui::DefaultTerminal,
    catalog: &Catalog,
) -> Result<Option<(String, Shape)>> {
    let mut entries: Vec<&crate::layer::CatalogEntry> =
        catalog.by_category(Category::Shape).collect();
    entries.sort_by_key(|entry| entry.manifest.qualified_id().to_string());
    let labels: Vec<String> = entries
        .iter()
        .map(|entry| entry.manifest.qualified_id().to_string())
        .collect();
    let default = entries
        .iter()
        .position(|entry| entry.manifest.default)
        .unwrap_or(0);

    let Some(index) = picker::pick_single(terminal, "Shape", &labels, default)? else {
        return Ok(None);
    };
    let shape = shape_of(std::iter::once(&entries[index].manifest))
        .context("the shape layer just chosen does not name a shape culm recognises")?;
    Ok(Some((labels[index].clone(), shape)))
}

/// One category, at whatever cardinality the shape gives it. `ZeroOrOne` and
/// `None` both render as a single-select with a "(none)" option rather than
/// forcing a choice; `ZeroOrMore`/`OneOrMore` render as a checklist. `None`
/// itself is not reachable here in practice (frontend/backend/data under
/// `tooling-only`), since the caller does not offer a category the shape has
/// already ruled out.
fn pick_category(
    terminal: &mut ratatui::DefaultTerminal,
    catalog: &Catalog,
    category: Category,
    shape: Shape,
    noun: &str,
    offer: &dyn Fn(&CatalogEntry) -> bool,
) -> Result<Option<Vec<String>>> {
    let cardinality = category.cardinality(shape);
    if matches!(cardinality, Cardinality::None) {
        return Ok(Some(Vec::new()));
    }

    let mut entries: Vec<&CatalogEntry> = catalog
        .by_category(category)
        .filter(|entry| offer(entry))
        .collect();
    /* A screen with nothing to choose is only a keypress to get past. */
    if entries.is_empty() {
        return Ok(Some(Vec::new()));
    }
    entries.sort_by_key(|entry| entry.manifest.qualified_id().to_string());
    let labels: Vec<String> = entries
        .iter()
        .map(|entry| entry.manifest.qualified_id().to_string())
        .collect();

    match cardinality {
        Cardinality::ZeroOrOne => {
            let mut options = vec![format!("(no {noun})")];
            options.extend(labels.iter().cloned());
            let title = format!("{category} ({noun}, optional)");
            let Some(index) = picker::pick_single(terminal, &title, &options, 0)? else {
                return Ok(None);
            };
            Ok(Some(if index == 0 {
                Vec::new()
            } else {
                vec![options[index].clone()]
            }))
        }
        Cardinality::ExactlyOne | Cardinality::OneOrMore | Cardinality::ZeroOrMore => {
            let defaults: Vec<bool> = entries.iter().map(|entry| entry.manifest.default).collect();
            let title = format!("{category} ({noun}{})", plural_hint(cardinality));
            let Some(indices) = picker::pick_multi(terminal, &title, &labels, &defaults)? else {
                return Ok(None);
            };
            if matches!(cardinality, Cardinality::OneOrMore) && indices.is_empty() {
                // Re-ask rather than silently proceed with an illegal empty
                // selection: `tooling-only` requires at least one toolchain,
                // and `validate_selection` would refuse this downstream
                // anyway, just with a less immediate message.
                return pick_category(terminal, catalog, category, shape, noun, offer);
            }
            Ok(Some(
                indices.into_iter().map(|i| labels[i].clone()).collect(),
            ))
        }
        Cardinality::None => unreachable!("handled above"),
    }
}

/// Every toolchain the layers chosen so far require, as qualified ids.
fn implied_toolchains(catalog: &Catalog, chosen: &[String]) -> HashSet<String> {
    let mut toolchains = HashSet::new();
    for id in chosen {
        let Some(entry) = catalog.get(&LayerId::new(id)) else {
            continue;
        };
        for required in &entry.manifest.requires {
            if required.starts_with("toolchain/") {
                toolchains.insert(required.clone());
            }
        }
    }
    return toolchains;
}

/// Whether a data layer belongs beside the apps already chosen.
/// Only its toolchain requirements count: a data layer that also needs a quality layer, such as `eslint`, is still offered, and validation names that layer if it is later unticked.
fn data_fits(entry: &CatalogEntry, toolchains: &HashSet<String>) -> bool {
    return entry
        .manifest
        .requires
        .iter()
        .filter(|required| required.starts_with("toolchain/"))
        .all(|required| toolchains.contains(required));
}

fn plural_hint(cardinality: Cardinality) -> &'static str {
    match cardinality {
        Cardinality::OneOrMore => ", at least one",
        _ => ", any number",
    }
}

/// `Runner` is always `ExactlyOne`, so this is `pick_category` without the
/// "(none)" branch cardinality never takes - `runner/none` is a real,
/// listed choice here, not a synthetic absence.
fn pick_single_category(
    terminal: &mut ratatui::DefaultTerminal,
    catalog: &Catalog,
    category: Category,
    shape: Shape,
    noun: &str,
) -> Result<Option<String>> {
    let mut entries: Vec<&crate::layer::CatalogEntry> = catalog.by_category(category).collect();
    entries.sort_by_key(|entry| entry.manifest.qualified_id().to_string());
    let labels: Vec<String> = entries
        .iter()
        .map(|entry| entry.manifest.qualified_id().to_string())
        .collect();
    let default = entries
        .iter()
        .position(|entry| entry.manifest.default)
        .unwrap_or(0);

    let title = format!("{category} ({noun})");
    let _ = category.cardinality(shape); // asserted by the caller's own choice of category, not re-derived
    Ok(picker::pick_single(terminal, &title, &labels, default)?.map(|i| labels[i].clone()))
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn offered_data(chosen: &[&str]) -> Vec<String> {
        let catalog = Catalog::embedded().unwrap();
        let chosen: Vec<String> = chosen.iter().map(|s| (*s).to_owned()).collect();
        let toolchains = implied_toolchains(&catalog, &chosen);
        let mut offered: Vec<String> = catalog
            .by_category(Category::Data)
            .filter(|entry| data_fits(entry, &toolchains))
            .map(|entry| entry.manifest.qualified_id().to_string())
            .collect();
        offered.sort();
        return offered;
    }

    #[test]
    fn a_rust_service_is_offered_sqlx_only() {
        assert_eq!(
            offered_data(&["shape/single-app", "backend/axum"]),
            vec!["data/sqlx"]
        );
    }

    #[test]
    fn a_node_app_is_offered_the_node_orms_only() {
        assert_eq!(
            offered_data(&["shape/single-app", "frontend/nuxt"]),
            vec!["data/drizzle", "data/prisma"]
        );
    }

    #[test]
    fn a_polyglot_monorepo_is_offered_all_three() {
        assert_eq!(
            offered_data(&["shape/monorepo", "frontend/nuxt", "backend/axum"]),
            vec!["data/drizzle", "data/prisma", "data/sqlx"]
        );
    }

    #[test]
    fn no_app_means_no_database_screen() {
        assert!(offered_data(&["shape/single-app"]).is_empty());
    }
}
