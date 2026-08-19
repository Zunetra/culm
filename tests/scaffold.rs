// === Scaffolding a workspace end to end

use std::fs;
use std::path::{Path, PathBuf};

use culm::layer::{Catalog, CatalogEntry, LayerSource, Manifest, resolve, validate_selection};
use culm::merge::{FileSet, LayerId};
use culm::scaffold::{CopyPlan, ScaffoldError, Variables, write};

// == Helpers

fn layers_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("layers")
}

fn layer_source(category: &str, id: &str) -> LayerSource {
    LayerSource::Disk(layers_root().join(category).join(id))
}

fn catalog() -> Catalog {
    Catalog::load(&layers_root()).expect("the shipped layers load")
}

/// Merges, plans, and writes an already-settled selection into `root`.
/// This is the whole pipeline a scaffold runs, which is the point: the pieces are tested apart elsewhere, and this proves they fit together.
fn scaffold_entries(root: &Path, entries: &[&CatalogEntry], project: &str) {
    /* The same order the CLI uses: settle the layout from the whole selection,
     * interpolate each manifest with its own variables, and only then merge.
     * A helper that took a shortcut here would be testing a pipeline nobody
     * runs. */
    let variables = culm::scaffold::variables(entries, project);
    let manifests: Vec<Manifest> = entries
        .iter()
        .zip(&variables)
        .map(|(entry, vars)| entry.interpolated(vars).expect("the manifest interpolates"))
        .collect();
    validate_selection(&manifests).expect("the selection is legal");

    let layers: Vec<_> = manifests
        .iter()
        .map(|manifest| manifest.to_layer().expect("manifest converts to a layer"))
        .collect();
    let file_set = FileSet::build(&layers).expect("the selection merges cleanly");
    let modes = file_set.modes();
    let merged = file_set.render();

    let planned: Vec<(&Manifest, &LayerSource, &Variables)> = manifests
        .iter()
        .zip(entries.iter())
        .zip(&variables)
        .map(|((manifest, entry), vars)| (manifest, &entry.source, vars))
        .collect();
    let plan = CopyPlan::build(planned).expect("the copy plan is buildable");

    write(root, &plan, &merged, &modes).expect("the workspace writes");
}

/// Scaffolds an exact selection, named by qualified id, with nothing added.
fn scaffold(root: &Path, selection: &[&str], project: &str) {
    let catalog = catalog();
    let entries: Vec<&CatalogEntry> = selection
        .iter()
        .map(|id| {
            catalog
                .get(&LayerId::new(*id))
                .unwrap_or_else(|| panic!("{id} is not a shipped layer"))
        })
        .collect();
    scaffold_entries(root, &entries, project);
}

/// Scaffolds what a user's choice resolves to, defaults and derived toolchains included.
fn scaffold_resolved(root: &Path, chosen: &[&str], project: &str) {
    let catalog = catalog();
    let chosen: Vec<LayerId> = chosen.iter().map(|id| LayerId::new(*id)).collect();
    let entries = resolve(&catalog, &chosen, &[]).expect("the selection resolves");
    scaffold_entries(root, &entries, project);
}

// == The pipeline produces a real directory

#[test]
fn a_javascript_tooling_workspace_lands_on_disk() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = dir.path();

    scaffold(
        root,
        &[
            "base/base",
            "shape/tooling-only",
            "runner/none",
            "toolchain/node-pnpm",
            "quality/prettier",
            "quality/eslint",
        ],
        "klassivox",
    );

    /* Copied files, owned by one layer each. */
    assert!(root.join(".prettierrc").is_file());
    assert!(root.join(".prettierignore").is_file());
    assert!(root.join("eslint.config.mjs").is_file());

    /* Merged files, contributed to by several. */
    let gitignore = fs::read_to_string(root.join(".gitignore")).expect("a .gitignore is written");
    assert!(gitignore.contains(".DS_Store"));
    assert!(gitignore.contains("node_modules/"));

    let package_json =
        fs::read_to_string(root.join("package.json")).expect("a package.json is written");
    assert!(package_json.contains("prettier --write"));

    assert!(root.join(".env.example").is_file());
}

#[test]
fn a_rust_workspace_writes_nested_directories() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = dir.path();

    scaffold(
        root,
        &[
            "base/base",
            "shape/single-app",
            "runner/none",
            "toolchain/rust-cargo",
            "backend/axum",
        ],
        "klassivox",
    );

    /* The axum layer writes into crates/axum/src/, so the copier has to create
     * parent directories that no layer declared. */
    let nested: Vec<PathBuf> = fs::read_dir(root.join("crates").join("axum").join("src"))
        .expect("crates/axum/src is created")
        .map(|entry| entry.expect("a directory entry").path())
        .collect();
    assert!(!nested.is_empty(), "the axum layer writes a binary source");

    /* A single Rust app is a one member workspace, so the root manifest is the
     * same shape it would be in a monorepo. */
    let cargo = fs::read_to_string(root.join("Cargo.toml")).expect("a root Cargo.toml is written");
    assert!(cargo.contains("[workspace]"));
    assert!(cargo.contains("\"crates/axum\""));
    assert!(
        root.join("crates")
            .join("axum")
            .join("Cargo.toml")
            .is_file(),
        "the member crate carries its own manifest"
    );

    let gitignore = fs::read_to_string(root.join(".gitignore")).expect("a .gitignore is written");
    assert!(gitignore.contains("/target"));
    assert!(
        !root.join("package.json").exists(),
        "a Rust-only workspace must not invent a package.json"
    );
}

