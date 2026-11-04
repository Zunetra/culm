// === Error code lists stay in step

/* Four lists must agree, or a real failure is either unreportable, rejected
 * by the receiver, or unexplained: the codes every `code()` method can
 * return, the closed `ALL` list the client sends from, the receiver's own
 * `ERROR_CODES`, and the dashboard's `ERROR_CODE_MEANINGS`. Read from source
 * at test time, not compiled in, so no JavaScript is part of the published
 * crate. */

use std::collections::BTreeSet;
use std::path::Path;

use culm::telemetry::codes::ALL;

const CODE_SOURCES: &[&str] = &[
    "src/cli/error.rs",
    "src/layer/error.rs",
    "src/scaffold/error.rs",
    "src/runner/error.rs",
    "src/preset/remote.rs",
    "src/install/mod.rs",
    "src/cas/error.rs",
    "src/preset/mod.rs",
    "src/runner/remote_cache.rs",
    "src/merge/mod.rs",
];

fn read(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    return std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
}

/// Every double-quoted snake_case word in `text`.
fn quoted_words(text: &str) -> BTreeSet<String> {
    return text
        .split('"')
        .skip(1)
        .step_by(2)
        .filter(|word| {
            !word.is_empty()
                && word
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        })
        .map(str::to_owned)
        .collect();
}

/// The codes each file's `code()` method returns.
fn method_codes() -> BTreeSet<String> {
    let mut codes = BTreeSet::new();
    for file in CODE_SOURCES {
        let source = read(file);
        let start = source
            .find("pub fn code(&self)")
            .unwrap_or_else(|| panic!("{file} has no code() method"));
        let end = start
            + source[start..]
                .find("\n    }\n")
                .expect("code() has an end");
        codes.extend(quoted_words(&source[start..end]));
    }
    codes.insert("io".to_owned());
    codes.insert("other".to_owned());
    return codes;
}

fn all() -> BTreeSet<String> {
    return ALL.iter().map(|code| (*code).to_owned()).collect();
}

#[test]
fn every_code_an_error_can_return_is_in_the_sent_list() {
    assert_eq!(method_codes(), all());
}

#[test]
fn the_receiver_accepts_exactly_the_codes_culm_sends() {
    let receiver = read("telemetry/worker/src/index.js");
    let start = receiver
        .find("export const ERROR_CODES = [")
        .expect("the receiver lists its error codes");
    let end = start + receiver[start..].find("];").expect("the list closes");
    assert_eq!(quoted_words(&receiver[start..end]), all());
}

#[test]
fn the_dashboard_explains_exactly_the_codes_culm_sends() {
    let dashboard = read("telemetry/dashboard/src/codes.js");
    let start = dashboard
        .find("export const ERROR_CODE_MEANINGS = {")
        .expect("the dashboard lists its error codes");
    let end = start + dashboard[start..].find("};").expect("the list closes");
    let keys: BTreeSet<String> = dashboard[start..end]
        .lines()
        .filter_map(|line| line.trim().split_once(':').map(|(key, _)| key.to_owned()))
        .filter(|key| {
            !key.is_empty()
                && key
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        })
        .collect();
    assert_eq!(keys, all());
}
