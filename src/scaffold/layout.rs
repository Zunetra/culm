// === Where an app layer puts itself

/* A workspace holding one application should look like one application. The
 * same layer in a workspace holding four cannot, because four layers writing
 * `scripts.dev` or owning `vite.config.ts` is a hard merge error rather than
 * something to resolve silently.
 *
 * So the layout is not baked into the layer. A layer names the variables below
 * and the scaffolder fills them in from the selection it was given. This is a
 * rule about how many app layers there are, never about which ones: the moment
 * a value depends on a specific pair of layers, the combinatorial death has
 * started, and that is a kill criterion. */

use crate::layer::{CatalogEntry, Category};

use super::interpolate::Variables;

/// Whether a layer is an application rather than tooling around one.
/// Data layers are excluded: they contribute to an app's crate or package rather than owning scripts and a source directory of their own.
fn is_app(category: Category) -> bool {
    matches!(category, Category::Frontend | Category::Backend)
}

/// The variables each entry's manifest and templates are rendered with, in the order the entries were given.
///
/// The whole selection decides the layout, which is why this takes the resolved
/// list rather than one entry. A single application gets the idiomatic layout
/// its framework's own tooling expects; two or more get one namespaced
/// directory and one namespaced script prefix each, because that is the only
/// way they coexist.
///
/// `to_root` is the relative path back to the workspace root from wherever
/// `app_dir` puts a file, so a template that must `extends` or `import` a
/// root-level file (a framework's own generated tsconfig, say) never counts
/// `../` itself. `app_dir` is always one segment (`src`) or two (`src/<id>`),
/// so this is the one place that fact is allowed to matter; a third depth
/// would mean this function changed and this constant would have to too.
pub fn variables(entries: &[&CatalogEntry], project: &str) -> Vec<Variables> {
    let apps = entries
        .iter()
        .filter(|entry| is_app(entry.manifest.category))
        .count();

    /* Unlike app_dir/pkg_target, this is not a per-entry branch: a tooling
     * layer (toolchain/node-pnpm) is never an app, so it would always land in
     * the namespaced branch below regardless of how many apps are actually
     * selected, and it is exactly the layer that needs to know the true
     * count. Blank outside a monorepo rather than omitted, because
     * `yaml-list` already treats a blank contribution as none at all (see
     * `merge::yaml_list`), which is what keeps `pnpm-workspace.yaml` from
     * being generated for a single-app or tooling-only workspace. */
    let workspace_glob = if apps > 1 { "src/*" } else { "" };

    entries
        .iter()
        .map(|entry| {
            let base = Variables::new()
                .with("project_name", project)
                .with("workspace_glob", workspace_glob);
            let id = &entry.manifest.id;

            /* Tooling layers never name these, so the value they get does not
             * matter. They are still given the namespaced form rather than the
             * bare one, so a layer that starts using a variable is wrong
             * loudly in a monorepo rather than quietly.
             *
             * `cd_app_dir` exists because `{{pkg_target}}`'s directory and
             * `{{app_dir}}` are the same directory in the monorepo case: pnpm
             * already sets a script's cwd to the package's own directory
             * before running it, so `cd {{app_dir}}` from inside that very
             * directory fails outright rather than being a harmless no-op.
             * In the single-app case the package.json sits at the workspace
             * root while the sources sit under `{{app_dir}}`, so the cd is
             * genuinely needed there. A script writes `{{cd_app_dir}}<rest>`
             * instead of hardcoding the cd itself. */
            if apps == 1 && is_app(entry.manifest.category) {
                base.with("app_dir", "src")
                    .with("test_dir", "test")
                    .with("script_suffix", "")
                    .with("config_suffix", "")
                    .with("to_root", "../")
                    .with("pkg_target", "package.json")
                    .with("pkg_name", project.to_owned())
                    .with("cd_app_dir", "cd src && ")
            } else {
                base.with("app_dir", format!("src/{id}"))
                    .with("test_dir", format!("test/{id}"))
                    .with("script_suffix", format!(":{id}"))
                    .with("config_suffix", format!(".{id}"))
                    .with("to_root", "../../")
                    .with("pkg_target", format!("src/{id}/package.json"))
                    .with("pkg_name", format!("{project}-{id}"))
                    .with("cd_app_dir", "")
            }
        })
        .collect()
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::{LayerSource, Manifest};
    use std::path::PathBuf;

    fn entry(source: &str) -> CatalogEntry {
        CatalogEntry {
            manifest: Manifest::parse("layer.toml", source).unwrap(),
            source: LayerSource::Disk(PathBuf::from("unused")),
        }
    }

    fn frontend(id: &str) -> CatalogEntry {
        entry(&format!("id = \"{id}\"\ncategory = \"frontend\"\n"))
    }

    fn backend(id: &str) -> CatalogEntry {
        entry(&format!("id = \"{id}\"\ncategory = \"backend\"\n"))
    }

    fn tooling() -> CatalogEntry {
        entry("id = \"prettier\"\ncategory = \"quality\"\n")
    }

    #[test]
    fn one_application_gets_the_layout_its_framework_expects() {
        let only = frontend("nuxt");
        let vars = variables(&[&only], "demo");

        assert_eq!(vars[0].get("app_dir"), Some("src"));
        assert_eq!(vars[0].get("test_dir"), Some("test"));
        assert_eq!(vars[0].get("script_suffix"), Some(""));
        assert_eq!(vars[0].get("config_suffix"), Some(""));
        assert_eq!(vars[0].get("to_root"), Some("../"));
        assert_eq!(vars[0].get("pkg_target"), Some("package.json"));
        assert_eq!(vars[0].get("pkg_name"), Some("demo"));
        assert_eq!(vars[0].get("cd_app_dir"), Some("cd src && "));
        assert_eq!(vars[0].get("workspace_glob"), Some(""));
    }

    /* Two frontends both writing `scripts.dev` is a hard merge error, so the
     * second application in a workspace is what forces the namespacing. */
    #[test]
    fn two_applications_are_each_namespaced() {
        let a = frontend("nuxt");
        let b = frontend("next");
        let vars = variables(&[&a, &b], "demo");

        assert_eq!(vars[0].get("app_dir"), Some("src/nuxt"));
        assert_eq!(vars[0].get("script_suffix"), Some(":nuxt"));
        assert_eq!(vars[0].get("to_root"), Some("../../"));
        assert_eq!(vars[0].get("pkg_target"), Some("src/nuxt/package.json"));
        assert_eq!(vars[0].get("pkg_name"), Some("demo-nuxt"));
        assert_eq!(vars[0].get("cd_app_dir"), Some(""));
        assert_eq!(vars[1].get("app_dir"), Some("src/next"));
        assert_eq!(vars[1].get("script_suffix"), Some(":next"));
        assert_eq!(vars[1].get("to_root"), Some("../../"));
        assert_eq!(vars[1].get("pkg_target"), Some("src/next/package.json"));
        assert_eq!(vars[1].get("pkg_name"), Some("demo-next"));
        assert_eq!(vars[1].get("cd_app_dir"), Some(""));
        assert_eq!(vars[0].get("workspace_glob"), Some("src/*"));
        assert_eq!(vars[1].get("workspace_glob"), Some("src/*"));
    }

    /* The whole reason `workspace_glob` is computed globally rather than
     * per-entry like `app_dir`: a tooling layer is never an app, so it needs
     * the true count directly rather than inheriting whichever branch its own
     * category happens to fall into. */
    #[test]
    fn a_tooling_layer_sees_the_true_application_count() {
        let front = frontend("nuxt");
        let single = tooling();
        assert_eq!(
            variables(&[&front, &single], "demo")[1].get("workspace_glob"),
            Some("")
        );

        let back = backend("express");
        let mono = tooling();
        assert_eq!(
            variables(&[&front, &back, &mono], "demo")[2].get("workspace_glob"),
            Some("src/*")
        );
    }

    /* The case a shape check alone would get wrong. A single-app workspace
     * takes at most one frontend and at most one backend, which is still two
     * layers wanting `dev`, so the count is what decides rather than the
     * shape. */
    #[test]
    fn a_frontend_beside_a_backend_is_namespaced_even_in_one_app() {
        let front = frontend("nuxt");
        let back = backend("express");
        let vars = variables(&[&front, &back], "demo");

        assert_eq!(vars[0].get("script_suffix"), Some(":nuxt"));
        assert_eq!(vars[1].get("script_suffix"), Some(":express"));
    }

    /* Quality and toolchain layers are not applications, so they must not make
     * the one application in a workspace think it has company. */
    #[test]
    fn tooling_does_not_count_as_an_application() {
        let only = frontend("nuxt");
        let prettier = tooling();
        let vars = variables(&[&only, &prettier], "demo");

        assert_eq!(vars[0].get("app_dir"), Some("src"));
        assert_eq!(vars[0].get("script_suffix"), Some(""));
    }
}
