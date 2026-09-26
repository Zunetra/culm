// === Task caching (r0.2)

/* docs/DESIGN.md section 3.3: hash inputs by content, check the cache,
 * restore on a hit, run and record on a miss. A cache miss must always be
 * safe and a cache hit must always be correct, so every doubt resolves
 * toward running the task:
 *
 * - A task with no `inputs` is never cached. Its key would name nothing it
 *   reads, so it would hit forever.
 * - A task whose dependency has no key is never cached either. Its inputs
 *   may be that dependency's outputs, which nothing hashed.
 * - Only a successful run is recorded.
 *
 * A key is computed just before its task runs, after every dependency has
 * finished, because a task's inputs are often its dependencies' outputs. */

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Stdio};
use std::time::Instant;

use super::error::RunnerError;
use super::exec::shell;
use super::inputs;
use super::task::Task;
use crate::cas::{CacheEntry, CacheKey, Captured, Hash, Store, write_outputs};

/// How a run treats the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheMode {
    /// Restore on a hit, record on a miss.
    ReadWrite,
    /// Run everything, read and write nothing. `--no-cache`.
    Off,
    /// Run everything, and fail if a task's output differs from what its
    /// cache entry holds. `--verify`, the tool for the r0.2 hand check.
    Verify,
}

/// The local cache a run reads and writes.
#[derive(Debug, Clone)]
pub struct Cache {
    store: Store,
    mode: CacheMode,
}

impl Cache {
    pub fn open(root: &Path, mode: CacheMode) -> Result<Self, RunnerError> {
        let store = Store::open(root).map_err(cache_error)?;
        Ok(Cache { store, mode })
    }
}

/// How one task was satisfied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Restored from the cache, or, under `--verify`, run and matched.
    Hit,
    /// Run for real and recorded.
    Miss,
    /// Run for real; not cacheable, or caching is off.
    Uncached,
}

/// What to do for a task, decided before it runs.
#[derive(Debug)]
pub enum Plan {
    /// Already restored. Nothing to run.
    Restored,
    /// Run it.
    Run(Box<Pending>),
}

/// A task about to run for real.
#[derive(Debug)]
pub struct Pending {
    /// `Some` only when the result should be recorded or verified.
    pub key: Option<CacheKey>,
    /// Under `--verify`, the entry the run must match.
    pub previous: Option<CacheEntry>,
}

impl Plan {
    fn run(key: Option<CacheKey>, previous: Option<CacheEntry>) -> Self {
        Plan::Run(Box::new(Pending { key, previous }))
    }
}

/// The cache-related state of one `culm run`.
#[derive(Debug)]
pub struct Session<'a> {
    cache: Option<&'a Cache>,
    /// Each task's key digest once decided, `None` when it has none.
    keys: HashMap<String, Option<Hash>>,
    pub hits: u32,
    pub misses: u32,
}

impl<'a> Session<'a> {
    pub fn new(cache: Option<&'a Cache>) -> Self {
        Session {
            cache,
            keys: HashMap::new(),
            hits: 0,
            misses: 0,
        }
    }

    pub fn cache(&self) -> Option<&'a Cache> {
        self.cache
    }

    /// Computes `task`'s key, restores it if the cache holds it, and prints
    /// the line naming the task. Every dependency must already have finished.
    pub fn plan(&mut self, task: &Task, out: &mut impl Write) -> Result<Plan, RunnerError> {
        let key = self.key_for(task)?;
        self.keys
            .insert(task.name.clone(), key.as_ref().map(CacheKey::digest));

        if task.cacheable && key.is_none() {
            writeln!(
                out,
                "  note: `{}` has cache = true but cannot be cached safely: it needs `inputs`, and so does every task it depends on",
                task.name
            )
            .ok();
        }

        let (Some(cache), true, Some(key)) = (self.cache, task.cacheable, key) else {
            writeln!(out, "> {} ({})", task.name, task.command).ok();
            return Ok(Plan::run(None, None));
        };

        match cache.mode {
            CacheMode::Off => {
                writeln!(out, "> {} ({})", task.name, task.command).ok();
                Ok(Plan::run(None, None))
            }
            CacheMode::Verify => {
                writeln!(out, "> {} ({}) [verifying]", task.name, task.command).ok();
                let previous = CacheEntry::lookup(&key, &cache.store).map_err(cache_error)?;
                Ok(Plan::run(Some(key), previous))
            }
            CacheMode::ReadWrite => {
                match CacheEntry::lookup(&key, &cache.store).map_err(cache_error)? {
                    Some(entry) => {
                        writeln!(out, "> {} ({}) [cache hit]", task.name, task.command).ok();
                        restore(task, &entry, cache)?;
                        Ok(Plan::Restored)
                    }
                    None => {
                        writeln!(out, "> {} ({}) [cache miss]", task.name, task.command).ok();
                        Ok(Plan::run(Some(key), None))
                    }
                }
            }
        }
    }

    pub fn count(&mut self, status: Status) {
        match status {
            Status::Hit => self.hits += 1,
            Status::Miss => self.misses += 1,
            Status::Uncached => {}
        }
    }

    /// Prints the one-line summary, when anything was cacheable at all.
    pub fn summarise(&self, out: &mut impl Write) {
        if self.hits + self.misses > 0 {
            writeln!(
                out,
                "\n{} cached task(s): {} from cache, {} run",
                self.hits + self.misses,
                self.hits,
                self.misses
            )
            .ok();
        }
    }

    fn key_for(&self, task: &Task) -> Result<Option<CacheKey>, RunnerError> {
        inputs::check_outputs(&task.name, &task.outputs)?;
        if task.inputs.is_empty() {
            return Ok(None);
        }

        let mut dependencies = std::collections::BTreeMap::new();
        for dependency in &task.depends_on {
            match self.keys.get(dependency) {
                Some(Some(digest)) => {
                    dependencies.insert(dependency.clone(), digest.to_hex().to_string());
                }
                /* A dependency with no key, or one this run never planned
                 * (`--affected` skipped it), cannot vouch for its outputs. */
                _ => return Ok(None),
            }
        }

        Ok(Some(CacheKey {
            task: task.name.clone(),
            command: task.command.clone(),
            input_hashes: inputs::hash_inputs(&task.dir, &task.inputs)?,
            declared_env: task
                .declared_env
                .iter()
                .map(|name| (name.clone(), std::env::var(name).ok()))
                .collect(),
            dependencies,
            platform: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
            tool_version: env!("CARGO_PKG_VERSION").to_owned(),
        }))
    }
}

