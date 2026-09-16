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

/// Every entry, owned, with the scaffold's variables already substituted into its manifest.
///
/// Interpolated rather than raw because a raw manifest is not a thing culm ever
/// merges or writes: paths and script names carry placeholders until a
/// selection settles the layout. Rendered through the real layout logic over
/// the whole catalog rather than a table restated here, so these tests cannot
/// drift from what a scaffold actually does. The catalog holds many
/// applications, so the variables come out namespaced, which is the case these
/// tests are about; the bare form arises only when a workspace holds exactly
/// one application, and that is covered end to end in `tests/cli.rs`.
///
/// Nothing an identity check reads is touched by this: `id`, `category`, and
/// the install lists carry no placeholders.
fn entries() -> Vec<CatalogEntry> {
    let catalog = shipped();
    let raw: Vec<&CatalogEntry> = catalog.entries().collect();
    let variables = culm::scaffold::variables(&raw, "demo");
    raw.iter()
        .zip(&variables)
        .map(|(entry, vars)| CatalogEntry {
            manifest: entry
                .interpolated(vars)
                .unwrap_or_else(|e| panic!("{} does not interpolate: {e}", entry.dir().display())),
            source: entry.source.clone(),
        })
        .collect()
}

/// Loads a manifest by its category and id, for the tests that need a named layer.
fn load_layer(category: &str, id: &str) -> Manifest {
    let wanted = LayerId::new(format!("{category}/{id}"));
    entries()
        .into_iter()
        .map(|entry| entry.manifest)
        .find(|manifest| manifest.qualified_id() == wanted)
        .unwrap_or_else(|| panic!("{wanted} is not a shipped layer"))
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

/// The deliberate, documented exceptions to decision 5, each recorded in
/// docs/CONTEXT.md alongside the upstream incompatibility that forces it:
/// - `typescript@<6`: TypeScript jumped straight from the 5.x line to a 7.x
///   native-compiler rewrite with no 6.x in between, and
///   `typescript-eslint@8.69.0` refuses to run against it; an unranged
///   `typescript` resolves straight into the broken major.
/// - `vitest@<5`: `@nuxt/test-utils` only declares a peer on `^4.0.2`, and
///   vitest 5 changed the `vitest/environments` export shape the "nuxt" test
///   environment it registers depends on.
/// - `prisma@<8`: the `prisma` CLI's `latest` dist-tag is a `8.0.0-rc`
///   pre-release of an entirely restructured CLI with no `generate`/
///   `migrate`/`studio` commands, while `@prisma/client`'s `latest` is still
///   on the 7.x line; an unranged `prisma` pulls the broken pre-release CLI.
///
/// Lift each cap, and its matching CONTEXT.md entry, once the upstream
/// package it names resolves the incompatibility.
const DOCUMENTED_VERSION_EXCEPTIONS: &[&str] = &["typescript@<6", "vitest@<5", "prisma@<8"];

#[test]
fn no_shipped_package_carries_a_version() {
    for entry in entries() {
        for install in &entry.manifest.installs {
            for package in &install.packages {
                if DOCUMENTED_VERSION_EXCEPTIONS.contains(&package.as_str()) {
                    continue;
                }
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

/// The whole reason each app gets its own real package.json (decision 23).
/// A frontend and a backend in one monorepo each own a distinct package.json
/// under their own directory, so neither's unnamespaced `scripts.dev` ever
/// meets the other's at the same key.
#[test]
fn a_monorepo_takes_two_apps_without_a_merge_conflict() {
    let selection = vec![
        load_layer("base", "base"),
        load_layer("shape", "monorepo"),
        load_layer("runner", "nx"),
        load_layer("toolchain", "node-pnpm"),
        load_layer("quality", "eslint"),
        load_layer("frontend", "next"),
        load_layer("backend", "nestjs"),
    ];
    validate_selection(&selection).expect("the selection breaks no cardinality rule");

    let layers: Vec<_> = selection
        .iter()
        .map(|manifest| manifest.to_layer().expect("manifest converts to a layer"))
        .collect();
    let set = FileSet::build(&layers).expect("two apps merge cleanly");

    let rendered = set.render();
    let next_manifest = rendered
        .get(&PathBuf::from("src/next/package.json"))
        .expect("next owns its own package.json");
    assert!(next_manifest.contains("\"dev\""));

    let nestjs_manifest = rendered
        .get(&PathBuf::from("src/nestjs/package.json"))
        .expect("nestjs owns its own package.json");
    assert!(nestjs_manifest.contains("\"dev\""));

    let gitignore = rendered
        .get(&PathBuf::from(".gitignore"))
        .expect("both apps contribute to .gitignore");
    assert!(gitignore.contains("src/next/.next/"));
    assert!(gitignore.contains("src/nestjs/dist/"));
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
    let go = load_layer("toolchain", "go-mod");

    for (index, a) in backends.iter().enumerate() {
        for b in &backends[index + 1..] {
            let selection = vec![
                base.clone(),
                shape.clone(),
                runner.clone(),
                node.clone(),
                eslint.clone(),
                rust.clone(),
                go.clone(),
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
/// The frontend and the backend each own a real package.json under their own
/// directory (decision 23); the data layer still has no app of its own to
/// belong to, so it remains the one contributor left at the shared root.
#[test]
fn a_full_stack_monorepo_validates_and_merges() {
    let selection = vec![
        load_layer("base", "base"),
        load_layer("shape", "monorepo"),
        load_layer("runner", "nx"),
        load_layer("toolchain", "node-pnpm"),
        load_layer("quality", "eslint"),
        load_layer("frontend", "next"),
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

    let root_package_json = rendered
        .get(&PathBuf::from("package.json"))
        .expect("the data layer still contributes to the shared root");
    assert!(root_package_json.contains("generate:prisma"));

    let next_manifest = rendered
        .get(&PathBuf::from("src/next/package.json"))
        .expect("next owns its own package.json");
    assert!(next_manifest.contains("\"dev\""));

    let nestjs_manifest = rendered
        .get(&PathBuf::from("src/nestjs/package.json"))
        .expect("nestjs owns its own package.json");
    assert!(nestjs_manifest.contains("\"dev\""));

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
            targets.contains(&"package.json".to_owned())
                || targets.contains(&format!("src/{id}/package.json")),
            "'{id}' must contribute scripts"
        );
        assert!(
            targets.contains(&".gitignore".to_owned()),
            "'{id}' must ignore its build output"
        );
    }
}

/// A layer id that also happens to be an ordinary word a different layer's
/// payload genuinely needs, unrelated to that sibling layer:
/// - `backend/express` shares its id with the `express` npm package
///   `backend/nestjs` installs as `@nestjs/core`'s HTTP platform adapter (a
///   peer dependency Nest requires to boot at all).
/// - `editor/vscode` shares its id with the `.vscode/*` entry `base/base`
///   ignores for every workspace regardless of which editor layers are
///   selected (generic editor noise, not a reference to that layer).
/// - `quality/editorconfig` shares its id with the VS Code marketplace
///   extension id `editorconfig.editorconfig` that `editor/vscode`
///   recommends unconditionally, not a reference to that layer.
///
/// The substring search below cannot tell any of these apart from a real
/// reference to the sibling layer without this narrow, explicit allowlist.
const SIBLING_NAME_FALSE_POSITIVES: &[(&str, &str)] = &[
    ("nestjs", "express"),
    ("base", "vscode"),
    ("vscode", "editorconfig"),
];

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
            let is_a_false_positive =
                SIBLING_NAME_FALSE_POSITIVES.contains(&(manifest.id.as_str(), other.as_str()));
            if *other == manifest.id || required || is_a_false_positive {
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
///
/// Compared against the raw, uninterpolated manifest rather than `entries()`'s
/// namespaced one: a fragment's `files` glob is shipped with `{{app_dir}}`
/// etc. still in it (the whole point, so it works whether or not the
/// selection ends up namespaced), and `entries()` renders those same
/// placeholders in `to` before this test ever sees them. Comparing rendered
/// paths against an unrendered glob would flag every well-formed fragment.
#[test]
fn every_eslint_fragment_claims_only_its_own_paths() {
    let raw = shipped();
    for (entry, fragment) in layers_needing_a_fragment() {
        let manifest = &raw
            .entries()
            .find(|raw_entry| raw_entry.manifest.qualified_id() == entry.manifest.qualified_id())
            .unwrap_or_else(|| panic!("{} vanished from the raw catalog", entry.dir().display()))
            .manifest;
        let spec = manifest
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

        let written: Vec<String> = manifest
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
            payload.contains("typecheck") || payload.contains("check"),
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

// == What a script promises, checked without a network

/// Every `scripts` entry a layer contributes to `package.json`, as name and command.
fn scripts_of(manifest: &Manifest) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for merge in &manifest.merges {
        let culm::layer::MergeSpec::JsonDeep { target, value } = merge else {
            continue;
        };
        if target != Path::new("package.json") {
            continue;
        }
        let Some(scripts) = value.get("scripts").and_then(|v| v.as_table()) else {
            continue;
        };
        for (name, command) in scripts {
            if let Some(command) = command.as_str() {
                found.push((name.clone(), command.to_owned()));
            }
        }
    }
    found
}

/// Every path any shipped layer writes, whether copied or merged into.
fn every_written_path() -> Vec<String> {
    let mut paths = Vec::new();
    for entry in entries() {
        for spec in &entry.manifest.files {
            paths.push(spec.to.to_string_lossy().into_owned());
        }
        for merge in &entry.manifest.merges {
            let target = match merge {
                culm::layer::MergeSpec::JsonDeep { target, .. }
                | culm::layer::MergeSpec::LineBlock { target, .. }
                | culm::layer::MergeSpec::TomlMembers { target, .. }
                | culm::layer::MergeSpec::YamlList { target, .. }
                | culm::layer::MergeSpec::HookScript { target, .. }
                | culm::layer::MergeSpec::YamlSteps { target, .. } => target,
            };
            paths.push(target.to_string_lossy().into_owned());
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

/// Every directory some layer declares as generated, taken from its `.gitignore` block.
/// A script may legitimately name a path inside one: `start:nestjs` runs `dist/nestjs/main.js`, which the build produces rather than the scaffold.
fn generated_roots() -> Vec<String> {
    let mut roots = Vec::new();
    for entry in entries() {
        for merge in &entry.manifest.merges {
            let culm::layer::MergeSpec::LineBlock { target, lines } = merge else {
                continue;
            };
            if target != Path::new(".gitignore") {
                continue;
            }
            for line in lines {
                let root = line.trim_start_matches('/').trim_end_matches('/');
                if !root.is_empty() && !root.contains('*') {
                    roots.push(root.to_owned());
                }
            }
        }
    }
    roots.sort();
    roots.dedup();
    roots
}

/// A script naming a file nothing writes fails the moment it is run.
/// `tsc -p src/react-vite/tsconfig.json` is correct only while that tsconfig stays where it is, and moving one is exactly the kind of edit that looks safe. Catching it here costs nothing; catching it after an install costs a network round trip per bug.
#[test]
fn every_file_a_script_names_is_actually_written() {
    let written = every_written_path();
    let generated = generated_roots();

    for entry in entries() {
        for (name, command) in scripts_of(&entry.manifest) {
            for token in command.split_whitespace() {
                /* A path argument, as opposed to a flag or a subcommand. The
                 * bare `.` that prettier and eslint take is the workspace
                 * root and is not a file anybody writes. */
                let looks_like_a_path = token.contains('/')
                    || token.ends_with(".json")
                    || token.ends_with(".ts")
                    || token.ends_with(".mjs");
                if !looks_like_a_path || token.starts_with('-') {
                    continue;
                }
                let token = token.trim_start_matches("./");
                let first = token.split('/').next().unwrap_or(token);
                let known = written.iter().any(|path| path == token)
                    || generated.iter().any(|root| root == first);
                assert!(
                    known,
                    "'{}' script '{name}' names '{token}', which no layer writes and no layer generates",
                    entry.manifest.id
                );
            }
        }
    }
}

/// The package that provides each binary a script invokes.
/// Spelled out rather than derived, because a package name and the binary it installs agree only by convention and the exceptions are the ones worth catching.
fn provider_of(binary: &str) -> Option<String> {
    match binary {
        "node" => None,
        "tsc" => Some("typescript".to_owned()),
        "svelte-kit" => Some("@sveltejs/kit".to_owned()),
        "nest" => Some("@nestjs/cli".to_owned()),
        "astro" | "next" | "nuxt" | "vite" | "vitest" | "eslint" | "prettier" | "vue-tsc"
        | "drizzle-kit" | "prisma" => Some(binary.to_owned()),
        _ => None,
    }
}

/// A package string with any trailing version range stripped.
/// A scoped package's leading `@` is not itself a version marker (`@sveltejs/kit`), so only an `@` after the first character can start one (`typescript@<6`).
fn package_name(package: &str) -> &str {
    match package.get(1..).and_then(|rest| rest.find('@')) {
        Some(i) => &package[..i + 1],
        None => package,
    }
}

/// A script may only invoke a tool its own layer installs.
/// `typecheck:vue-vite` invoking `vue-tsc` is only meaningful because that layer declares it; a script reaching for a binary nobody installed fails with `command not found` after a full install, which is the most expensive possible moment to learn it.
#[test]
fn every_tool_a_script_invokes_is_installed() {
    for entry in entries() {
        let declared: Vec<&str> = entry
            .manifest
            .installs
            .iter()
            .flat_map(|install| install.packages.iter().map(|p| package_name(p)))
            .collect();

        for (name, command) in scripts_of(&entry.manifest) {
            let Some(binary) = command.split_whitespace().next() else {
                continue;
            };
            let Some(provider) = provider_of(binary) else {
                continue;
            };
            assert!(
                declared.contains(&provider.as_str()),
                "'{}' script '{name}' runs '{binary}' but the layer never installs '{provider}'",
                entry.manifest.id
            );
        }
    }
}

// == The template fixes of 2026-08-19, so they cannot quietly regress

/// Every template a layer ships, as destination and body, with placeholders resolved.
fn templates_of(entry: &CatalogEntry) -> Vec<(String, String)> {
    entry
        .manifest
        .files
        .iter()
        .filter_map(|spec| {
            let body = entry.source.read(&spec.from)?;
            Some((spec.to.to_string_lossy().into_owned(), body))
        })
        .collect()
}

/// TypeScript cannot resolve a `.vue` import without a module declaration.
/// A meta framework generates one during its prepare step; a layer without a meta framework has to ship it, or the entry point and everything it reaches is untyped while `tsc` reports `TS2307`.
#[test]
fn a_layer_shipping_vue_components_ships_a_type_shim() {
    for entry in entries() {
        let ships_components = entry
            .manifest
            .files
            .iter()
            .any(|spec| spec.to.to_string_lossy().ends_with(".vue"));
        if !ships_components {
            continue;
        }
        let has_a_shim = templates_of(&entry)
            .iter()
            .any(|(_, body)| body.contains(r#"declare module "*.vue""#));
        let has_a_meta_framework = entry
            .manifest
            .installs
            .iter()
            .any(|install| install.packages.iter().any(|p| p == "nuxt"));
        assert!(
            has_a_shim || has_a_meta_framework,
            "'{}' ships .vue components with neither a type shim nor a meta framework to generate one",
            entry.manifest.id
        );
    }
}

/// Testing Library registers its cleanup on a global `afterEach`, which `globals: false` removes.
/// Without an explicit call every render accumulates into the same document, so the second test in a file matches twice and fails. It is invisible in a file holding one test, which is exactly how it shipped.
#[test]
fn a_spec_using_testing_library_cleans_up_between_tests() {
    for entry in entries() {
        let templates = templates_of(&entry);
        let disables_globals = templates
            .iter()
            .any(|(to, body)| to.contains("vitest.") && body.contains("globals: false"));
        if !disables_globals {
            continue;
        }
        for (to, body) in &templates {
            if !body.contains("@testing-library/react") {
                continue;
            }
            assert!(
                body.contains("cleanup"),
                "'{}' ships {to} using Testing Library with globals: false but never calls cleanup, so renders leak between tests",
                entry.manifest.id
            );
        }
    }
}

/// A spec file outside every tsconfig is type checked by nothing.
/// The include globs resolve against the tsconfig's own directory, so a tsconfig scoped to `src/<id>/` cannot see the sibling `test/<id>/` tree unless it names it.
#[test]
fn a_scoped_tsconfig_covers_its_own_tests() {
    for entry in entries() {
        let templates = templates_of(&entry);
        let ships_specs = entry
            .manifest
            .files
            .iter()
            .any(|spec| spec.to.to_string_lossy().starts_with("test/"));
        if !ships_specs {
            continue;
        }
        /* A layer may ship two tsconfigs on purpose (an app-scoped one plus a
         * root one that extends it and adds the test tree, as vue-vite and
         * react-vite both do) - the app-scoped one is not expected to name
         * tests itself, only *some* tsconfig for the layer has to. And the
         * name it names them by may be the raw "test/" a layer that hardcodes
         * its own (namespaced) path uses, or the "{{test_dir}}" placeholder a
         * layer that works in any layout uses instead - both cover the tests,
         * only the raw text differs. */
        let tsconfigs_with_an_include: Vec<&(String, String)> = templates
            .iter()
            .filter(|(to, body)| {
                to.contains("tsconfig") && to.ends_with(".json") && body.contains("\"include\"")
            })
            .collect();
        if tsconfigs_with_an_include.is_empty() {
            continue;
        }
        assert!(
            tsconfigs_with_an_include
                .iter()
                .any(|(_, body)| body.contains("test/") || body.contains("{{test_dir}}")),
            "'{}' ships specs under test/ but no tsconfig's include ever names them",
            entry.manifest.id
        );
    }
}

/// A placeholder nothing substitutes reaches the user's workspace as literal text.
/// Interpolation deliberately leaves an unknown placeholder alone rather than blanking it, so a typo survives all the way into a generated file rather than failing the scaffold.
#[test]
fn every_placeholder_in_a_template_is_a_variable_that_exists() {
    let known = [
        "project_name",
        "app_dir",
        "test_dir",
        "script_suffix",
        "config_suffix",
        "to_root",
    ];

    for entry in entries() {
        for (to, body) in templates_of(&entry) {
            /* Vue, Astro and Svelte all use moustaches for their own runtime
             * interpolation, and culm shares the syntax. That collision is
             * survivable because interpolation leaves an unknown placeholder
             * exactly as written, so `{{ appName }}` reaches the framework
             * untouched. It does mean a typo cannot be told from a component
             * binding in those files, so they are not checked here. */
            let framework_moustaches = [".vue", ".astro", ".svelte", ".html"]
                .iter()
                .any(|ext| to.ends_with(ext));
            if framework_moustaches {
                continue;
            }
            let mut rest = body.as_str();
            while let Some(start) = rest.find("{{") {
                let after = &rest[start + 2..];
                let Some(end) = after.find("}}") else {
                    break;
                };
                let name = after[..end].trim();
                let is_an_identifier =
                    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
                if is_an_identifier {
                    assert!(
                        known.contains(&name),
                        "'{}' template {to} uses '{{{{{name}}}}}', which nothing substitutes",
                        entry.manifest.id
                    );
                }
                rest = &after[end + 2..];
            }
        }
    }
}

/// Every JSON a layer ships is valid JSON once its placeholders are resolved.
/// A malformed tsconfig is not caught by any merge, because a copied file is never parsed on the way through; the first thing that reads it is the user's toolchain, after an install.
#[test]
fn every_json_template_parses() {
    for entry in entries() {
        for (to, body) in templates_of(&entry) {
            if !to.ends_with(".json") {
                continue;
            }
            let resolved = body.replace("{{project_name}}", "demo");
            serde_json::from_str::<serde_json::Value>(&resolved).unwrap_or_else(|e| {
                panic!(
                    "'{}' ships {to}, which is not valid JSON: {e}",
                    entry.manifest.id
                )
            });
        }
    }
}
