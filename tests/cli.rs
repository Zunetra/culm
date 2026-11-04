// === culm new, end to end

/* Driven through `run_new` rather than by spawning the binary. A subprocess
 * test has to locate a binary whose path differs per profile, it is slower, and
 * a failure reports an exit code where this reports the error. */

use std::fs;
use std::path::{Path, PathBuf};

use culm::cli::{NewArgs, run_new};

// == Helpers

fn layers_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("layers")
}

/// The arguments a scripted run would pass: chosen layers, no prompt, layers compiled in.
/// `no_install` is on for every test in this file. A test that shells out to a real pnpm needs a network and a machine that has one, and neither belongs in CI.
fn args(project: &str, layers: &[&str]) -> NewArgs {
    NewArgs {
        project: project.to_owned(),
        layers: layers.iter().map(|s| (*s).to_owned()).collect(),
        preset: None,
        without: Vec::new(),
        yes: true,
        layers_dir: None,
        no_install: true,
        min_release_age: culm::install::release_age::DEFAULT_DAYS,
        save_preset: None,
    }
}

/// The same run, with everything printed captured.
fn report(args: &NewArgs, parent: &Path) -> String {
    let mut out: Vec<u8> = Vec::new();
    culm::cli::run_new_to(args, parent, &mut out).expect("the scaffold runs");
    String::from_utf8(out).expect("the report is UTF-8")
}

/// Every path under `root`, relative and sorted, so two trees can be compared as a whole.
fn tree(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    walk(root, root, &mut found);
    found.sort();
    found
}

fn walk(root: &Path, dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("a readable directory") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            walk(root, &path, found);
        } else {
            found.push(
                path.strip_prefix(root)
                    .expect("every path is under the root")
                    .to_path_buf(),
            );
        }
    }
}

// == A workspace lands on disk

#[test]
fn a_rust_selection_is_scaffolded() {
    let dir = tempfile::tempdir().expect("a temp dir");
    run_new(&args("demo", &["backend/axum"]), dir.path()).expect("the scaffold runs");

    let root = dir.path().join("demo");
    assert!(root.join("Cargo.toml").is_file());
    assert!(root.join(".gitignore").is_file());
    assert!(
        !root.join("package.json").exists(),
        "a Rust-only workspace must not invent a package.json"
    );
}

#[test]
fn a_node_selection_is_scaffolded() {
    let dir = tempfile::tempdir().expect("a temp dir");
    run_new(&args("demo", &["backend/fastify"]), dir.path()).expect("the scaffold runs");

    let root = dir.path().join("demo");
    assert!(root.join("package.json").is_file());
    assert!(
        !root.join("Cargo.toml").exists(),
        "a Node-only workspace must not invent a Cargo.toml"
    );
}

/// The project name reaches a copied template, not just the directory name.
/// Merged files are covered by `the_root_package_json_is_named_after_the_project`, since the two go through different code paths.
#[test]
fn the_project_name_is_interpolated() {
    let dir = tempfile::tempdir().expect("a temp dir");
    run_new(&args("klassivox", &["backend/axum"]), dir.path()).expect("the scaffold runs");

    let manifest = fs::read_to_string(
        dir.path()
            .join("klassivox")
            .join("crates")
            .join("axum")
            .join("Cargo.toml"),
    )
    .expect("the member crate manifest is written");
    assert!(manifest.contains("klassivox-axum"));
    assert!(
        !manifest.contains("{{project_name}}"),
        "an uninterpolated placeholder reached disk"
    );
}

/// No copied file may reach disk still carrying a placeholder.
#[test]
fn no_placeholder_survives_into_the_workspace() {
    let dir = tempfile::tempdir().expect("a temp dir");
    run_new(
        &args(
            "demo",
            &["shape/monorepo", "backend/axum", "backend/nestjs"],
        ),
        dir.path(),
    )
    .expect("the scaffold runs");

    let root = dir.path().join("demo");
    for path in tree(&root) {
        let contents = fs::read_to_string(root.join(&path)).unwrap_or_default();
        assert!(
            !contents.contains("{{project_name}}"),
            "{} still carries a placeholder",
            path.display()
        );
    }
}

// == The escape hatch stays honest

