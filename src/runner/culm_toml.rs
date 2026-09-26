// === culm's own backend

/* The `runner/culm` layer: a root `culm.toml` declaring `[workspace]`, plus
 * one `culm.toml` per application. Decision 3 keeps this one runner among
 * several and never the default; nothing any other layer emits depends on
 * it.
 *
 * Every choice here errs toward running, because a wrong hit is the one
 * failure the runner exists to avoid (docs/DESIGN.md section 3.1):
 *
 * - Inputs are every file in the app that `.gitignore` does not exclude,
 *   plus the root lockfile. Too broad only costs a miss.
 * - `build` is never cached. Where a build writes its output differs per
 *   framework, and a hit that restores nothing is a wrong hit.
 * - `test` is cached: its result is pass or fail plus the logs, which a hit
 *   replays exactly.
 * - `dev` is a long running server, never cached. */

use std::fmt::Write as _;

/// How an app's tasks are run, from the toolchain its layer requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toolchain {
    Node,
    Rust,
    Go,
}

/// One application, as its `culm.toml` needs to see it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CulmApp {
    pub app_dir: String,
    /// The path from `app_dir` back to the workspace root, ending in `/`
    /// or empty.
    pub to_root: String,
    pub toolchain: Toolchain,
}

/// The root `culm.toml`. `root_tasks` are the root-owned scripts a quality
/// layer contributed (`lint`, `format`).
pub fn generate_root(root_tasks: &[String]) -> String {
    let mut doc = String::from("# culm's task runner. See docs/DESIGN.md in culm for the format.\n[workspace]\n");
    for task in root_tasks {
        /* `format` rewrites files in place, so it has no output a cache
         * could restore; `lint` only reports. */
        let cacheable = task != "format";
        write!(
            doc,
            "\n[tasks.{task}]\ncommand = \"pnpm run {task}\"\n"
        )
        .expect("writing to a String cannot fail");
        if cacheable {
            doc.push_str("inputs = [\"**/*\"]\ncache = true\n");
        }
    }
    doc
}

/// One application's `culm.toml`.
pub fn generate_project(app: &CulmApp) -> String {
    let (dev, build, test, lockfile) = match app.toolchain {
        Toolchain::Node => ("pnpm run dev", "pnpm run build", "pnpm run test", "pnpm-lock.yaml"),
        Toolchain::Rust => ("cargo run", "cargo build", "cargo test", "Cargo.lock"),
        Toolchain::Go => ("go run .", "go build ./...", "go test ./...", "go.sum"),
    };
    /* Go's lockfile sits beside `go.mod` in the app itself, which `**` already covers. */
    let inputs = if app.toolchain == Toolchain::Go {
        "[\"**/*\"]".to_owned()
    } else {
        format!("[\"**/*\", \"{}{lockfile}\"]", app.to_root)
    };

    format!(
        "[tasks.dev]\n\
         command = \"{dev}\"\n\
         \n\
         [tasks.build]\n\
         command = \"{build}\"\n\
         inputs = {inputs}\n\
         \n\
         [tasks.test]\n\
         command = \"{test}\"\n\
         inputs = {inputs}\n\
         depends_on = [\"build\"]\n\
         cache = true\n"
    )
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::RunnerConfig;

    fn parse(doc: &str) -> RunnerConfig {
        toml::from_str(doc).unwrap()
    }

    #[test]
    fn the_root_declares_a_workspace_and_caches_lint_but_not_format() {
        let config = parse(&generate_root(&["lint".to_owned(), "format".to_owned()]));
        assert!(config.workspace.is_some());
        assert!(config.tasks["lint"].cache);
        assert!(!config.tasks["format"].cache);
    }

    #[test]
    fn a_node_app_caches_test_but_never_build_or_dev() {
        let config = parse(&generate_project(&CulmApp {
            app_dir: "src/nuxt".to_owned(),
            to_root: "../../".to_owned(),
            toolchain: Toolchain::Node,
        }));
        assert!(!config.tasks["build"].cache);
        assert!(!config.tasks["dev"].cache);
        assert!(config.tasks["test"].cache);
        assert_eq!(
            config.tasks["test"].inputs,
            vec!["**/*".to_owned(), "../../pnpm-lock.yaml".to_owned()]
        );
        assert_eq!(config.tasks["test"].depends_on, vec!["build".to_owned()]);
    }

    #[test]
    fn a_rust_app_hashes_the_root_cargo_lock() {
        let config = parse(&generate_project(&CulmApp {
            app_dir: "crates/axum".to_owned(),
            to_root: "../../".to_owned(),
            toolchain: Toolchain::Rust,
        }));
        assert_eq!(config.tasks["test"].command, "cargo test");
        assert!(config.tasks["test"].inputs.contains(&"../../Cargo.lock".to_owned()));
    }
}
