// === The shipped layers

use std::fs;
use std::path::{Path, PathBuf};

use ipile::layer::{Manifest, validate_selection};
use ipile::merge::FileSet;

// == Discovery

/// The `layers/` directory of this repository.
fn layers_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("layers")
}

/// Every `layer.toml` under `layers/`, found by walking the tree.
/// Walking rather than listing is the point: a layer added tomorrow is covered by these tests without anybody remembering to register it.
fn discover_manifest_paths() -> Vec<PathBuf> {
    let mut found = Vec::new();
    collect(&layers_root(), &mut found);
    found.sort();
    found
}

fn collect(dir: &Path, found: &mut Vec<PathBuf>) {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("cannot read directory entry").path();
        if path.is_dir() {
            collect(&path, found);
        } else if path.file_name().is_some_and(|name| name == "layer.toml") {
            found.push(path);
        }
    }
}

/// Loads one manifest, failing the test with the offending path rather than a bare parse error.
fn load(path: &Path) -> Manifest {
    let source =
        fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    Manifest::parse(&path.display().to_string(), &source)
        .unwrap_or_else(|e| panic!("{} does not parse: {e}", path.display()))
}

/// Loads a manifest by its category and id, for the tests that need a named layer.
fn load_layer(category: &str, id: &str) -> Manifest {
    load(&layers_root().join(category).join(id).join("layer.toml"))
}

// == Every shipped manifest is well formed

#[test]
fn every_shipped_manifest_parses() {
    let paths = discover_manifest_paths();
    assert!(
        !paths.is_empty(),
        "no layer.toml found under {}",
        layers_root().display()
    );
    for path in &paths {
        let manifest = load(path);
        /* to_layer is where a json-deep value with a non-table root is caught,
         * so parsing alone would not prove the manifest is usable. */
        manifest
            .to_layer()
            .unwrap_or_else(|e| panic!("{} does not convert to a layer: {e}", path.display()));
    }
}

#[test]
fn every_manifest_id_matches_its_directory_name() {
    for path in discover_manifest_paths() {
        let directory = path
            .parent()
            .and_then(Path::file_name)
            .expect("a layer.toml always sits inside its layer directory")
            .to_string_lossy()
            .into_owned();
        let manifest = load(&path);
        assert_eq!(
            manifest.id,
            directory,
            "{} declares id '{}' but lives in '{directory}'",
            path.display(),
            manifest.id
        );
    }
}

#[test]
fn every_manifest_sits_in_its_category_directory() {
    for path in discover_manifest_paths() {
        let category = path
            .parent()
            .and_then(Path::parent)
            .and_then(Path::file_name)
            .expect("a layer directory always sits inside a category directory")
            .to_string_lossy()
            .into_owned();
        let manifest = load(&path);
        assert_eq!(
            manifest.category.as_str(),
            category,
            "{} declares category '{}' but lives under '{category}'",
            path.display(),
            manifest.category
        );
    }
}

// == Decision 5, enforced mechanically

/// Whether a package string carries a version of any kind.
/// Scoped names such as `@vitejs/plugin-vue` are not versions, so only an `@` followed by a digit counts.
fn carries_a_version(package: &str) -> bool {
    let at_a_digit = package
        .as_bytes()
        .windows(2)
        .any(|pair| pair[0] == b'@' && pair[1].is_ascii_digit());
    let range_marker = package.contains(['^', '~', '>', '<', '=', '*']);
    at_a_digit || range_marker
}

#[test]
fn no_shipped_package_carries_a_version() {
    for path in discover_manifest_paths() {
        for install in load(&path).installs {
            for package in install.packages {
                assert!(
                    !carries_a_version(&package),
                    "{} pins '{package}', but package managers write versions, not layers",
                    path.display()
                );
            }
        }
    }
}

