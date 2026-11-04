// === culm new

/* The whole scaffold, driven by flags rather than by a wizard. The wizard is
 * deliberately last: flags have to work anyway, they are what CI uses, and a
 * tool whose only entry point is a set of prompts cannot be scripted. */

use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;

use crate::cli::CliError;
use crate::install;
use crate::layer::{Catalog, CatalogEntry, Category, LayerSource, Manifest, resolve};
use crate::merge::{FileSet, LayerId};
use crate::preset;
use crate::runner;
use crate::scaffold::{self, CopyPlan, Variables, write};
use crate::telemetry::{ErrorKind, Install, Scaffold, SelectedVia};

#[derive(Debug, Args)]
pub struct NewArgs {
    /// Name of the project, used as the directory name and as `{{project_name}}`.
    pub project: String,

    /// A layer to include, qualified as `<category>/<id>`. Repeat for several.
    #[arg(long = "layer", value_name = "ID")]
    pub layers: Vec<String>,

    /// A named selection of layers to start from.
    /// Any `--layer` given as well is added on top of it.
    #[arg(long, value_name = "NAME")]
    pub preset: Option<String>,

    /// A layer to suppress even though it would otherwise arrive as a default.
    #[arg(long = "without", value_name = "ID")]
    pub without: Vec<String>,

    /// Skip the confirmation prompt. This is what CI uses.
    #[arg(long)]
    pub yes: bool,

    /// Read layers from this directory instead of the ones compiled into the binary.
    /// For authoring layers and for the tests. A release scaffolds from what shipped.
    #[arg(long, value_name = "PATH")]
    pub layers_dir: Option<PathBuf>,

    /// Write the workspace but print the package manager commands instead of running them.
    #[arg(long)]
    pub no_install: bool,

    /// Never install a release younger than this many days (decision 5's
    /// supply-chain guard). pnpm enforces it through `pnpm-workspace.yaml`;
    /// for Cargo and Go, culm picks the version itself. 0 turns it off.
    #[arg(long, value_name = "DAYS", default_value_t = install::release_age::DEFAULT_DAYS)]
    pub min_release_age: u64,

    /// Save the resolved layer selection as `<NAME>.toml`, next to where this was run.
    /// The local half of decision 6: no network, no registry, just the file this run already resolved, written out so a second `culm new --preset ./NAME.toml`-shaped workflow does not have to re-list every layer by hand.
    #[arg(long, value_name = "NAME")]
    pub save_preset: Option<String>,
}

/// What a `culm new` run learned about itself, for telemetry.
/// Filled as the run goes, so a failure still reports how far it got.
#[derive(Debug, Default)]
pub struct NewReport {
    pub scaffold: Scaffold,
    /// The stage the run is in; the stage it failed in, if it failed.
    pub stage: ErrorKind,
    /// The user backed out, in the wizard or at the confirmation prompt.
    pub cancelled: bool,
}

/// Scaffolds a new workspace into `parent/<project>`.
///
/// `parent` is separate from the arguments because the binary takes it from the
/// working directory and the tests take it from a temporary one. A flag for it
/// would be a flag that exists only to be tested.
pub fn run_new(args: &NewArgs, parent: &Path) -> Result<()> {
    run_new_to(args, parent, &mut io::stdout())
}

/// The same run, reporting into a writer the caller owns.
///
/// Split out so a test can assert what the user is told rather than only what
/// landed on disk. The `--no-install` plan is the case that matters: its whole
/// output is the report.
pub fn run_new_to(args: &NewArgs, parent: &Path, out: &mut impl Write) -> Result<()> {
    run_new_reporting(args, parent, out, &mut NewReport::default())
}