/* T1 kept `Catalog::load` so layers can be authored on disk while a release
 * reads the copy compiled in. The moment those two disagree, every test in this
 * repository is testing something no user will ever run. */
#[test]
fn the_embedded_layers_and_the_directory_produce_the_same_tree() {
    let embedded = tempfile::tempdir().expect("a temp dir");
    run_new(&args("demo", &["backend/axum"]), embedded.path()).expect("the embedded run works");

    let from_disk = tempfile::tempdir().expect("a temp dir");
    let mut args = args("demo", &["backend/axum"]);
    args.layers_dir = Some(layers_root());
    run_new(&args, from_disk.path()).expect("the on-disk run works");

    let a = embedded.path().join("demo");
    let b = from_disk.path().join("demo");
    assert_eq!(tree(&a), tree(&b), "the two catalogs write different files");

    for path in tree(&a) {
        assert_eq!(
            fs::read(a.join(&path)).expect("a readable file"),
            fs::read(b.join(&path)).expect("a readable file"),
            "{} differs between the two catalogs",
            path.display()
        );
    }
}

// == Refusals

#[test]
fn a_default_layer_can_be_suppressed() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut args = args("demo", &["backend/axum"]);
    args.without = vec!["quality/clippy".to_owned()];
    run_new(&args, dir.path()).expect("the scaffold runs");

    assert!(
        !dir.path().join("demo").join("clippy.toml").exists(),
        "a suppressed layer still wrote its file"
    );
}

#[test]
fn an_unknown_layer_is_named() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let error = run_new(&args("demo", &["backend/ember"]), dir.path())
        .expect_err("an unknown layer is refused");
    assert!(
        error.to_string().contains("backend/ember"),
        "the error does not name the layer: {error}"
    );
}

#[test]
fn choosing_nothing_is_refused() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let error = run_new(&args("demo", &[]), dir.path()).expect_err("an empty selection is refused");
    assert!(error.to_string().contains("--layer"));
}

// == Installs

/* The plan is what `--no-install` exists to show, so the assertion is on the
 * exact commands rather than on the fact that something was printed. */
#[test]
fn the_install_plan_is_printed_when_it_is_not_run() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let printed = report(&args("demo", &["backend/axum"]), dir.path());

    assert!(printed.contains("Installs skipped"), "{printed}");
    assert!(
        printed.contains("cargo add --manifest-path crates/axum/Cargo.toml axum"),
        "{printed}"
    );
    assert!(
        printed.contains("cargo add --manifest-path crates/axum/Cargo.toml --features full tokio"),
        "{printed}"
    );
    assert!(
        !printed.contains("Installing dependencies"),
        "the plan was skipped and run at once: {printed}"
    );
}

/// Decision 5: the package manager writes versions, so no command may name one.
#[test]
fn no_planned_install_names_a_version() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let printed = report(
        &args("demo", &["shape/monorepo", "frontend/nuxt", "backend/axum"]),
        dir.path(),
    );

    for line in printed.lines().filter(|line| {
        line.trim_start().starts_with("pnpm add") || line.trim_start().starts_with("cargo add")
    }) {
        /* A scope is `@nuxt/test-utils`; a version is `axum@0.7`. Only the
         * second puts a digit straight after the `@`. */
        for (index, byte) in line.bytes().enumerate() {
            if byte == b'@' {
                let next = line.as_bytes().get(index + 1).copied().unwrap_or(b' ');
                assert!(
                    !next.is_ascii_digit(),
                    "a version reached an install command: {line}"
                );
            }
        }
    }
}

// == Presets

#[test]
fn each_preset_scaffolds() {
    for (preset, marker) in [
        ("backend", "Cargo.toml"),
        ("frontend", "src/nuxt.config.ts"),
        ("monorepo", "src/nuxt/nuxt.config.ts"),
    ] {
        let dir = tempfile::tempdir().expect("a temp dir");
        let mut args = args("demo", &[]);
        args.preset = Some(preset.to_owned());
        run_new(&args, dir.path()).unwrap_or_else(|e| panic!("preset `{preset}` fails: {e}"));

        assert!(
            dir.path().join("demo").join(marker).is_file(),
            "preset `{preset}` did not write {marker}"
        );
    }
}

