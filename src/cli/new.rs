// === culm new

/* The whole scaffold, driven by flags rather than by a wizard. The wizard is
 * deliberately last: flags have to work anyway, they are what CI uses, and a
 * tool whose only entry point is a set of prompts cannot be scripted. */

use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Args;

use crate::install;
use crate::layer::{Catalog, CatalogEntry, LayerSource, Manifest, resolve};
use crate::merge::{FileSet, LayerId};
use crate::preset;
use crate::scaffold::{self, CopyPlan, Variables, write};

#[derive(Debug, Args)]
pub struct NewArgs {
    /// Name of the project, used as the directory name and as `{{project_name}}`.
    pub project: String,

    /// A layer to include, qualified as `<category>/<id>`. Repeat for several.
    #[arg(long = "layer", value_name = "ID")]
    pub layers: Vec<String>,

    /// A named selection of layers to start from, instead of listing them.
    /// Extending a preset with extra layers is a later question, so the two are exclusive for now.
    #[arg(long, value_name = "NAME", conflicts_with = "layers")]
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
    let project = project_name(&args.project)?;

    let chosen = chosen_layers(args)?;

    let catalog = match &args.layers_dir {
        Some(dir) => Catalog::load(dir)
            .with_context(|| format!("cannot load layers from {}", dir.display()))?,
        None => Catalog::embedded().context("the compiled-in layer catalog is unreadable")?,
    };

    let opt_outs = ids(&args.without);
    let entries = resolve(&catalog, &chosen, &opt_outs)?;

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
    let installs = install::plan(&entries)?;

    summarise(out, &entries, plan.len(), merged.len(), &root)?;

    if !args.yes && !confirmed(out)? {
        writeln!(out, "Nothing was written.")?;
        return Ok(());
    }

    write(&root, &plan, &merged, &modes)?;

    writeln!(out, "\nScaffolded {} into {}", project, root.display())?;
    finish_installs(out, &installs, &root, args.no_install)?;
    Ok(())
}

/// Runs the install plan, or prints it when the user asked not to run it.
///
/// Printing the exact argv rather than a sentence about package managers is
/// what makes `--no-install` useful: the output is something to paste, and it
/// is the same thing culm would have run.
fn finish_installs(
    out: &mut impl Write,
    commands: &[install::InstallCommand],
    root: &Path,
    skip: bool,
) -> Result<()> {
    if commands.is_empty() {
        return Ok(());
    }

    if skip {
        writeln!(out, "\nInstalls skipped. Run these in {}:", root.display())?;
        for command in commands {
            writeln!(out, "  {}", command.display())?;
        }
        return Ok(());
    }

    writeln!(out, "\nInstalling dependencies:")?;
    for command in commands {
        writeln!(out, "  {}", command.display())?;
    }
    install::run(commands, root)?;
    Ok(())
}

/// Rejects a project name that is not a single, plain directory segment.
/// The name becomes a path component and a template variable, so a separator or a `..` in it is a way to write outside the directory the user named.
fn project_name(raw: &str) -> Result<&str> {
    let name = raw.trim();
    if name.is_empty() {
        bail!("the project name is empty");
    }
    if name == "." || name == ".." {
        bail!("`{name}` is not a usable project name");
    }
    if name.contains('/') || name.contains('\\') {
        bail!("the project name `{name}` must be a single directory name, not a path");
    }
    if name.starts_with('-') {
        bail!("the project name `{name}` must not start with a dash");
    }
    Ok(name)
}

fn ids(raw: &[String]) -> Vec<LayerId> {
    raw.iter().map(LayerId::new).collect()
}

/// The layers the run starts from, whether they were listed or named by a preset.
///
/// A preset resolves to ids here and then takes the ordinary path, so nothing
/// downstream has to know a preset was involved. Clap keeps the two flags
/// exclusive, and the check is repeated because `run_new` is also called
/// directly by the tests.
fn chosen_layers(args: &NewArgs) -> Result<Vec<LayerId>> {
    match (&args.preset, args.layers.is_empty()) {
        (Some(_), false) => bail!("--preset and --layer cannot be combined"),
        (Some(name), true) => Ok(preset::get(name)?.ids()),
        (None, false) => Ok(ids(&args.layers)),
        (None, true) => bail!(
            "no layers chosen: pass --preset NAME, or at least one --layer, for example --layer backend/axum"
        ),
    }
}

/// Refuses anything that would scaffold on top of existing work.
/// An empty directory is fine, because creating one and then running culm in it is a reasonable thing to have done.
fn check_destination(root: &Path) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }
    if !root.is_dir() {
        bail!("{} already exists and is not a directory", root.display());
    }
    let mut existing = root
        .read_dir()
        .with_context(|| format!("cannot read {}", root.display()))?;
    if existing.next().is_some() {
        bail!("{} already exists and is not empty", root.display());
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
}
