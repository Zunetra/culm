// === Release age guard

/* Decision 5: "latest stable" means the latest release that has been public
 * for N days, because a compromised publish is usually pulled within a few
 * days. pnpm enforces this itself through `minimumReleaseAge` in
 * `pnpm-workspace.yaml`, for every dependency, so nothing here touches a
 * pnpm install. Cargo and Go have no such setting, so for them culm asks the
 * registry and names the version itself:
 *
 * - Go: `go get module@vX`. Minimum version selection then uses exactly vX.
 * - Cargo: `cargo add name@X`, then `cargo update --precise X`, because the
 *   lockfile would otherwise still resolve to the newest compatible release,
 *   however young.
 *
 * Direct dependencies only; transitive ones resolve as each tool decides.
 * Nothing here runs under `cargo test`: the selection is pure and tested,
 * the fetching is not, the same split `maven_central` makes. */

use std::path::Path;

use semver::{Version, VersionReq};

use super::InstallCommand;
use super::crates_io;
use super::go_proxy;

/// One published version and when it was published.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    /// Seconds since the Unix epoch.
    pub published: i64,
    pub yanked: bool,
}

/// The default guard, in days, matching the three days `toolchain/node-pnpm`
/// writes for pnpm.
pub const DEFAULT_DAYS: u64 = 3;

/// The newest release that is not yanked, not a pre-release, inside `range`
/// when there is one, and at least `min_age_secs` old at `now`.
pub fn select<'a>(
    releases: &'a [Release],
    range: Option<&VersionReq>,
    min_age_secs: i64,
    now: i64,
) -> Option<&'a Release> {
    releases
        .iter()
        .filter(|release| !release.yanked && release.version.pre.is_empty())
        .filter(|release| range.is_none_or(|range| range.matches(&release.version)))
        .filter(|release| now - release.published >= min_age_secs)
        .max_by(|a, b| a.version.cmp(&b.version))
}

/// A package spec split into its name and optional range: `tokio`,
/// `typescript@<6`, `github.com/go-chi/chi/v5`.
pub fn split(spec: &str) -> (&str, Option<&str>) {
    /* A leading `@` is an npm scope, never a range. */
    match spec.get(1..).and_then(|rest| rest.find('@')) {
        Some(at) => (&spec[..=at], Some(&spec[at + 2..])),
        None => (spec, None),
    }
}

/// What the guard did to one install, for the caller to run after it.
#[derive(Debug, Default)]
pub struct Pinned {
    /// The install commands, with Cargo and Go packages given versions.
    pub commands: Vec<InstallCommand>,
    /// `(manifest path, crate, version)` for each Cargo crate whose lockfile
    /// entry must be moved back to the chosen version after installing.
    pub cargo_locks: Vec<(Option<String>, String, String)>,
    /// Packages pinned below their newest release because that release was too young.
    pub held_back: u32,
}

/// Names a version for every Cargo and Go package in `commands`. pnpm
/// commands pass through untouched.
pub fn pin(commands: &[InstallCommand], days: u64, now: i64) -> Result<Pinned, String> {
    let min_age = i64::try_from(days).unwrap_or(i64::MAX / 86_400) * 86_400;
    let mut pinned = Pinned::default();

    for command in commands {
        let mut command = command.clone();
        if matches!(command.program.as_str(), "cargo" | "go") {
            let first = command.args.len() - command.packages;
            let manifest = manifest_path(&command);
            for arg in &mut command.args[first..] {
                let (name, range) = split(arg);
                let range = range
                    .map(|raw| VersionReq::parse(raw).map_err(|e| format!("{arg}: {e}")))
                    .transpose()?;
                let chosen = match command.program.as_str() {
                    "cargo" => {
                        let releases = crates_io::releases(name)?;
                        let newest = select(&releases, range.as_ref(), i64::MIN, now);
                        select(&releases, range.as_ref(), min_age, now).map(|r| {
                            let held = newest.is_some_and(|n| n.version != r.version);
                            (r.version.clone(), held)
                        })
                    }
                    _ => go_proxy::choose(name, range.as_ref(), min_age, now)?,
                };
                if chosen.as_ref().is_some_and(|(_, held)| *held) {
                    pinned.held_back += 1;
                }
                let version = chosen
                    .map(|(version, _)| version)
                    .ok_or_else(|| format!("{name} has no release at least {days} day(s) old"))?
                    .to_string();
                if command.program == "cargo" {
                    pinned
                        .cargo_locks
                        .push((manifest.clone(), name.to_owned(), version.clone()));
                    *arg = format!("{name}@{version}");
                } else {
                    *arg = format!("{name}@v{version}");
                }
            }
        }
        pinned.commands.push(command);
    }
    Ok(pinned)
}