#[test]
fn an_unknown_preset_lists_the_ones_that_exist() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut args = args("demo", &[]);
    args.preset = Some("nosuch".to_owned());

    let error = run_new(&args, dir.path()).expect_err("an unknown preset is refused");
    let message = error.to_string();
    assert!(message.contains("nosuch"), "{message}");
    assert!(message.contains("backend"), "{message}");
    assert!(message.contains("monorepo"), "{message}");
}

/// A `--layer` given with a preset extends it rather than replacing it.
#[test]
fn a_layer_extends_a_preset() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut args = args("demo", &["runner/culm"]);
    args.preset = Some("backend".to_owned());

    run_new(&args, dir.path()).expect("the preset plus a layer scaffolds");

    let root = dir.path().join("demo");
    assert!(
        root.join("crates/axum/Cargo.toml").is_file(),
        "the preset's backend is missing"
    );
    assert!(
        root.join("culm.toml").is_file(),
        "the extra runner layer is missing"
    );
}

/// Naming a layer the preset already has is not a duplicate.
#[test]
fn a_layer_the_preset_already_names_is_not_repeated() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut args = args("demo", &["backend/axum"]);
    args.preset = Some("backend".to_owned());

    run_new(&args, dir.path()).expect("the overlap is harmless");
}

/// A preset fixes the shape, so a second shape is a selection error, not a silent swap.
#[test]
fn a_second_shape_on_top_of_a_preset_is_refused() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut args = args("demo", &["shape/monorepo"]);
    args.preset = Some("backend".to_owned());

    let error = run_new(&args, dir.path()).expect_err("two shapes are refused");
    assert!(error.to_string().contains("shape"), "{error}");
}

// == The nuxt layer emits an application

/// One application in a workspace gets the layout its own framework's tooling expects.
/// A developer who asked for Nuxt and nothing else should not have to explain to anyone why their sources sit one directory deeper than every Nuxt project they have seen.
#[test]
fn one_application_is_laid_out_like_an_ordinary_project() {
    let dir = tempfile::tempdir().expect("a temp dir");
    run_new(&args("demo", &["frontend/nuxt"]), dir.path()).expect("the scaffold runs");

    let root = dir.path().join("demo");
    for path in [
        "src/nuxt.config.ts",
        "src/app.vue",
        "src/error.vue",
        "src/pages/index.vue",
        "src/layouts/default.vue",
        "src/vitest.config.ts",
        "src/test/index.spec.ts",
    ] {
        assert!(root.join(path).is_file(), "{path} is missing");
    }

    let manifest = fs::read_to_string(root.join("package.json")).expect("a package.json");
    assert!(
        manifest.contains("\"dev\""),
        "the only app should own `dev`"
    );
    assert!(
        !manifest.contains("dev:nuxt"),
        "nothing to disambiguate from: {manifest}"
    );
}

/// A second application is what forces the namespacing, and it has to force it for both.
/// Two layers writing different values to `scripts.dev`, or both owning `vitest.config.ts`, is a hard error rather than something to resolve silently.
#[test]
fn a_second_application_namespaces_both_of_them() {
    let dir = tempfile::tempdir().expect("a temp dir");
    run_new(
        &args(
            "demo",
            &["shape/monorepo", "frontend/nuxt", "frontend/react-vite"],
        ),
        dir.path(),
    )
    .expect("the scaffold runs");

    let root = dir.path().join("demo");
    for path in [
        "src/nuxt/app.vue",
        "src/nuxt/vitest.config.ts",
        "src/nuxt/test/index.spec.ts",
        "src/react-vite/App.tsx",
        "src/react-vite/vitest.config.ts",
        "src/react-vite/test/App.spec.tsx",
    ] {
        assert!(root.join(path).is_file(), "{path} is missing");
    }

    let nuxt_manifest =
        fs::read_to_string(root.join("src/nuxt/package.json")).expect("nuxt's own package.json");
    assert!(
        nuxt_manifest.contains("\"name\": \"demo-nuxt\""),
        "{nuxt_manifest}"
    );
    assert!(nuxt_manifest.contains("\"dev\""), "{nuxt_manifest}");

    let react_vite_manifest = fs::read_to_string(root.join("src/react-vite/package.json"))
        .expect("react-vite's own package.json");
    assert!(
        react_vite_manifest.contains("\"name\": \"demo-react-vite\""),
        "{react_vite_manifest}"
    );
    assert!(
        react_vite_manifest.contains("\"dev\""),
        "{react_vite_manifest}"
    );
}