/// The same run, also filling `report` for telemetry.
/// Only `main.rs` reads the report; tests go through `run_new_to`.
pub fn run_new_reporting(
    args: &NewArgs,
    parent: &Path,
    out: &mut impl Write,
    report: &mut NewReport,
) -> Result<()> {
    report.stage = ErrorKind::Select;
    let project = project_name(&args.project)?;

    let catalog = match &args.layers_dir {
        Some(dir) => Catalog::load(dir)
            .with_context(|| format!("cannot load layers from {}", dir.display()))?,
        None => Catalog::embedded().context("the compiled-in layer catalog is unreadable")?,
    };

    let chosen = chosen_layers(args, &catalog, report)?;
    report.scaffold.layers = first_party(chosen.iter().map(LayerId::to_string));

    report.stage = ErrorKind::Resolve;
    let opt_outs = ids(&args.without);
    let entries = resolve(&catalog, &chosen, &opt_outs)?;
    report.scaffold.layers = first_party(
        entries
            .iter()
            .map(|entry| entry.manifest.qualified_id().to_string()),
    );
    report.scaffold.shape = entries
        .iter()
        .find(|entry| entry.manifest.category == Category::Shape)
        .map(|entry| entry.manifest.id.clone());

    let root = parent.join(project);
    check_destination(&root)?;

    /* Everything is assembled and validated before anything touches disk, so a
     * refusal leaves no half-written directory behind.
     *
     * The layout is settled before a single manifest is used, because how many
     * applications the selection holds decides each one's directory and script
     * names. Interpolating the manifest rather than the output is what lets a
     * merge value carry a placeholder: substitution happens per layer, before
     * anything is merged, so two layers still contribute distinct values where
     * they should and identical ones where they should. */
    let variables = scaffold::variables(&entries, project);
    let manifests = entries
        .iter()
        .zip(&variables)
        .map(|(entry, vars)| entry.interpolated(vars))
        .collect::<Result<Vec<Manifest>, _>>()?;

    report.stage = ErrorKind::Merge;
    let layers = manifests
        .iter()
        .map(Manifest::to_layer)
        .collect::<Result<Vec<_>, _>>()?;
    let file_set = FileSet::build(&layers)?;
    let modes = file_set.modes();
    let merged = file_set.render();

    let planned: Vec<(&Manifest, &LayerSource, &Variables)> = manifests
        .iter()
        .zip(entries.iter())
        .zip(&variables)
        .map(|((manifest, entry), vars)| (manifest, &entry.source, vars))
        .collect();
    let plan = CopyPlan::build(planned)?;

    /* Interpolated, not the raw `entries`: a placeholder in a `[[files]] to`
     * path (`{{app_dir}}/go.mod`) is real text only after substitution, and
     * `install::plan` reads that path to find where a `go get`/`cargo add`
     * belongs. This never surfaced before `backend/go-chi`, because every
     * Rust layer's `Cargo.toml` path is a literal `crates/<id>/` rather than
     * a templated one. */
    let interpolated_entries: Vec<CatalogEntry> = entries
        .iter()
        .zip(&manifests)
        .map(|(entry, manifest)| CatalogEntry {
            manifest: manifest.clone(),
            source: entry.source.clone(),
        })
        .collect();
    let installs = install::plan(&interpolated_entries.iter().collect::<Vec<_>>())?;

    /* A merge that rendered to nothing is never written (see scaffold::write),
     * so it does not belong in "N files merged" either. */
    let written_merges = merged
        .values()
        .filter(|contents| !contents.is_empty())
        .count();
    summarise(out, &entries, plan.len(), written_merges, &root)?;

    if !args.yes && !confirmed(out)? {
        report.cancelled = true;
        report.scaffold.cancelled_at = Some("confirm".to_owned());
        writeln!(out, "Nothing was written.")?;
        return Ok(());
    }

    report.stage = ErrorKind::Write;
    write(&root, &plan, &merged, &modes)?;
    set_pnpm_release_age(&root, args.min_release_age)?;

    /* After the scaffold itself, never before: a backend needs to see each
     * application's final `app_dir`, which is only known once the layout in
     * `variables` above has already been decided. Decision 3's own reason
     * these backends exist at all: `none` must stay a legitimate answer, so
     * a selection without a real backend writes nothing here. */
    runner::write_root_config(&entries, &variables, &root)?;

    writeln!(out, "\nScaffolded {} into {}", project, root.display())?;
    finish_installs(out, &installs, &root, args, report)?;

    if selected_runner_id(&entries) == Some("turborepo") {
        if args.no_install {
            writeln!(
                out,
                "\nTurborepo needs a `packageManager` field pinned in package.json.\n\
                 Run `pnpm --version` and add \"packageManager\": \"pnpm@<version>\" yourself."
            )?;
        } else {
            install::apply_package_manager_pin(&root)
                .context("cannot pin the package manager Turborepo requires")?;
        }
    }

    if let Some(name) = &args.save_preset {
        let layers = entries
            .iter()
            .map(|entry| entry.manifest.qualified_id().to_string())
            .collect();
        report.stage = ErrorKind::Write;
        let saved = save_preset(name, layers, parent)?;
        report.scaffold.saved_preset = true;
        writeln!(out, "\nSaved this selection as {}", saved.display())?;
    }

    Ok(())
}