/// Runs a planned task. Safe to call from several threads at once.
pub fn run(
    task: &Task,
    cache: Option<&Cache>,
    key: Option<&CacheKey>,
    previous: Option<&CacheEntry>,
) -> Result<Status, RunnerError> {
    let (Some(cache), Some(key)) = (cache, key) else {
        let status = shell(&task.command)
            .current_dir(&task.dir)
            .status()
            .map_err(|e| not_started(task, e))?;
        check(task, status)?;
        return Ok(Status::Uncached);
    };

    let started = Instant::now();
    let (status, stdout, stderr) = run_captured(task)?;
    check(task, status)?;

    let captured = Captured {
        outputs: inputs::collect_outputs(&task.dir, &task.outputs)?,
        stdout,
        stderr,
        exit_code: status.code().unwrap_or(0),
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    };

    if let Some(previous) = previous {
        verify(task, previous, &captured)?;
        return Ok(Status::Hit);
    }
    CacheEntry::record(key, &captured, &cache.store).map_err(cache_error)?;
    Ok(Status::Miss)
}

/// Replaces the task's declared outputs with the recorded ones and replays
/// its logs, so a hit looks like the run it stands in for.
fn restore(task: &Task, entry: &CacheEntry, cache: &Cache) -> Result<(), RunnerError> {
    /* Read and verify every object before deleting anything: a corrupted
     * store must fail the restore, not leave the outputs half gone. */
    let (stdout, stderr) = entry.logs(&cache.store).map_err(cache_error)?;
    let outputs = entry.load(&cache.store).map_err(cache_error)?;

    inputs::clear_outputs(&task.dir, &task.outputs)?;
    write_outputs(&outputs, &task.dir).map_err(cache_error)?;
    io::stdout().write_all(&stdout).ok();
    io::stderr().write_all(&stderr).ok();
    Ok(())
}

/// A verified run must have produced exactly the recorded outputs, byte for
/// byte, with nothing extra and nothing missing.
fn verify(task: &Task, previous: &CacheEntry, run: &Captured) -> Result<(), RunnerError> {
    let mut differing: Vec<String> = Vec::new();
    for output in &run.outputs {
        let digest = crate::cas::hash(&output.bytes).to_hex().to_string();
        if previous.outputs.get(&output.path) != Some(&digest) {
            differing.push(output.path.display().to_string());
        }
    }
    let produced: Vec<&PathBuf> = run.outputs.iter().map(|o| &o.path).collect();
    for path in previous.outputs.keys() {
        if !produced.contains(&path) {
            differing.push(path.display().to_string());
        }
    }
    if differing.is_empty() {
        return Ok(());
    }
    Err(RunnerError::CacheMismatch {
        task: task.name.clone(),
        paths: differing,
    })
}

/// Runs the command with its output shown live and also kept, since a hit
/// must replay exactly what a real run printed.
fn run_captured(task: &Task) -> Result<(ExitStatus, Vec<u8>, Vec<u8>), RunnerError> {
    let mut child = shell(&task.command)
        .current_dir(&task.dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| not_started(task, e))?;

    let stdout = child.stdout.take().expect("stdout was piped");
    let stderr = child.stderr.take().expect("stderr was piped");
    let stdout = std::thread::spawn(move || tee(stdout, io::stdout()));
    let stderr = std::thread::spawn(move || tee(stderr, io::stderr()));

    let status = child.wait().map_err(|e| not_started(task, e))?;
    let stdout = stdout.join().expect("the stdout reader must not panic");
    let stderr = stderr.join().expect("the stderr reader must not panic");
    Ok((status, stdout, stderr))
}

fn tee(mut from: impl Read, mut to: impl Write) -> Vec<u8> {
    let mut kept = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        match from.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                to.write_all(&buffer[..n]).ok();
                kept.extend_from_slice(&buffer[..n]);
            }
        }
    }
    to.flush().ok();
    kept
}

fn check(task: &Task, status: ExitStatus) -> Result<(), RunnerError> {
    if status.success() {
        return Ok(());
    }
    Err(RunnerError::TaskFailed {
        task: task.name.clone(),
        status: status.code(),
    })
}

fn not_started(task: &Task, error: io::Error) -> RunnerError {
    RunnerError::NotStarted {
        command: task.command.clone(),
        message: error.to_string(),
    }
}

fn cache_error(error: crate::cas::CasError) -> RunnerError {
    RunnerError::Cache {
        message: error.to_string(),
    }
}