/* The pipeline as a user actually drives it: name two frameworks, and the
 * toolchains, quality tooling, base, shape, and runner arrive on their own. */
#[test]
fn a_resolved_polyglot_workspace_lands_on_disk() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = dir.path();

    scaffold_resolved(
        root,
        &["shape/monorepo", "frontend/next", "backend/axum"],
        "klassivox",
    );

    /* Both languages' tooling, neither of them asked for. */
    assert!(root.join(".prettierrc").is_file());
    assert!(root.join("rustfmt.toml").is_file() || root.join(".rustfmt.toml").is_file());

    let gitignore = fs::read_to_string(root.join(".gitignore")).expect("a .gitignore is written");
    assert!(gitignore.contains("node_modules/"));
    assert!(gitignore.contains("/target"));

    assert!(
        root.join("package.json").is_file(),
        "the derived node toolchain owns package.json"
    );

    let cargo = fs::read_to_string(root.join("Cargo.toml")).expect("a root Cargo.toml is written");
    assert!(
        cargo.contains("\"crates/axum\""),
        "the layer that adds a crate is the layer that lists it as a member"
    );
}

// == Safety and determinism

/// A layer is data, and data must not be able to write outside the directory it was pointed at.
#[test]
fn a_destination_escaping_the_root_is_refused() {
    let source = r#"
id = "hostile"
category = "quality"
requires = ["toolchain/node-pnpm"]

[[files]]
from = "files/.prettierrc"
to = "../../../.ssh/authorized_keys"
"#;
    let manifest = Manifest::parse("hostile/layer.toml", source).expect("it parses");
    let prettier = layer_source("quality", "prettier");
    let vars = Variables::new();
    let planned = vec![(&manifest, &prettier, &vars)];

    match CopyPlan::build(planned) {
        Err(ScaffoldError::EscapesRoot { layer, .. }) => {
            assert_eq!(layer.as_str(), "quality/hostile");
        }
        other => panic!("expected the escape to be refused, got {other:?}"),
    }
}

#[test]
fn a_missing_template_is_named_with_its_layer() {
    let source = r#"
id = "typo"
category = "quality"

[[files]]
from = "files/does-not-exist.toml"
to = "whatever.toml"
"#;
    let manifest = Manifest::parse("typo/layer.toml", source).expect("it parses");
    let prettier = layer_source("quality", "prettier");
    let vars = Variables::new();
    let planned = vec![(&manifest, &prettier, &vars)];

    match CopyPlan::build(planned) {
        Err(ScaffoldError::TemplateMissing { layer, .. }) => {
            assert_eq!(layer.as_str(), "quality/typo");
        }
        other => panic!("expected the missing template to be named, got {other:?}"),
    }
}

/// Selection order must not change what lands on disk, the same property the merge engine guarantees.
#[test]
fn selection_order_does_not_change_the_workspace() {
    let forward = tempfile::tempdir().expect("a temp dir");
    let backward = tempfile::tempdir().expect("a temp dir");

    let mut selection = vec![
        "base/base",
        "shape/tooling-only",
        "runner/none",
        "toolchain/node-pnpm",
        "quality/prettier",
        "quality/eslint",
    ];
    scaffold(forward.path(), &selection, "klassivox");
    selection.reverse();
    scaffold(backward.path(), &selection, "klassivox");

    for name in [".gitignore", "package.json", ".env.example", ".prettierrc"] {
        let a = fs::read_to_string(forward.path().join(name)).expect("written");
        let b = fs::read_to_string(backward.path().join(name)).expect("written");
        assert_eq!(a, b, "{name} depends on selection order");
    }
}

// == Hooks reach disk able to run

/// A merged hook is written with the executable bit, end to end.
/// git skips a hook that is not executable without printing anything, so a hook that reaches disk unexecutable is a feature that silently does not exist. The bit travels from the strategy through `FileSet::modes` to `write`, and only this test covers the whole path.
#[cfg(unix)]
#[test]
fn a_merged_hook_lands_executable() {
    use std::os::unix::fs::PermissionsExt;

    use culm::merge::{Contribution, Layer};

    let dir = tempfile::tempdir().expect("a temp dir");
    let root = dir.path();

    let formatting = Layer::new("quality/prettier").with_merge(
        ".husky/pre-commit",
        Contribution::HookScript {
            order: 10,
            body: vec!["pnpm lint-staged".to_owned()],
        },
    );
    let rust = Layer::new("quality/rustfmt").with_merge(
        ".husky/pre-commit",
        Contribution::HookScript {
            order: 20,
            body: vec!["cargo fmt --check".to_owned()],
        },
    );

    let set = FileSet::build([&formatting, &rust]).expect("the two hooks merge");
    let merged = set.render();
    let modes = set.modes();
    write(root, &CopyPlan::default(), &merged, &modes).expect("the workspace writes");

    let hook = root.join(".husky").join("pre-commit");
    let body = fs::read_to_string(&hook).expect("the hook was written");
    assert!(body.starts_with("#!/bin/sh\n"));
    assert!(
        body.find("pnpm lint-staged").unwrap() < body.find("cargo fmt --check").unwrap(),
        "the declared order decides which body runs first"
    );

    let mode = fs::metadata(&hook)
        .expect("the hook is on disk")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o755, "a hook git will not run is not a hook");
}