/// The `cargo update --precise` commands that move each pinned crate's
/// lockfile entry back to the chosen version. A crate locked at more than one
/// version is skipped, since `-p name` would be ambiguous; the caller warns.
pub fn precise_updates(
    root: &Path,
    locks: &[(Option<String>, String, String)],
) -> (Vec<InstallCommand>, Vec<String>) {
    let lockfile = std::fs::read_to_string(root.join("Cargo.lock")).unwrap_or_default();
    let locked: toml::Table = toml::from_str(&lockfile).unwrap_or_default();
    let packages = locked
        .get("package")
        .and_then(toml::Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut commands = Vec::new();
    let mut skipped = Vec::new();
    for (manifest, name, version) in locks {
        let versions: Vec<&str> = packages
            .iter()
            .filter(|package| package.get("name").and_then(toml::Value::as_str) == Some(name))
            .filter_map(|package| package.get("version").and_then(toml::Value::as_str))
            .collect();
        match versions.as_slice() {
            [locked] if locked == version => {}
            [_] => {
                let mut args = vec!["update".to_owned()];
                if let Some(manifest) = manifest {
                    args.push("--manifest-path".to_owned());
                    args.push(manifest.clone());
                }
                args.extend([
                    "-p".to_owned(),
                    name.clone(),
                    "--precise".to_owned(),
                    version.clone(),
                ]);
                commands.push(InstallCommand {
                    program: "cargo".to_owned(),
                    args,
                    packages: 0,
                });
            }
            _ => skipped.push(name.clone()),
        }
    }
    (commands, skipped)
}

fn manifest_path(command: &InstallCommand) -> Option<String> {
    command
        .args
        .iter()
        .position(|arg| arg == "--manifest-path")
        .and_then(|at| command.args.get(at + 1).cloned())
}

/// Parses an RFC 3339 timestamp (`2024-05-01T12:34:56.789+00:00` or `...Z`)
/// into seconds since the Unix epoch. Hand-rolled rather than a date crate,
/// for one format from two registries that both always send UTC-offset
/// timestamps.
pub fn unix_seconds(rfc3339: &str) -> Option<i64> {
    let (date, rest) = rfc3339.split_once('T')?;
    let mut parts = date.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: i64 = parts.next()?.parse().ok()?;
    let day: i64 = parts.next()?.parse().ok()?;

    let time_end = rest.find(['Z', '+', '-']).unwrap_or(rest.len());
    let (time, zone) = rest.split_at(time_end);
    let mut clock = time.split(':');
    let hour: i64 = clock.next()?.parse().ok()?;
    let minute: i64 = clock.next()?.parse().ok()?;
    let second: i64 = clock.next()?.split('.').next()?.parse().ok()?;

    let offset = match zone {
        "" | "Z" => 0,
        signed => {
            let sign = if signed.starts_with('-') { -1 } else { 1 };
            let (h, m) = signed[1..].split_once(':')?;
            sign * (h.parse::<i64>().ok()? * 3600 + m.parse::<i64>().ok()? * 60)
        }
    };

    Some(days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second - offset)
}

/// Days from 1970-01-01 to a proleptic Gregorian date. Howard Hinnant's
/// `days_from_civil`.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_index = (month + 9) % 12;
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: i64 = 86_400;

    fn release(version: &str, age_days: i64, yanked: bool) -> Release {
        Release {
            version: Version::parse(version).unwrap(),
            published: 100 * DAY - age_days * DAY,
            yanked,
        }
    }

    fn pick(releases: &[Release], range: Option<&str>, days: i64) -> Option<String> {
        let range = range.map(|r| VersionReq::parse(r).unwrap());
        select(releases, range.as_ref(), days * DAY, 100 * DAY).map(|r| r.version.to_string())
    }

    #[test]
    fn a_release_younger_than_the_guard_is_skipped() {
        let releases = [release("1.2.0", 10, false), release("1.3.0", 1, false)];
        assert_eq!(pick(&releases, None, 3).as_deref(), Some("1.2.0"));
        assert_eq!(pick(&releases, None, 0).as_deref(), Some("1.3.0"));
    }

    #[test]
    fn yanked_and_pre_releases_are_never_chosen() {
        let releases = [
            release("1.0.0", 30, false),
            release("1.1.0", 20, true),
            release("2.0.0-rc.1", 20, false),
        ];
        assert_eq!(pick(&releases, None, 3).as_deref(), Some("1.0.0"));
    }

    #[test]
    fn a_range_caps_the_major() {
        let releases = [release("5.9.0", 30, false), release("7.0.0", 30, false)];
        assert_eq!(pick(&releases, Some("<6"), 3).as_deref(), Some("5.9.0"));
    }

    #[test]
    fn the_newest_by_version_wins_not_by_date() {
        let releases = [release("1.10.0", 30, false), release("1.9.5", 5, false)];
        assert_eq!(pick(&releases, None, 3).as_deref(), Some("1.10.0"));
    }

    #[test]
    fn nothing_old_enough_is_none() {
        let releases = [release("1.0.0", 1, false)];
        assert_eq!(pick(&releases, None, 3), None);
    }

    #[test]
    fn a_spec_splits_into_name_and_range_but_a_scope_is_not_a_range() {
        assert_eq!(split("tokio"), ("tokio", None));
        assert_eq!(split("typescript@<6"), ("typescript", Some("<6")));
        assert_eq!(split("@vitejs/plugin-vue"), ("@vitejs/plugin-vue", None));
        assert_eq!(split("@scope/pkg@^2"), ("@scope/pkg", Some("^2")));
        assert_eq!(
            split("github.com/go-chi/chi/v5"),
            ("github.com/go-chi/chi/v5", None)
        );
    }

    #[test]
    fn timestamps_from_both_registries_parse() {
        assert_eq!(unix_seconds("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(unix_seconds("2000-03-01T00:00:00Z"), Some(951_868_800));
        assert_eq!(
            unix_seconds("2024-05-01T12:34:56.789+00:00"),
            Some(1_714_566_896)
        );
        assert_eq!(
            unix_seconds("2024-05-01T14:34:56+02:00"),
            Some(1_714_566_896)
        );
        assert_eq!(unix_seconds("not a date"), None);
    }

    #[test]
    fn a_crate_already_locked_at_the_chosen_version_needs_no_update() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.lock"),
            "[[package]]\nname = \"tokio\"\nversion = \"1.40.0\"\n\n[[package]]\nname = \"serde\"\nversion = \"1.0.300\"\n\n[[package]]\nname = \"syn\"\nversion = \"1.0.0\"\n\n[[package]]\nname = \"syn\"\nversion = \"2.0.0\"\n",
        )
        .unwrap();
        let locks = vec![
            (None, "tokio".to_owned(), "1.40.0".to_owned()),
            (
                Some("crates/a/Cargo.toml".to_owned()),
                "serde".to_owned(),
                "1.0.200".to_owned(),
            ),
            (None, "syn".to_owned(), "2.0.0".to_owned()),
        ];
        let (commands, skipped) = precise_updates(dir.path(), &locks);
        assert_eq!(commands.len(), 1);
        assert_eq!(
            commands[0].display(),
            "cargo update --manifest-path crates/a/Cargo.toml -p serde --precise 1.0.200"
        );
        assert_eq!(skipped, vec!["syn".to_owned()]);
    }
}