/// The case a shape check alone gets wrong.
/// A single-app workspace still takes one frontend and one backend, which is two layers wanting `dev`, so what decides is how many applications there are rather than the shape.
#[test]
fn a_frontend_beside_a_backend_is_namespaced_in_a_single_app() {
    let dir = tempfile::tempdir().expect("a temp dir");
    run_new(
        &args("demo", &["frontend/nuxt", "backend/fastify"]),
        dir.path(),
    )
    .expect("the scaffold runs");

    let root = dir.path().join("demo");
    let root_manifest = fs::read_to_string(root.join("package.json")).expect("a package.json");
    assert!(
        !root_manifest.contains("\"dev\""),
        "nuxt and fastify each own their own package.json now: {root_manifest}"
    );

    let nuxt_manifest =
        fs::read_to_string(root.join("src/nuxt/package.json")).expect("nuxt's own package.json");
    assert!(nuxt_manifest.contains("\"dev\""), "{nuxt_manifest}");

    let fastify_manifest = fs::read_to_string(root.join("src/fastify/package.json"))
        .expect("fastify's own package.json");
    assert!(fastify_manifest.contains("\"dev\""), "{fastify_manifest}");
}

/// Scaffolding on top of existing work would overwrite files the user did not name.
#[test]
fn a_non_empty_destination_is_refused() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = dir.path().join("demo");
    fs::create_dir(&root).expect("the directory is created");
    fs::write(root.join("README.md"), "mine").expect("a file is written");

    let error =
        run_new(&args("demo", &["backend/axum"]), dir.path()).expect_err("the destination is kept");
    assert!(error.to_string().contains("not empty"));
    assert_eq!(
        fs::read_to_string(root.join("README.md")).expect("the file survives"),
        "mine"
    );
}

/// An empty directory is a reasonable thing to have created before running culm in it.
#[test]
fn an_empty_destination_is_accepted() {
    let dir = tempfile::tempdir().expect("a temp dir");
    fs::create_dir(dir.path().join("demo")).expect("the directory is created");

    run_new(&args("demo", &["backend/axum"]), dir.path()).expect("the scaffold runs");
    assert!(dir.path().join("demo").join("Cargo.toml").is_file());
}

#[test]
fn a_project_name_that_is_a_path_is_refused() {
    let dir = tempfile::tempdir().expect("a temp dir");
    assert!(run_new(&args("../escape", &["backend/axum"]), dir.path()).is_err());
}

/// The workspace carries the project's name, not just its directory.
/// Merged files were not interpolated at all until this was fixed, so nothing could own a key whose value is only known at scaffold time.
#[test]
fn the_root_package_json_is_named_after_the_project() {
    let dir = tempfile::tempdir().expect("a temp dir");
    run_new(&args("klassivox", &["frontend/nuxt"]), dir.path()).expect("the scaffold runs");

    let manifest = fs::read_to_string(dir.path().join("klassivox").join("package.json"))
        .expect("a package.json is written");
    assert!(
        manifest.contains("\"name\": \"klassivox\""),
        "the root manifest is unnamed: {manifest}"
    );
    assert!(
        !manifest.contains("{{project_name}}"),
        "a placeholder survived into a merged file"
    );
}

/* Decision 5's guard is written into every JS workspace, and a different
 * `--min-release-age` replaces it in pnpm's own unit, minutes. */
#[test]
fn every_js_workspace_carries_the_release_age_guard() {
    let dir = tempfile::tempdir().unwrap();
    report(&args("demo", &["frontend/react-vite"]), dir.path());
    let written = fs::read_to_string(dir.path().join("demo/pnpm-workspace.yaml")).unwrap();
    assert!(written.contains("minimumReleaseAge: 4320"), "{written}");

    let other = tempfile::tempdir().unwrap();
    let mut week = args("demo", &["frontend/react-vite"]);
    week.min_release_age = 7;
    report(&week, other.path());
    let written = fs::read_to_string(other.path().join("demo/pnpm-workspace.yaml")).unwrap();
    assert!(written.contains("minimumReleaseAge: 10080"), "{written}");
}
