// === culm cache

/* r0.2, per docs/02-runner.md section 3's own interface: `culm cache
 * stats|clear`, inspecting or emptying the local `cas` store this machine
 * keeps. Neither subcommand touches a `culm.toml`: the cache is one store
 * per machine, not one per project, so both work from anywhere. */

use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::cas::Store;

#[derive(Debug, Args)]
pub struct CacheArgs {
    #[command(subcommand)]
    pub command: CacheCommand,

    /// Use this directory as the cache instead of the OS cache directory.
    /// For the tests, the same way `--layers-dir` lets `culm new` be pointed elsewhere.
    #[arg(long, value_name = "PATH")]
    pub root: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum CacheCommand {
    /// Reports how many objects the local cache holds and their total size.
    Stats,
    /// Empties the local cache. The next run of any cacheable task is a miss.
    Clear,
}

pub fn run_cache(args: &CacheArgs) -> Result<()> {
    run_cache_to(args, &mut io::stdout())
}

pub fn run_cache_to(args: &CacheArgs, out: &mut impl Write) -> Result<()> {
    let root = match &args.root {
        Some(root) => root.clone(),
        None => Store::default_root().context("cannot find a cache directory for this platform")?,
    };
    let store = Store::open(&root).with_context(|| format!("cannot open {}", root.display()))?;

    match args.command {
        CacheCommand::Stats => {
            let stats = store.stats().context("cannot read the local cache")?;
            writeln!(out, "{} objects, {} bytes, at {}", stats.object_count, stats.total_bytes, root.display())?;
        }
        CacheCommand::Clear => {
            store.clear().context("cannot clear the local cache")?;
            writeln!(out, "Cleared {}", root.display())?;
        }
    }
    Ok(())
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn args(command: CacheCommand, root: &std::path::Path) -> CacheArgs {
        CacheArgs {
            command,
            root: Some(root.to_owned()),
        }
    }

    #[test]
    fn stats_on_an_empty_store_reports_zero() {
        let dir = tempfile::tempdir().unwrap();
        let mut out: Vec<u8> = Vec::new();
        run_cache_to(&args(CacheCommand::Stats, dir.path()), &mut out).unwrap();
        assert!(String::from_utf8(out).unwrap().contains("0 objects"));
    }

    #[test]
    fn clear_leaves_the_store_open_and_empty() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        store.put(b"one").unwrap();

        let mut out: Vec<u8> = Vec::new();
        run_cache_to(&args(CacheCommand::Clear, dir.path()), &mut out).unwrap();

        assert_eq!(store.stats().unwrap().object_count, 0);
    }
}