#[test]
fn the_version_check_can_actually_fail() {
    assert!(carries_a_version("nuxt@3"));
    assert!(carries_a_version("vite@^5.0.0"));
    assert!(carries_a_version("typescript@5.4"));
    assert!(carries_a_version("react@>=18"));
    assert!(!carries_a_version("nuxt"));
    assert!(!carries_a_version("@vitejs/plugin-vue"));
    assert!(!carries_a_version("@sveltejs/adapter-auto"));
}

// == Several frontends in one workspace

/// The whole reason the scripts are namespaced.
/// Two frontends in one monorepo both write to `package.json`, and differing scalars at the same key are a hard error, so a shared `scripts.dev` would fail here.
#[test]
fn a_monorepo_takes_two_frontends_without_a_merge_conflict() {
    let selection = vec![
        load_layer("base", "base"),
        load_layer("shape", "monorepo"),
        load_layer("runner", "nx"),
        load_layer("toolchain", "node-pnpm"),
        load_layer("frontend", "nuxt"),
        load_layer("frontend", "react-vite"),
    ];
    validate_selection(&selection).expect("the selection breaks no cardinality rule");

    let layers: Vec<_> = selection
        .iter()
        .map(|manifest| manifest.to_layer().expect("manifest converts to a layer"))
        .collect();
    let set = FileSet::build(&layers).expect("two frontends merge cleanly");

    let rendered = set.render();
    let package_json = rendered
        .get(&PathBuf::from("package.json"))
        .expect("both frontends contribute to package.json");
    assert!(package_json.contains("dev:nuxt"));
    assert!(package_json.contains("dev:react-vite"));

    let gitignore = rendered
        .get(&PathBuf::from(".gitignore"))
        .expect("both frontends contribute to .gitignore");
    assert!(gitignore.contains(".nuxt/"));
    assert!(gitignore.contains("# layer:frontend/react-vite"));
}

// == Tooling only

/// `shape = none` drops tooling onto a repository that already exists.
/// The toolchain is what decides which tooling, since with no framework there is nothing left to imply it.
#[test]
fn a_none_shape_yields_javascript_tooling() {
    let selection = vec![
        load_layer("base", "base"),
        load_layer("shape", "tooling-only"),
        load_layer("runner", "none"),
        load_layer("toolchain", "node-pnpm"),
        load_layer("quality", "prettier"),
        load_layer("quality", "eslint"),
    ];
    validate_selection(&selection).expect("tooling only is a legal selection");

    let layers: Vec<_> = selection
        .iter()
        .map(|manifest| manifest.to_layer().expect("manifest converts to a layer"))
        .collect();
    let rendered = FileSet::build(&layers)
        .expect("the tooling layers merge cleanly")
        .render();

    let package_json = rendered
        .get(&PathBuf::from("package.json"))
        .expect("the JS tooling contributes to package.json");
    assert!(package_json.contains("prettier --write"));
    assert!(package_json.contains("eslint ."));
}

#[test]
fn a_none_shape_yields_rust_tooling() {
    let selection = vec![
        load_layer("base", "base"),
        load_layer("shape", "tooling-only"),
        load_layer("runner", "none"),
        load_layer("toolchain", "rust-cargo"),
        load_layer("quality", "rustfmt"),
        load_layer("quality", "clippy"),
    ];
    validate_selection(&selection).expect("tooling only is a legal selection");

    let layers: Vec<_> = selection
        .iter()
        .map(|manifest| manifest.to_layer().expect("manifest converts to a layer"))
        .collect();
    let rendered = FileSet::build(&layers)
        .expect("the tooling layers merge cleanly")
        .render();

    let gitignore = rendered
        .get(&PathBuf::from(".gitignore"))
        .expect("the Rust toolchain contributes to .gitignore");
    assert!(gitignore.contains("/target"));
    /* A Rust-only selection must not invent a package.json. */
    assert!(!rendered.contains_key(&PathBuf::from("package.json")));
}

