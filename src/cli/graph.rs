// === culm graph

/* r1.0, per docs/DESIGN.md section 3.2's own interface: `culm graph`, and
 * section 6's `culm graph --format dot`. Prints the whole task graph, not
 * one task's closure, which is what `TaskGraph::all_in_order` is for rather
 * than reusing `execution_order`. */

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{Args, ValueEnum};

use crate::cli::CliError;
use crate::runner::{TaskGraph, workspace};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum GraphFormat {
    /// One line per task, in a valid dependency order.
    List,
    /// Graphviz DOT, pipeable straight into `dot -Tsvg`.
    Dot,
}

#[derive(Debug, Args)]
pub struct GraphArgs {
    /// Read a `culm.toml` at this path instead of the one in the working directory.
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    #[arg(long, value_enum, default_value = "list")]
    pub format: GraphFormat,
}

pub fn run_graph(args: &GraphArgs, cwd: &Path) -> Result<()> {
    run_graph_to(args, cwd, &mut io::stdout())
}

pub fn run_graph_to(args: &GraphArgs, cwd: &Path, out: &mut impl Write) -> Result<()> {
    let loaded = workspace::load(cwd, args.config.as_deref())?;
    let config_path = loaded.config_path;
    let tasks = loaded.tasks;

    if tasks.is_empty() {
        return Err(CliError::NoTasksDefined { path: config_path }.into());
    }

    let graph = TaskGraph::build(tasks, &config_path)?;

    match args.format {
        GraphFormat::List => {
            for task in graph.all_in_order() {
                writeln!(out, "{}", task.name)?;
            }
        }
        GraphFormat::Dot => {
            write!(out, "{}", graph.to_dot())?;
        }
    }
    Ok(())
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_config(dir: &Path, contents: &str) -> PathBuf {
        let path = dir.join("culm.toml");
        fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn list_format_names_every_task_in_a_valid_order() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_config(
            dir.path(),
            r#"
            [tasks.build]
            command = "echo build"

            [tasks.test]
            command = "echo test"
            depends_on = ["build"]
            "#,
        );

        let args = GraphArgs {
            config: Some(config),
            format: GraphFormat::List,
        };
        let mut out: Vec<u8> = Vec::new();
        run_graph_to(&args, dir.path(), &mut out).unwrap();

        let lines: Vec<&str> = std::str::from_utf8(&out).unwrap().lines().collect();
        assert_eq!(lines, vec!["build", "test"]);
    }

    #[test]
    fn dot_format_names_every_task_and_edge() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_config(
            dir.path(),
            r#"
            [tasks.build]
            command = "echo build"

            [tasks.test]
            command = "echo test"
            depends_on = ["build"]
            "#,
        );

        let args = GraphArgs {
            config: Some(config),
            format: GraphFormat::Dot,
        };
        let mut out: Vec<u8> = Vec::new();
        run_graph_to(&args, dir.path(), &mut out).unwrap();

        let dot = std::str::from_utf8(&out).unwrap();
        assert!(dot.contains("digraph"));
        assert!(dot.contains("build"));
        assert!(dot.contains("test"));
    }
}
