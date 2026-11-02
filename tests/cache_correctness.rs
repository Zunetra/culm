// === Cache correctness

/* docs/DESIGN.md section 3.6. These never trust culm's own "cache hit" line:
 * each one counts how many times a command really ran, in a file outside
 * every declared input and output, and compares the bytes a hit restored
 * against the bytes a real run wrote. A fast wrong answer is the failure
 * this whole suite exists to catch.
 *
 * Unix only: the fixture commands are `sh`. Run single-threaded in CI
 * (`--test-threads=1`), since each drives real subprocesses. */

#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};

use culm::cli::{RunArgs, run_task_to};

struct Fixture {
    project: tempfile::TempDir,
    /* Outside the project, so no input glob can see it. */
    scratch: tempfile::TempDir,
}

impl Fixture {
    fn new(config: &str) -> Self {
        let fixture = Fixture {
            project: tempfile::tempdir().unwrap(),
            scratch: tempfile::tempdir().unwrap(),
        };
        let config = config.replace("{count}", &fixture.counter().display().to_string());
        fs::write(fixture.project.path().join("culm.toml"), config).unwrap();
        fixture
    }

    fn root(&self) -> &Path {
        self.project.path()
    }

    fn counter(&self) -> PathBuf {
        self.scratch.path().join("count")
    }

    fn write(&self, path: &str, contents: &str) {
        let full = self.root().join(path);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, contents).unwrap();
    }

    fn read(&self, path: &str) -> Vec<u8> {
        fs::read(self.root().join(path)).unwrap()
    }

    /// How many times any fixture command has really run.
    fn runs(&self) -> usize {
        fs::read_to_string(self.counter())
            .map(|text| text.lines().count())
            .unwrap_or(0)
    }

    fn args(&self, task: &str) -> RunArgs {
        RunArgs {
            task: Some(task.to_owned()),
            affected: None,
            parallel: false,
            no_cache: false,
            verify: false,
            cache_dir: Some(self.scratch.path().join("cache")),
            config: Some(self.root().join("culm.toml")),
        }
    }

    fn run(&self, task: &str) -> String {
        self.run_with(self.args(task))
    }

    fn run_with(&self, args: RunArgs) -> String {
        let mut out = Vec::new();
        run_task_to(&args, self.root(), &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }
}

const BUILD: &str = r#"
[tasks.build]
command = "echo run >> {count} && mkdir -p out && cat src/a.txt src/b.txt > out/joined.txt && cp src/a.txt out/copy.txt"
inputs = ["src/*.txt"]
outputs = ["out"]
cache = true
"#;

#[test]
fn a_second_identical_run_is_restored_byte_for_byte_without_running() {
    let fixture = Fixture::new(BUILD);
    fixture.write("src/a.txt", "alpha\n");
    fixture.write("src/b.txt", "beta\n");

    fixture.run("build");
    assert_eq!(fixture.runs(), 1);
    let joined = fixture.read("out/joined.txt");
    let copy = fixture.read("out/copy.txt");

    fs::remove_dir_all(fixture.root().join("out")).unwrap();
    let report = fixture.run("build");

    assert_eq!(fixture.runs(), 1, "a hit must not run the command");
    assert!(report.contains("[cache hit]"), "{report}");
    assert_eq!(fixture.read("out/joined.txt"), joined);
    assert_eq!(fixture.read("out/copy.txt"), copy);
}

#[test]
fn changing_an_input_is_a_miss_and_the_new_output_is_real() {
    let fixture = Fixture::new(BUILD);
    fixture.write("src/a.txt", "alpha\n");
    fixture.write("src/b.txt", "beta\n");
    fixture.run("build");

    fixture.write("src/b.txt", "gamma\n");
    let report = fixture.run("build");

    assert_eq!(fixture.runs(), 2);
    assert!(report.contains("[cache miss]"), "{report}");
    assert_eq!(fixture.read("out/joined.txt"), b"alpha\ngamma\n");
}

#[test]
fn a_new_file_matching_an_input_glob_is_a_miss() {
    let fixture = Fixture::new(BUILD);
    fixture.write("src/a.txt", "alpha\n");
    fixture.write("src/b.txt", "beta\n");
    fixture.run("build");

    fixture.write("src/c.txt", "new\n");
    fixture.run("build");
    assert_eq!(fixture.runs(), 2);
}