/// Every `[[files]]` entry must point at a template that exists.
/// A manifest naming a file nobody wrote fails at scaffold time, in front of a user, rather than here.
#[test]
fn every_declared_template_file_exists() {
    for path in discover_manifest_paths() {
        let manifest = load(&path);
        let dir = path
            .parent()
            .expect("a manifest always sits in a directory");
        for spec in &manifest.files {
            let template = dir.join(&spec.from);
            assert!(
                template.is_file(),
                "{} declares {} which does not exist",
                path.display(),
                spec.from.display()
            );
        }
    }
}

/// Every quality layer must require a toolchain, since a linter with no ecosystem is meaningless.
#[test]
fn every_quality_layer_requires_a_toolchain() {
    for path in discover_manifest_paths() {
        let manifest = load(&path);
        if manifest.category.as_str() != "quality" {
            continue;
        }
        assert!(
            manifest
                .requires
                .iter()
                .any(|r| r.starts_with("toolchain/")),
            "{} is a quality layer and must require a toolchain",
            path.display()
        );
    }
}

/// Every shipped frontend against every other, so a newly added one cannot quietly collide with an existing one.
#[test]
fn every_pair_of_frontends_merges_cleanly() {
    let frontends: Vec<Manifest> = discover_manifest_paths()
        .iter()
        .map(|path| load(path))
        .filter(|manifest| manifest.category.as_str() == "frontend")
        .collect();
    assert!(frontends.len() >= 2, "expected several frontend layers");

    let base = load_layer("base", "base");
    let shape = load_layer("shape", "monorepo");
    let runner = load_layer("runner", "nx");
    let toolchain = load_layer("toolchain", "node-pnpm");

    for (index, a) in frontends.iter().enumerate() {
        for b in &frontends[index + 1..] {
            let selection = vec![
                base.clone(),
                shape.clone(),
                runner.clone(),
                toolchain.clone(),
                a.clone(),
                b.clone(),
            ];
            validate_selection(&selection).unwrap_or_else(|e| {
                panic!("'{}' with '{}' is an illegal selection: {e}", a.id, b.id)
            });
            let layers: Vec<_> = selection
                .iter()
                .map(|manifest| manifest.to_layer().expect("manifest converts to a layer"))
                .collect();
            FileSet::build(&layers)
                .unwrap_or_else(|e| panic!("'{}' and '{}' collide: {e}", a.id, b.id));
        }
    }
}

// == Several backends in one workspace

/// Every shipped backend against every other, so a newly added one cannot quietly collide with an existing one.
/// Both toolchains are selected because a pair can straddle the two ecosystems, which is the case a monorepo exists to serve.
#[test]
fn every_pair_of_backends_merges_cleanly() {
    let backends: Vec<Manifest> = discover_manifest_paths()
        .iter()
        .map(|path| load(path))
        .filter(|manifest| manifest.category.as_str() == "backend")
        .collect();
    assert!(backends.len() >= 2, "expected several backend layers");

    let base = load_layer("base", "base");
    let shape = load_layer("shape", "monorepo");
    let runner = load_layer("runner", "nx");
    let node = load_layer("toolchain", "node-pnpm");
    let rust = load_layer("toolchain", "rust-cargo");

    for (index, a) in backends.iter().enumerate() {
        for b in &backends[index + 1..] {
            let selection = vec![
                base.clone(),
                shape.clone(),
                runner.clone(),
                node.clone(),
                rust.clone(),
                a.clone(),
                b.clone(),
            ];
            validate_selection(&selection).unwrap_or_else(|e| {
                panic!("'{}' with '{}' is an illegal selection: {e}", a.id, b.id)
            });
            let layers: Vec<_> = selection
                .iter()
                .map(|manifest| manifest.to_layer().expect("manifest converts to a layer"))
                .collect();
            FileSet::build(&layers)
                .unwrap_or_else(|e| panic!("'{}' and '{}' collide: {e}", a.id, b.id));
        }
    }
}

