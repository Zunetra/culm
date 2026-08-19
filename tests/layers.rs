// === The shipped layers

use std::path::{Path, PathBuf};

use culm::layer::{Catalog, CatalogEntry, Manifest, validate_selection};
use culm::merge::{FileSet, LayerId};

// == Discovery

/// The `layers/` directory of this repository.
fn layers_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("layers")
}

/// Every shipped layer, discovered by walking the tree.
/// Walking rather than listing is the point: a layer added tomorrow is covered by these tests without anybody remembering to register it.
fn shipped() -> Catalog {
    Catalog::load(&layers_root()).unwrap_or_else(|e| panic!("the shipped layers do not load: {e}"))
}

/// Every entry, owned, so a test can iterate without holding the catalog open.
fn entries() -> Vec<CatalogEntry> {
    shipped().entries().cloned().collect()
}

/// Loads a manifest by its category and id, for the tests that need a named layer.
fn load_layer(category: &str, id: &str) -> Manifest {
    let id = LayerId::new(format!("{category}/{id}"));
    shipped()
        .get(&id)
        .unwrap_or_else(|| panic!("{id} is not a shipped layer"))
        .manifest
        .clone()
}

// == Every shipped manifest is well formed

#[test]
fn every_shipped_manifest_parses() {
    let shipped = entries();
    assert!(
        !shipped.is_empty(),
        "no layer.toml found under {}",
        layers_root().display()
    );
    for entry in &shipped {
        let manifest = &entry.manifest;
        /* to_layer is where a json-deep value with a non-table root is caught,
         * so parsing alone would not prove the manifest is usable. */
        manifest.to_layer().unwrap_or_else(|e| {
            panic!("{} does not convert to a layer: {e}", entry.dir().display())
        });
    }
}

#[test]
fn every_manifest_id_matches_its_directory_name() {
    for entry in entries() {
        let directory = entry
            .dir()
            .file_name()
            .expect("a layer.toml always sits inside its layer directory")
            .to_string_lossy()
            .into_owned();
        let manifest = &entry.manifest;
        assert_eq!(
            manifest.id,
            directory,
            "{} declares id '{}' but lives in '{directory}'",
            entry.dir().display(),
            manifest.id
        );
    }
}