/// Writes the resolved selection out as a preset, for a later `culm new
/// --preset ./NAME.toml`.
///
/// Written beside the generated workspace, not inside it: a preset describes
/// how to build a workspace, so it belongs with the command that produced it,
/// not as a file the generated project would otherwise have to `.gitignore`
/// or explain to itself.
fn save_preset(name: &str, layers: Vec<String>, parent: &Path) -> Result<PathBuf> {
    let preset = preset::Preset {
        name: name.to_owned(),
        description: format!("Saved by `culm new --save-preset {name}`."),
        layers,
    };
    let path = parent.join(format!("{name}.toml"));
    let body = toml::to_string_pretty(&preset).context("cannot serialise the saved preset")?;
    std::fs::write(&path, body).with_context(|| format!("cannot write {}", path.display()))?;
    Ok(path)
}

/// Runs the install plan, or prints it when the user asked not to run it.
///
/// Printing the exact argv rather than a sentence about package managers is
/// what makes `--no-install` useful: the output is something to paste, and it
/// is the same thing culm would have run, less the versions the release age
/// guard would have chosen, since choosing them needs the network.
fn finish_installs(
    out: &mut impl Write,
    commands: &[install::InstallCommand],
    root: &Path,
    args: &NewArgs,
    report: &mut NewReport,
) -> Result<()> {
    if commands.is_empty() {
        report.scaffold.install = Some(Install::None);
        return Ok(());
    }

    if args.no_install {
        report.scaffold.install = Some(Install::Skipped);
        writeln!(out, "\nInstalls skipped. Run these in {}:", root.display())?;
        for command in commands {
            writeln!(out, "  {}", command.display())?;
        }
        return Ok(());
    }

    let (commands, cargo_locks) = if args.min_release_age == 0 {
        (commands.to_vec(), Vec::new())
    } else {
        report.stage = ErrorKind::VersionResolve;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX))
            .unwrap_or(0);
        let pinned = install::release_age::pin(commands, args.min_release_age, now)
            .map_err(|message| CliError::ReleaseAge { message })?;
        report.scaffold.held_back = Some(pinned.held_back);
        (pinned.commands, pinned.cargo_locks)
    };

    report.stage = ErrorKind::Install;
    let install_started = std::time::Instant::now();
    let installed = install_all(&commands, &cargo_locks, root, out, &mut report.scaffold);
    report.scaffold.install_ms =
        Some(u64::try_from(install_started.elapsed().as_millis()).unwrap_or(u64::MAX));
    installed?;
    report.scaffold.install = Some(Install::Ok);
    Ok(())
}

/// Runs every install command, then moves each crate the guard pinned back to its chosen version.
/// Separate so the caller can time the whole phase, failures included.
fn install_all(
    commands: &[install::InstallCommand],
    cargo_locks: &[(Option<String>, String, String)],
    root: &Path,
    out: &mut impl Write,
    report: &mut Scaffold,
) -> Result<()> {
    writeln!(out, "\nInstalling dependencies:")?;
    for command in commands {
        writeln!(out, "  {}", command.display())?;
    }
    run_each(commands, root, report)?;

    /* After every add, when `Cargo.lock` exists: move each crate the guard
     * chose back to that version, since the lockfile resolved the newest. */
    if !cargo_locks.is_empty() {
        let (updates, skipped) = install::release_age::precise_updates(root, cargo_locks);
        for command in &updates {
            writeln!(out, "  {}", command.display())?;
        }
        run_each(&updates, root, report)?;
        for name in skipped {
            writeln!(
                out,
                "  note: `{name}` is locked at more than one version, so the release age guard left its lockfile entry alone"
            )?;
        }
    }
    return Ok(());
}

/* One at a time, so the manager that failed is known without reading the
 * error text: only its program name is reported, never its arguments. */
fn run_each(
    commands: &[install::InstallCommand],
    root: &Path,
    report: &mut Scaffold,
) -> Result<()> {
    for command in commands {
        if let Err(error) = install::run(std::slice::from_ref(command), root) {
            report.install = Some(Install::Failed);
            report.install_failed_manager = Some(command.program.clone());
            return Err(error.into());
        }
    }
    Ok(())
}

/// `toolchain/node-pnpm` writes decision 5's default guard into
/// `pnpm-workspace.yaml`; a different `--min-release-age` replaces it, in
/// minutes, which is the unit pnpm reads.
fn set_pnpm_release_age(root: &Path, days: u64) -> Result<()> {
    if days == install::release_age::DEFAULT_DAYS {
        return Ok(());
    }
    let path = root.join("pnpm-workspace.yaml");
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return Ok(());
    };
    let default = format!(
        "minimumReleaseAge: {}\n",
        install::release_age::DEFAULT_DAYS * 1440
    );
    let chosen = format!("minimumReleaseAge: {}\n", days * 1440);
    std::fs::write(&path, contents.replace(&default, &chosen))
        .with_context(|| format!("cannot write {}", path.display()))
}