/// The selection a real project actually makes, end to end.
/// A frontend, a backend, and a data layer all write to package.json at once, which is where an unnamespaced script key would surface.
#[test]
fn a_full_stack_monorepo_validates_and_merges() {
    let selection = vec![
        load_layer("base", "base"),
        load_layer("shape", "monorepo"),
        load_layer("runner", "nx"),
        load_layer("toolchain", "node-pnpm"),
        load_layer("frontend", "nuxt"),
        load_layer("backend", "nestjs"),
        load_layer("data", "prisma"),
    ];
    validate_selection(&selection).expect("a full stack monorepo breaks no cardinality rule");

    let layers: Vec<_> = selection
        .iter()
        .map(|manifest| manifest.to_layer().expect("manifest converts to a layer"))
        .collect();
    let rendered = FileSet::build(&layers)
        .expect("a frontend, a backend, and a data layer merge cleanly")
        .render();

    let package_json = rendered
        .get(&PathBuf::from("package.json"))
        .expect("all three contribute to package.json");
    assert!(package_json.contains("dev:nuxt"));
    assert!(package_json.contains("dev:nestjs"));
    assert!(package_json.contains("generate:prisma"));

    let env_example = rendered
        .get(&PathBuf::from(".env.example"))
        .expect("the data layer documents its connection string");
    assert!(env_example.contains("DATABASE_URL"));
}

/// A Rust service and its data layer, with no JavaScript anywhere in the selection.
/// The sharp part is the negative: a Rust workspace has no package.json, so a layer inventing one would leave a project nobody can run.
#[test]
fn a_rust_service_selection_produces_no_package_json() {
    let selection = vec![
        load_layer("base", "base"),
        load_layer("shape", "single-app"),
        load_layer("runner", "none"),
        load_layer("toolchain", "rust-cargo"),
        load_layer("backend", "axum"),
        load_layer("data", "sqlx"),
    ];
    validate_selection(&selection).expect("a Rust service breaks no cardinality rule");

    let layers: Vec<_> = selection
        .iter()
        .map(|manifest| manifest.to_layer().expect("manifest converts to a layer"))
        .collect();
    let rendered = FileSet::build(&layers)
        .expect("the Rust layers merge cleanly")
        .render();

    assert!(!rendered.contains_key(&PathBuf::from("package.json")));

    let gitignore = rendered
        .get(&PathBuf::from(".gitignore"))
        .expect("every workspace gets a .gitignore");
    assert!(gitignore.contains("/target"));

    let env_example = rendered
        .get(&PathBuf::from(".env.example"))
        .expect("the data layer documents its connection string");
    assert!(env_example.contains("DATABASE_URL"));
}

/// A backend or a data layer with no ecosystem cannot be installed, so both categories must name a toolchain.
#[test]
fn every_backend_and_data_layer_requires_a_toolchain() {
    let mut checked = 0;
    for path in discover_manifest_paths() {
        let manifest = load(&path);
        let category = manifest.category.as_str();
        if category != "backend" && category != "data" {
            continue;
        }
        assert!(
            manifest
                .requires
                .iter()
                .any(|r| r.starts_with("toolchain/")),
            "{} is a {category} layer and must require a toolchain",
            path.display()
        );
        assert!(
            !manifest.installs.is_empty(),
            "{} installs nothing, so it cannot produce a working app",
            path.display()
        );
        checked += 1;
    }
    assert!(
        checked >= 8,
        "expected the backend and data layers to exist"
    );
}

