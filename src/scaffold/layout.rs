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
pub fn variables(entries: &[&CatalogEntry], project: &str) -> Vec<Variables> {
    let apps = entries
        .iter()
        .filter(|entry| is_app(entry.manifest.category))
        .count();

    entries
        .iter()
        .map(|entry| {
            let base = Variables::new().with("project_name", project);
            let id = &entry.manifest.id;

            /* Tooling layers never name these, so the value they get does not
             * matter. They are still given the namespaced form rather than the
             * bare one, so a layer that starts using a variable is wrong
             * loudly in a monorepo rather than quietly. */
            if apps == 1 && is_app(entry.manifest.category) {
                base.with("app_dir", "src")
                    .with("test_dir", "test")
                    .with("script_suffix", "")
                    .with("config_suffix", "")
            } else {
                base.with("app_dir", format!("src/{id}"))
                    .with("test_dir", format!("test/{id}"))
                    .with("script_suffix", format!(":{id}"))
                    .with("config_suffix", format!(".{id}"))
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
        assert_eq!(vars[1].get("app_dir"), Some("src/next"));
        assert_eq!(vars[1].get("script_suffix"), Some(":next"));
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
