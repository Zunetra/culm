// === Scaffolding a workspace end to end

use std::fs;
use std::path::{Path, PathBuf};

use ipile::layer::{Manifest, validate_selection};
use ipile::merge::FileSet;
use ipile::scaffold::{CopyPlan, ScaffoldError, Variables, write};

// == Helpers

fn layers_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("layers")
}

fn layer_dir(category: &str, id: &str) -> PathBuf {
    layers_root().join(category).join(id)
}

fn load_layer(category: &str, id: &str) -> Manifest {
    let path = layer_dir(category, id).join("layer.toml");
    let source =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    Manifest::parse(&path.display().to_string(), &source)
        .unwrap_or_else(|e| panic!("{} does not parse: {e}", path.display()))
}

/// Selects layers, validates, merges, plans, and writes into `root`.
/// This is the whole pipeline a scaffold runs, which is the point: the pieces are tested apart elsewhere, and this proves they fit together.
fn scaffold(root: &Path, selection: &[(&str, &str)], project: &str) {
    let manifests: Vec<Manifest> = selection
        .iter()
        .map(|(category, id)| load_layer(category, id))
        .collect();
    validate_selection(&manifests).expect("the selection is legal");

    let layers: Vec<_> = manifests
        .iter()
        .map(|manifest| manifest.to_layer().expect("manifest converts to a layer"))
        .collect();
    let merged = FileSet::build(&layers)
        .expect("the selection merges cleanly")
        .render();

    let variables = Variables::new().with("project_name", project);
    let planned: Vec<(&Manifest, PathBuf)> = manifests
        .iter()
        .zip(selection.iter())
        .map(|(manifest, (category, id))| (manifest, layer_dir(category, id)))
        .collect();
    let plan = CopyPlan::build(planned, &variables).expect("the copy plan is buildable");

    write(root, &plan, &merged).expect("the workspace writes");
}

// == The pipeline produces a real directory

#[test]
fn a_javascript_tooling_workspace_lands_on_disk() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = dir.path();

    scaffold(
        root,
        &[
            ("base", "base"),
            ("shape", "tooling-only"),
            ("runner", "none"),
            ("toolchain", "node-pnpm"),
            ("quality", "prettier"),
            ("quality", "eslint"),
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
            ("base", "base"),
            ("shape", "single-app"),
            ("runner", "none"),
            ("toolchain", "rust-cargo"),
            ("backend", "axum"),
        ],
        "klassivox",
    );

    /* The axum layer writes into src/bin/, so the copier has to create parent
     * directories that no layer declared. */
    let nested: Vec<PathBuf> = fs::read_dir(root.join("src").join("bin"))
        .expect("src/bin is created")
        .map(|entry| entry.expect("a directory entry").path())
        .collect();
    assert!(!nested.is_empty(), "the axum layer writes a binary source");

    let gitignore = fs::read_to_string(root.join(".gitignore")).expect("a .gitignore is written");
    assert!(gitignore.contains("/target"));
    assert!(
        !root.join("package.json").exists(),
        "a Rust-only workspace must not invent a package.json"
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
    let planned = vec![(&manifest, layer_dir("quality", "prettier"))];

    match CopyPlan::build(planned, &Variables::new()) {
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
    let planned = vec![(&manifest, layer_dir("quality", "prettier"))];

    match CopyPlan::build(planned, &Variables::new()) {
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
        ("base", "base"),
        ("shape", "tooling-only"),
        ("runner", "none"),
        ("toolchain", "node-pnpm"),
        ("quality", "prettier"),
        ("quality", "eslint"),
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