/// Output must not depend on the order the user happened to pick layers in.
#[test]
fn frontend_selection_order_does_not_change_the_output() {
    let shape = load_layer("shape", "monorepo").to_layer().unwrap();
    let runner = load_layer("runner", "nx").to_layer().unwrap();
    let toolchain = load_layer("toolchain", "node-pnpm").to_layer().unwrap();
    let astro = load_layer("frontend", "astro").to_layer().unwrap();
    let svelte = load_layer("frontend", "svelte").to_layer().unwrap();

    let base = load_layer("base", "base").to_layer().unwrap();

    let forward = FileSet::build([&base, &shape, &runner, &toolchain, &astro, &svelte]).unwrap();
    let backward = FileSet::build([&svelte, &astro, &toolchain, &runner, &shape, &base]).unwrap();
    assert_eq!(forward.render_all(), backward.render_all());
}

/// Every workspace gets a .gitignore and a .env.example, whatever else is selected.
/// A Rust tooling-only selection is the sharp case: without the base layer its .gitignore would hold nothing but /target.
#[test]
fn every_workspace_gets_the_base_files() {
    let selection = vec![
        load_layer("base", "base"),
        load_layer("shape", "tooling-only"),
        load_layer("runner", "none"),
        load_layer("toolchain", "rust-cargo"),
    ];
    validate_selection(&selection).expect("tooling only is a legal selection");

    let layers: Vec<_> = selection
        .iter()
        .map(|manifest| manifest.to_layer().expect("manifest converts to a layer"))
        .collect();
    let rendered = FileSet::build(&layers)
        .expect("the base layer merges")
        .render();

    let gitignore = rendered
        .get(&PathBuf::from(".gitignore"))
        .expect("every workspace gets a .gitignore");
    assert!(gitignore.contains(".DS_Store"));
    assert!(gitignore.contains("/target"));

    let env_example = rendered
        .get(&PathBuf::from(".env.example"))
        .expect("every workspace gets a .env.example");
    assert!(env_example.contains("APP_ENV"));
}

// == The six frontends are all present and shaped alike

#[test]
fn every_frontend_declares_the_node_toolchain_and_the_two_shared_files() {
    let expected = ["astro", "next", "nuxt", "react-vite", "svelte", "vue-vite"];
    for id in expected {
        let manifest = load_layer("frontend", id);
        assert_eq!(manifest.category.as_str(), "frontend");
        assert!(!manifest.default, "'{id}' must not be selected by default");
        assert!(
            manifest.requires.iter().any(|r| r == "toolchain/node-pnpm"),
            "'{id}' is a Node project and must require toolchain/node-pnpm"
        );
        assert!(
            !manifest.installs.is_empty(),
            "'{id}' installs nothing, so it cannot produce a working app"
        );

        let targets: Vec<String> = manifest
            .to_layer()
            .unwrap()
            .merges
            .keys()
            .map(|path| path.display().to_string())
            .collect();
        assert!(
            targets.contains(&"package.json".to_owned()),
            "'{id}' must contribute scripts"
        );
        assert!(
            targets.contains(&".gitignore".to_owned()),
            "'{id}' must ignore its build output"
        );
    }
}

/// No layer may name another layer outside its own `requires`, which is where combination authoring would start.
#[test]
fn no_layer_special_cases_a_sibling() {
    let ids: Vec<String> = discover_manifest_paths()
        .iter()
        .map(|path| load(path).id)
        .collect();

    for path in discover_manifest_paths() {
        let manifest = load(&path);
        /* Only the merge and install payloads are searched, because those are
         * the parts that reach the generated workspace. `requires` naming
         * another layer is the mechanism working, not a special case, and a
         * comment never leaves the manifest. */
        let payload = format!("{:?}{:?}", manifest.merges, manifest.installs);
        for other in &ids {
            if *other == manifest.id || manifest.requires.contains(other) {
                continue;
            }
            assert!(
                !payload.contains(other.as_str()),
                "{} mentions the sibling layer '{other}' in its payload",
                path.display()
            );
        }
        assert!(
            !payload.to_lowercase().contains("ipile"),
            "{} emits a reference to ipile, but a generated workspace must run without it",
            path.display()
        );
    }
}