#[test]
fn every_manifest_sits_in_its_category_directory() {
    for entry in entries() {
        let category = entry
            .dir()
            .parent()
            .and_then(Path::file_name)
            .expect("a layer directory always sits inside a category directory")
            .to_string_lossy()
            .into_owned();
        let manifest = &entry.manifest;
        assert_eq!(
            manifest.category.as_str(),
            category,
            "{} declares category '{}' but lives under '{category}'",
            entry.dir().display(),
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
    for entry in entries() {
        for install in &entry.manifest.installs {
            for package in &install.packages {
                assert!(
                    !carries_a_version(package),
                    "{} pins '{package}', but package managers write versions, not layers",
                    entry.dir().display()
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
        load_layer("quality", "eslint"),
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
    for entry in entries() {
        let manifest = &entry.manifest;
        for spec in &manifest.files {
            assert!(
                entry.source.read(&spec.from).is_some(),
                "{} declares {} which does not exist",
                entry.dir().display(),
                spec.from.display()
            );
        }
    }
}

/// Every quality layer must require a toolchain, since a linter with no ecosystem is meaningless.
#[test]
fn every_quality_layer_requires_a_toolchain() {
    for entry in entries() {
        let manifest = &entry.manifest;
        if manifest.category.as_str() != "quality" {
            continue;
        }
        assert!(
            manifest
                .requires
                .iter()
                .any(|r| r.starts_with("toolchain/")),
            "{} is a quality layer and must require a toolchain",
            entry.dir().display()
        );
    }
}

/// Every shipped frontend against every other, so a newly added one cannot quietly collide with an existing one.
#[test]
fn every_pair_of_frontends_merges_cleanly() {
    let frontends: Vec<Manifest> = entries()
        .into_iter()
        .map(|entry| entry.manifest)
        .filter(|manifest| manifest.category.as_str() == "frontend")
        .collect();
    assert!(frontends.len() >= 2, "expected several frontend layers");

    let base = load_layer("base", "base");
    let shape = load_layer("shape", "monorepo");
    let runner = load_layer("runner", "nx");
    let toolchain = load_layer("toolchain", "node-pnpm");
    let eslint = load_layer("quality", "eslint");

    for (index, a) in frontends.iter().enumerate() {
        for b in &frontends[index + 1..] {
            let selection = vec![
                base.clone(),
                shape.clone(),
                runner.clone(),
                toolchain.clone(),
                eslint.clone(),
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
    let backends: Vec<Manifest> = entries()
        .into_iter()
        .map(|entry| entry.manifest)
        .filter(|manifest| manifest.category.as_str() == "backend")
        .collect();
    assert!(backends.len() >= 2, "expected several backend layers");

    let base = load_layer("base", "base");
    let shape = load_layer("shape", "monorepo");
    let runner = load_layer("runner", "nx");
    let node = load_layer("toolchain", "node-pnpm");
    let eslint = load_layer("quality", "eslint");
    let rust = load_layer("toolchain", "rust-cargo");

    for (index, a) in backends.iter().enumerate() {
        for b in &backends[index + 1..] {
            let selection = vec![
                base.clone(),
                shape.clone(),
                runner.clone(),
                node.clone(),
                eslint.clone(),
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
        load_layer("quality", "eslint"),
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
    for entry in entries() {
        let manifest = &entry.manifest;
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
            entry.dir().display()
        );
        assert!(
            !manifest.installs.is_empty(),
            "{} installs nothing, so it cannot produce a working app",
            entry.dir().display()
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
    let eslint = load_layer("quality", "eslint").to_layer().unwrap();
    let astro = load_layer("frontend", "astro").to_layer().unwrap();
    let svelte = load_layer("frontend", "svelte").to_layer().unwrap();

    let base = load_layer("base", "base").to_layer().unwrap();

    let forward =
        FileSet::build([&base, &shape, &runner, &toolchain, &eslint, &astro, &svelte]).unwrap();
    let backward =
        FileSet::build([&svelte, &astro, &eslint, &toolchain, &runner, &shape, &base]).unwrap();
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
    let ids: Vec<String> = entries()
        .into_iter()
        .map(|entry| entry.manifest.id)
        .collect();

    for entry in entries() {
        let manifest = &entry.manifest;
        /* Only the merge and install payloads are searched, because those are
         * the parts that reach the generated workspace. `requires` naming
         * another layer is the mechanism working, not a special case, and a
         * comment never leaves the manifest. */
        let payload = format!("{:?}{:?}", manifest.merges, manifest.installs);
        for other in &ids {
            /* `requires` holds qualified ids and `ids` holds bare ones, so the
             * exemption has to compare across that boundary. Comparing the two
             * directly never matched, which meant a declared requirement did
             * not actually exempt anything and the escape hatch this test
             * documents was closed. */
            let required = manifest
                .requires
                .iter()
                .any(|id| id == other || id.ends_with(&format!("/{other}")));
            if *other == manifest.id || required {
                continue;
            }
            assert!(
                !payload.contains(other.as_str()),
                "{} mentions the sibling layer '{other}' in its payload",
                entry.dir().display()
            );
        }
        assert!(
            !payload.to_lowercase().contains("culm"),
            "{} emits a reference to culm, but a generated workspace must run without it",
            entry.dir().display()
        );
    }
}

// == Linting, which is the half that fails silently

/// Extensions ESLint's flat config does not look at unless something claims them.
/// A flat config entry covers .js, .mjs and .cjs and nothing else, so a workspace of TypeScript reports success having examined only its own config file.
const UNCLAIMED: [&str; 5] = [".ts", ".tsx", ".vue", ".svelte", ".astro"];

/// The layers whose sources ESLint would skip, paired with the fragment each one must ship.
fn layers_needing_a_fragment() -> Vec<(CatalogEntry, String)> {
    entries()
        .into_iter()
        .filter(|entry| {
            entry.manifest.files.iter().any(|spec| {
                let from = spec.from.to_string_lossy();
                UNCLAIMED.iter().any(|ext| from.ends_with(ext))
            })
        })
        .map(|entry| {
            let fragment = format!("eslint.{}.config.mjs", entry.manifest.id);
            (entry, fragment)
        })
        .collect()
}

/// A layer whose file types ESLint ignores has to claim them, or `lint` passes having checked nothing.
/// The silent green is the whole problem: a workspace can be entirely unlinted and still report success, which is worse than shipping no linter at all.
#[test]
fn every_layer_with_unclaimed_sources_ships_an_eslint_fragment() {
    let needing = layers_needing_a_fragment();
    assert!(
        needing.len() >= 6,
        "expected every frontend to need a fragment, found {}",
        needing.len()
    );

    for (entry, fragment) in needing {
        let shipped = entry
            .manifest
            .files
            .iter()
            .any(|spec| spec.to.to_string_lossy() == fragment);
        assert!(
            shipped,
            "{} ships sources ESLint does not look at by default but no '{fragment}', so they would be silently unlinted",
            entry.dir().display()
        );
    }
}

/// Whether a quoted string in a fragment is a path it claims rather than a package or a rule name.
fn is_a_path(literal: &str) -> bool {
    literal.contains('*')
        || [".ts", ".tsx", ".mjs", ".json"]
            .iter()
            .any(|ext| literal.ends_with(ext))
}

/// Every glob a fragment claims must land inside that layer's own output.
/// Disjoint globs are what let the root config load fragments in any order: two layers claiming the same path would make the result depend on which was read first, which is the guarantee the whole merge engine is built on.
#[test]
fn every_eslint_fragment_claims_only_its_own_paths() {
    for (entry, fragment) in layers_needing_a_fragment() {
        let spec = entry
            .manifest
            .files
            .iter()
            .find(|spec| spec.to.to_string_lossy() == fragment)
            .unwrap_or_else(|| panic!("{} declares no {fragment}", entry.dir().display()));
        let body = entry.source.read(&spec.from).unwrap_or_else(|| {
            panic!(
                "{} does not carry {}",
                entry.dir().display(),
                spec.from.display()
            )
        });

        let written: Vec<String> = entry
            .manifest
            .files
            .iter()
            .map(|spec| spec.to.to_string_lossy().into_owned())
            .collect();

        for literal in body.split('"').skip(1).step_by(2) {
            if !is_a_path(literal) {
                continue;
            }
            let prefix = literal.split('*').next().unwrap_or(literal);
            assert!(
                written.iter().any(|to| to.starts_with(prefix)),
                "{} claims '{literal}' but writes nothing under '{prefix}', so it reaches another layer's files",
                entry.dir().display()
            );
        }
    }
}

/// A scoped tsconfig that nothing invokes is dead configuration.
/// Every frontend shipped one for months while no script ever ran a type check, which is how a removed compiler option survived in a template until a real build was attempted.
#[test]
fn every_layer_shipping_a_tsconfig_invokes_it() {
    for entry in entries() {
        let ships_one = entry
            .manifest
            .files
            .iter()
            .any(|spec| spec.to.to_string_lossy().ends_with("tsconfig.json"));
        if !ships_one {
            continue;
        }
        let payload = format!("{:?}", entry.manifest.merges);
        assert!(
            payload.contains("typecheck:") || payload.contains("check:"),
            "{} ships a tsconfig but no script type checks against it",
            entry.dir().display()
        );
    }
}

/// Both formatters read the ignores the framework layers already declare.
/// Restating a framework's build directory inside a quality layer would make that layer aware of a framework; forgetting to restate it is invisible, because `lint` and `format:check` pass on a clean checkout and fail forever after the first build.
#[test]
fn the_quality_layers_inherit_the_gitignore() {
    let prettier = load_layer("quality", "prettier");
    let scripts = format!("{:?}", prettier.merges);
    assert!(
        scripts.matches("--ignore-path .gitignore").count() >= 2,
        "both prettier scripts must inherit .gitignore, not just one"
    );

    let eslint = shipped()
        .get(&LayerId::new("quality/eslint"))
        .expect("the eslint layer is shipped")
        .clone();
    let spec = eslint
        .manifest
        .files
        .iter()
        .find(|spec| spec.to.to_string_lossy() == "eslint.config.mjs")
        .expect("the eslint layer ships its config");
    let body = eslint
        .source
        .read(&spec.from)
        .expect("the eslint layer carries its config");
    assert!(
        body.contains("includeIgnoreFile"),
        "eslint.config.mjs must inherit .gitignore rather than restate build directories"
    );
}

// == The gate that needs a network

/// Every layer that emits an application is exercised by the weekly template run.
/// `cargo test` cannot reach a network, so nothing here proves a template builds; that job is the only thing that does. Five frontends reached the quality bar and none were added to its matrix, which left two thirds of the shipped layers unproven while the suite stayed green.
#[test]
fn every_layer_that_emits_an_application_is_in_the_template_workflow() {
    let workflow = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(".github")
        .join("workflows")
        .join("templates.yml");
    let body = std::fs::read_to_string(&workflow)
        .unwrap_or_else(|e| panic!("{} does not read: {e}", workflow.display()));

    for entry in entries() {
        let category = entry.manifest.category.as_str();
        if category != "frontend" && category != "backend" {
            continue;
        }
        if entry.manifest.files.is_empty() {
            continue;
        }
        let qualified = format!("{category}/{}", entry.manifest.id);
        assert!(
            body.contains(&format!("layer: {qualified}")),
            "'{qualified}' emits an application but the template workflow never builds it, so nothing proves it installs"
        );
    }
}