#[test]
fn reverting_an_input_hits_the_earlier_entry_again() {
    let fixture = Fixture::new(BUILD);
    fixture.write("src/a.txt", "alpha\n");
    fixture.write("src/b.txt", "beta\n");
    fixture.run("build");
    fixture.write("src/b.txt", "gamma\n");
    fixture.run("build");

    fixture.write("src/b.txt", "beta\n");
    fixture.run("build");

    assert_eq!(fixture.runs(), 2);
    assert_eq!(fixture.read("out/joined.txt"), b"alpha\nbeta\n");
}

#[test]
fn a_restore_removes_stale_files_from_a_declared_output_directory() {
    let fixture = Fixture::new(BUILD);
    fixture.write("src/a.txt", "alpha\n");
    fixture.write("src/b.txt", "beta\n");
    fixture.run("build");

    fixture.write("out/stale.txt", "left over from some other build");
    fixture.run("build");

    assert!(!fixture.root().join("out/stale.txt").exists());
    assert!(fixture.root().join("out/joined.txt").exists());
}

#[test]
fn a_changed_dependency_reruns_its_dependent_even_when_its_own_inputs_did_not_change() {
    let fixture = Fixture::new(
        r#"
        [tasks.gen]
        command = "echo run >> {count} && mkdir -p gen && cp schema.txt gen/schema.txt"
        inputs = ["schema.txt"]
        outputs = ["gen"]
        cache = true

        [tasks.test]
        command = "echo run >> {count} && cat app.txt > report.txt"
        inputs = ["app.txt"]
        outputs = ["report.txt"]
        depends_on = ["gen"]
        cache = true
        "#,
    );
    fixture.write("schema.txt", "v1");
    fixture.write("app.txt", "app");
    fixture.run("test");
    assert_eq!(fixture.runs(), 2);

    fixture.run("test");
    assert_eq!(fixture.runs(), 2, "nothing changed, both hit");

    fixture.write("schema.txt", "v2");
    fixture.run("test");
    assert_eq!(fixture.runs(), 4, "gen changed, so test must rerun too");
}

#[test]
fn a_failed_run_is_never_recorded() {
    let fixture = Fixture::new(
        r#"
        [tasks.build]
        command = "echo run >> {count} && test -f ok.flag && mkdir -p out && echo done > out/x"
        inputs = ["src.txt"]
        outputs = ["out"]
        cache = true
        "#,
    );
    fixture.write("src.txt", "same input both times");

    let mut out = Vec::new();
    assert!(run_task_to(&fixture.args("build"), fixture.root(), &mut out).is_err());

    /* `ok.flag` is not an input, so the key is identical: only a recorded
     * failure could make this a hit. */
    fixture.write("ok.flag", "");
    fixture.run("build");
    assert_eq!(fixture.runs(), 2);
    assert_eq!(fixture.read("out/x"), b"done\n");
}

#[test]
fn a_task_with_no_inputs_is_never_cached() {
    let fixture = Fixture::new(
        r#"
        [tasks.build]
        command = "echo run >> {count}"
        cache = true
        "#,
    );
    fixture.run("build");
    let report = fixture.run("build");
    assert_eq!(fixture.runs(), 2);
    assert!(report.contains("cannot be cached safely"), "{report}");
}

#[test]
fn a_task_depending_on_an_unkeyed_task_is_never_cached() {
    let fixture = Fixture::new(
        r#"
        [tasks.fetch]
        command = "echo run >> {count} && wc -l < {count} > fetched.txt"

        [tasks.build]
        command = "echo run >> {count} && cp fetched.txt built.txt"
        inputs = ["src.txt"]
        outputs = ["built.txt"]
        depends_on = ["fetch"]
        cache = true
        "#,
    );
    fixture.write("src.txt", "x");
    fixture.run("build");
    fixture.run("build");
    assert_eq!(fixture.runs(), 4);
}

#[test]
fn no_cache_runs_for_real_and_records_nothing() {
    let fixture = Fixture::new(BUILD);
    fixture.write("src/a.txt", "alpha\n");
    fixture.write("src/b.txt", "beta\n");

    let mut args = fixture.args("build");
    args.no_cache = true;
    fixture.run_with(args);
    fixture.run("build");

    assert_eq!(
        fixture.runs(),
        2,
        "--no-cache must not have recorded an entry"
    );
}

#[test]
fn verify_passes_for_a_deterministic_task() {
    let fixture = Fixture::new(BUILD);
    fixture.write("src/a.txt", "alpha\n");
    fixture.write("src/b.txt", "beta\n");
    fixture.run("build");

    let mut args = fixture.args("build");
    args.verify = true;
    let report = fixture.run_with(args);

    assert_eq!(fixture.runs(), 2, "--verify always runs for real");
    assert!(report.contains("[verifying]"), "{report}");
}