/// Rejects a project name that is not a single, plain directory segment.
/// The name becomes a path component and a template variable, so a separator or a `..` in it is a way to write outside the directory the user named.
fn project_name(raw: &str) -> Result<&str> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(CliError::ProjectNameEmpty.into());
    }
    if name == "." || name == ".." {
        return Err(CliError::ProjectNameUnusable {
            name: name.to_owned(),
        }
        .into());
    }
    if name.contains('/') || name.contains('\\') {
        return Err(CliError::ProjectNameIsPath {
            name: name.to_owned(),
        }
        .into());
    }
    if name.starts_with('-') {
        return Err(CliError::ProjectNameStartsWithDash {
            name: name.to_owned(),
        }
        .into());
    }
    Ok(name)
}

/// The bare id of whichever `runner/*` layer this selection carries, or
/// `None` when for some reason it carries none (never happens for a real
/// selection, since `Category::Runner` is `ExactlyOne`, but `resolve` is
/// what enforces that, not this function).
fn selected_runner_id<'a>(entries: &'a [&CatalogEntry]) -> Option<&'a str> {
    entries
        .iter()
        .find(|entry| entry.manifest.category == Category::Runner)
        .map(|entry| entry.manifest.id.as_str())
}

fn ids(raw: &[String]) -> Vec<LayerId> {
    raw.iter().map(LayerId::new).collect()
}

/// The layers the run starts from, whether they were listed, named by a
/// preset, or picked in the wizard.
///
/// A preset resolves to ids here and then takes the ordinary path, so nothing
/// downstream has to know a preset was involved. Any `--layer` given with a
/// preset extends it; a layer the preset already names is not repeated, and a
/// clash such as a second shape is left to validation to report by name.
///
/// The wizard is the fallback when neither is given, and only when both
/// stdin and stdout are real terminals: a script that forgot `--layer` gets
/// the same clear error it always has, never a hang waiting for keystrokes
/// nothing will send. This is also what keeps `cargo test` from ever
/// launching it, the same way `banner::print` never draws under a test
/// harness.
fn chosen_layers(
    args: &NewArgs,
    catalog: &Catalog,
    report: &mut NewReport,
) -> Result<Vec<LayerId>> {
    match (&args.preset, args.layers.is_empty()) {
        (Some(source), _) => {
            report.scaffold.selected_via = Some(SelectedVia::Preset);
            report.scaffold.preset = Some(preset_label(source));
            let mut chosen = resolve_preset(source, report)?.ids();
            for extra in ids(&args.layers) {
                if !chosen.contains(&extra) {
                    chosen.push(extra);
                }
            }
            Ok(chosen)
        }
        (None, false) => {
            report.scaffold.selected_via = Some(SelectedVia::Flags);
            Ok(ids(&args.layers))
        }
        (None, true) if is_interactive() => {
            report.scaffold.selected_via = Some(SelectedVia::Wizard);
            match crate::wizard::run(catalog)? {
                crate::wizard::Outcome::Chosen { layers, changed } => {
                    report.scaffold.wizard_changed = Some(changed);
                    Ok(ids(&layers))
                }
                crate::wizard::Outcome::Cancelled(step) => {
                    report.cancelled = true;
                    report.scaffold.cancelled_at = Some(step.to_string());
                    Err(CliError::Cancelled.into())
                }
            }
        }
        (None, true) => Err(CliError::NoLayersChosen.into()),
    }
}

