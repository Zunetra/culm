// === Telemetry

/* Decision 9 is LOCKED: opt-out, anonymous, and the exact payload is printed
 * rather than summarised. This module is the one place that decision is
 * implemented, so a change to what gets sent is a change to this file and to
 * docs/CONTEXT.md together, never to a call site.
 *
 * Best-effort by construction: a machine with no config directory, a network
 * that is down, or an endpoint that is slow all end the same way, which is
 * with the command finishing exactly as it would have without this module.
 * Nothing here ever turns an `Ok` into an `Err`. */

mod id;
mod payload;

use std::sync::mpsc;
use std::time::Duration;

pub use payload::{
    ErrorKind, Event, Exit, Install, Outcome, RunStats, Scaffold, SelectedVia,
};

/// The default collector. Overridable so a deployment (or a test) can point
/// elsewhere without a rebuild; there is no working default until the
/// receiver in `telemetry/worker/` is deployed and this constant is updated.
const DEFAULT_ENDPOINT: &str = "https://telemetry.culm.dev/events";

/// Where the first-run notice sends a reader for the full specification.
const SPEC_URL: &str = "https://github.com/Zunetra/culm/blob/main/docs/TELEMETRY.md";

/// How long a command will wait for the send to finish before moving on.
/// Long enough for a healthy network to complete a small POST, short enough
/// that a dead endpoint is not something anyone waits on.
const SEND_BUDGET: Duration = Duration::from_millis(500);

/// Whether telemetry is off, as a pure function of the three variables that
/// decide it, so the rule is testable without mutating process-wide
/// environment state, which `#![forbid(unsafe_code)]` disallows outright
/// (`std::env::set_var` is `unsafe` as of edition 2024).
///
/// `CULM_TELEMETRY` wins when set: any of `0`, `false`, `off`, `no` turns it
/// off, and `1` turns it on even in CI. Otherwise `DO_NOT_TRACK=1` turns it
/// off, and so does a set `CI`, since CI runs would swamp the numbers and make
/// them meaningless.
fn disabled(culm_telemetry: Option<&str>, do_not_track: Option<&str>, ci: Option<&str>) -> bool {
    if let Some(value) = culm_telemetry {
        let value = value.trim().to_ascii_lowercase();
        if matches!(value.as_str(), "0" | "false" | "off" | "no") {
            return true;
        }
        if value == "1" {
            return false;
        }
    }
    if do_not_track.is_some_and(|value| value.trim() == "1") {
        return true;
    }
    ci.is_some_and(|value| {
        let value = value.trim().to_ascii_lowercase();
        !value.is_empty() && value != "false" && value != "0"
    })
}

/// Records one command, unless telemetry is disabled.
/// Called only from `main.rs`, never from the command modules: those are what
/// a test invokes directly to avoid spawning a process, and they must never
/// touch the network or this machine's real telemetry id as a side effect of
/// running `cargo test`.
pub fn record(outcome: Outcome) {
    let var = |name: &str| std::env::var(name).ok();
    if disabled(
        var("CULM_TELEMETRY").as_deref(),
        var("DO_NOT_TRACK").as_deref(),
        var("CI").as_deref(),
    ) {
        return;
    }

    let Some((machine, first_run)) = id::machine_id() else {
        return;
    };

    let event = Event::new(machine, outcome);

    if first_run {
        // Printed, not summarised: decision 9's whole point is that the exact
        // bytes leaving the machine are the same bytes a user can read here.
        // Stderr, so a script reading `--no-install`'s plan from stdout never
        // has to filter this out.
        if let Ok(json) = serde_json::to_string_pretty(&event) {
            eprintln!("culm sends anonymous usage data. This is the exact payload:");
            eprintln!("{json}");
            eprintln!("Full specification: {SPEC_URL}");
            eprintln!("Opt out:            CULM_TELEMETRY=0");
        }
    }

    send(event);
}

/// Fires the request on its own thread and gives it `SEND_BUDGET` to finish.
/// A thread rather than a bare synchronous call because the alternative is
/// blocking every command on the network even when nothing above is willing
/// to wait for it; a channel rather than a bare `join` because `join` has no
/// timeout and a hung connection would otherwise hang the command with it.
fn send(event: Event) {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = post(&event);
        let _ = tx.send(());
    });
    let _ = rx.recv_timeout(SEND_BUDGET);
}

fn post(event: &Event) -> Result<(), Box<dyn std::error::Error>> {
    let endpoint =
        std::env::var("CULM_TELEMETRY_ENDPOINT").unwrap_or_else(|_| DEFAULT_ENDPOINT.to_owned());
    ureq::post(&endpoint)
        .config()
        .timeout_global(Some(SEND_BUDGET))
        .build()
        .send_json(event)?;
    Ok(())
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn any_plain_no_turns_it_off() {
        for value in ["0", "false", "OFF", "no", " False "] {
            assert!(disabled(Some(value), None, None), "{value}");
        }
        assert!(!disabled(None, None, None));
        assert!(!disabled(Some(""), None, None));
    }

    #[test]
    fn do_not_track_is_honoured() {
        assert!(disabled(None, Some("1"), None));
        assert!(!disabled(None, Some("0"), None));
    }

    /* CI would otherwise dominate the numbers, including culm's own
     * templates.yml. An explicit `CULM_TELEMETRY=1` is the one way back in. */
    #[test]
    fn ci_is_off_unless_explicitly_opted_in() {
        assert!(disabled(None, None, Some("true")));
        assert!(disabled(None, None, Some("1")));
        assert!(!disabled(None, None, Some("false")));
        assert!(!disabled(None, None, Some("")));
        assert!(!disabled(Some("1"), None, Some("true")));
    }

    #[test]
    fn an_explicit_opt_out_beats_everything() {
        assert!(disabled(Some("0"), None, Some("true")));
    }
}