/* The whole point of --verify: a task whose output is not a function of its
 * declared inputs is exactly the one a cache would serve stale. */
#[test]
fn verify_catches_a_task_whose_output_is_not_a_function_of_its_inputs() {
    let fixture = Fixture::new(
        r#"
        [tasks.build]
        command = "echo run >> {count} && mkdir -p out && wc -l < {count} > out/stamp"
        inputs = ["src.txt"]
        outputs = ["out"]
        cache = true
        "#,
    );
    fixture.write("src.txt", "x");
    fixture.run("build");

    let mut args = fixture.args("build");
    args.verify = true;
    let mut out = Vec::new();
    let error = run_task_to(&args, fixture.root(), &mut out).unwrap_err();
    assert!(error.to_string().contains("out/stamp"), "{error}");
}

#[test]
fn a_parallel_run_hits_the_same_entries() {
    let fixture = Fixture::new(BUILD);
    fixture.write("src/a.txt", "alpha\n");
    fixture.write("src/b.txt", "beta\n");
    fixture.run("build");
    let joined = fixture.read("out/joined.txt");
    fs::remove_dir_all(fixture.root().join("out")).unwrap();

    let mut args = fixture.args("build");
    args.parallel = true;
    fixture.run_with(args);

    assert_eq!(fixture.runs(), 1);
    assert_eq!(fixture.read("out/joined.txt"), joined);
}

#[cfg(unix)]
#[test]
fn an_executable_output_comes_back_executable() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new(
        r#"
        [tasks.build]
        command = "echo run >> {count} && mkdir -p bin && cp tool.sh bin/tool && chmod +x bin/tool"
        inputs = ["tool.sh"]
        outputs = ["bin"]
        cache = true
        "#,
    );
    fixture.write("tool.sh", "#!/bin/sh\necho hi\n");
    fixture.run("build");
    fs::remove_dir_all(fixture.root().join("bin")).unwrap();
    fixture.run("build");

    assert_eq!(fixture.runs(), 1);
    let mode = fs::metadata(fixture.root().join("bin/tool"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o111, 0o111);
}

#[test]
fn an_output_outside_the_project_is_refused_before_anything_runs() {
    let fixture = Fixture::new(
        r#"
        [tasks.build]
        command = "echo run >> {count}"
        inputs = ["a"]
        outputs = ["../elsewhere"]
        cache = true
        "#,
    );
    let mut out = Vec::new();
    let error = run_task_to(&fixture.args("build"), fixture.root(), &mut out).unwrap_err();
    assert!(error.to_string().contains("outside its project"), "{error}");
    assert_eq!(fixture.runs(), 0);
}

#[test]
fn in_a_workspace_a_library_change_reruns_the_app_built_on_it() {
    let fixture = Fixture::new("[workspace]\n");
    let count = fixture.counter().display().to_string();
    fixture.write(
        "libs/ui/culm.toml",
        &format!(
            r#"
            [tasks.build]
            command = "echo run >> {count} && mkdir -p dist && cp ui.txt dist/ui.txt"
            inputs = ["ui.txt"]
            outputs = ["dist"]
            cache = true
            "#
        ),
    );
    fixture.write(
        "apps/web/culm.toml",
        &format!(
            r#"
            [project]
            dependencies = ["libs/ui"]

            [tasks.build]
            command = "echo run >> {count} && mkdir -p dist && cat ../../libs/ui/dist/ui.txt page.txt > dist/site.txt"
            inputs = ["page.txt"]
            outputs = ["dist"]
            depends_on = ["^build"]
            cache = true
            "#
        ),
    );
    fixture.write("libs/ui/ui.txt", "button\n");
    fixture.write("apps/web/page.txt", "home\n");

    let mut args = fixture.args("build");
    args.config = None;
    fixture.run_with(args);
    assert_eq!(fixture.runs(), 2);
    assert_eq!(fixture.read("apps/web/dist/site.txt"), b"button\nhome\n");

    let mut args = fixture.args("build");
    args.config = None;
    fixture.run_with(args);
    assert_eq!(fixture.runs(), 2, "nothing changed, both projects hit");

    fixture.write("libs/ui/ui.txt", "link\n");
    let mut args = fixture.args("build");
    args.config = None;
    fixture.run_with(args);
    assert_eq!(
        fixture.runs(),
        4,
        "the library changed, so the app reran too"
    );
    assert_eq!(fixture.read("apps/web/dist/site.txt"), b"link\nhome\n");
}