fn is_interactive() -> bool {
    use std::io::IsTerminal;
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

/// A preset by first-party name, by local file, or by git URL, in that order.
///
/// A local file is checked first and only when the string plainly looks like
/// one (ends in `.toml`, and a first-party preset never does since it is
/// named bare), so a real preset called `team` is never shadowed by a
/// same-named file sitting in the working directory by coincidence.
fn resolve_preset(source: &str, report: &mut NewReport) -> Result<preset::Preset> {
    if source.ends_with(".toml") {
        let contents = std::fs::read_to_string(source)
            .with_context(|| format!("cannot read preset file `{source}`"))?;
        return toml::from_str(&contents)
            .with_context(|| format!("`{source}` is not a valid preset"));
    }

    if let Some(remote) = preset::remote::parse_source(source) {
        report.stage = ErrorKind::PresetFetch;
        let cache_root = preset::remote::default_cache_root()
            .context("cannot find a cache directory for a remote preset on this platform")?;
        return preset::remote::fetch(&remote, &cache_root)
            .with_context(|| format!("cannot fetch preset `{source}`"));
    }

    Ok(preset::get(source)?)
}

/// How a preset is reported: its name when it is first-party, otherwise only
/// whether it came from a file or a git URL. The source string itself can be
/// a repository name or a path, both of which decision 9 forbids sending.
fn preset_label(source: &str) -> String {
    if source.ends_with(".toml") {
        return "file".to_owned();
    }
    if preset::remote::parse_source(source).is_some() {
        return "git".to_owned();
    }
    if preset::get(source).is_ok() {
        return source.to_owned();
    }
    /* Neither a file, a git URL, nor a shipped preset: the run is about to
     * fail, and the string is whatever the user typed, so it is not sent. */
    "unknown".to_owned()
}

/// Keeps only layer ids that ship in the binary.
/// A `--layers-dir` catalog or a typo can name anything, and a private
/// layer's id is nobody else's business.
fn first_party(ids: impl IntoIterator<Item = String>) -> Vec<String> {
    let Ok(shipped) = Catalog::embedded() else {
        return Vec::new();
    };
    ids.into_iter()
        .filter(|id| shipped.get(&LayerId::new(id)).is_some())
        .collect()
}

/// Refuses anything that would scaffold on top of existing work.
/// An empty directory is fine, because creating one and then running culm in it is a reasonable thing to have done.
fn check_destination(root: &Path) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }
    if !root.is_dir() {
        return Err(CliError::DestinationNotADirectory {
            path: root.to_owned(),
        }
        .into());
    }
    let mut existing = root
        .read_dir()
        .with_context(|| format!("cannot read {}", root.display()))?;
    if existing.next().is_some() {
        return Err(CliError::DestinationNotEmpty {
            path: root.to_owned(),
        }
        .into());
    }
    Ok(())
}

fn summarise(
    out: &mut impl Write,
    entries: &[&CatalogEntry],
    copied: usize,
    merged: usize,
    root: &Path,
) -> io::Result<()> {
    writeln!(out, "Scaffolding into {}", root.display())?;
    writeln!(out, "\n{} layers:", entries.len())?;
    for entry in entries {
        writeln!(out, "  {}", entry.manifest.qualified_id())?;
    }
    writeln!(out, "\n{copied} files copied, {merged} files merged")?;
    Ok(())
}

/// Asks before writing, using plain stdio.
/// No prompt library: the wizard is a later decision, and pulling one in now would make it look settled.
fn confirmed(out: &mut impl Write) -> Result<bool> {
    write!(out, "\nWrite this workspace? [y/N] ")?;
    out.flush()?;

    let mut answer = String::new();
    io::stdin()
        .lock()
        .read_line(&mut answer)
        .context("cannot read a confirmation from stdin")?;
    let answer = answer.trim().to_ascii_lowercase();
    Ok(answer == "y" || answer == "yes")
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_name_is_accepted() {
        assert_eq!(project_name("klassivox").unwrap(), "klassivox");
        assert_eq!(project_name("  demo  ").unwrap(), "demo");
    }

    /* The name is both a path segment and a template variable, so this is the
     * same class of hole the copy plan's `normalise` closes. */
    #[test]
    fn a_name_that_is_really_a_path_is_refused() {
        assert!(project_name("../etc").is_err());
        assert!(project_name("a/b").is_err());
        assert!(project_name("a\\b").is_err());
        assert!(project_name("..").is_err());
        assert!(project_name(".").is_err());
    }

    #[test]
    fn an_empty_name_is_refused() {
        assert!(project_name("").is_err());
        assert!(project_name("   ").is_err());
    }

    /* Otherwise the name is indistinguishable from a flag in any command the
     * user later copies out of a script. */
    #[test]
    fn a_name_starting_with_a_dash_is_refused() {
        assert!(project_name("-rf").is_err());
    }

    /* Decision 9: a preset source can be a repository name or a path, so
     * only a first-party name ever leaves the machine as itself. */
    #[test]
    fn a_preset_is_reported_without_its_source() {
        assert_eq!(preset_label("monorepo"), "monorepo");
        assert_eq!(preset_label("github:someone/private-repo"), "git");
        assert_eq!(preset_label("./team/our-setup.toml"), "file");
        assert_eq!(preset_label("not-a-real-preset"), "unknown");
    }

    #[test]
    fn only_shipped_layers_are_reported() {
        let reported = first_party(
            ["frontend/nuxt", "frontend/acme-internal", "typo"]
                .into_iter()
                .map(str::to_owned),
        );
        assert_eq!(reported, vec!["frontend/nuxt".to_owned()]);
    }
}
