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

pub use payload::{ErrorKind, Event, Exit, Install, Outcome, RunStats, Scaffold, SelectedVia};

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

/// Whether telemetry is off, as a pure function of the variable that decides
/// it, so the rule is testable without mutating process-wide environment
/// state, which `#![forbid(unsafe_code)]` disallows outright
/// (`std::env::set_var` is `unsafe` as of edition 2024).
///
/// Opt-in, per decision 9 as amended 2026-11-02: only `CULM_TELEMETRY` set to
/// `1`, `true`, `on`, or `yes` turns it on, and an explicit opt-in holds even
/// in CI. Anything else, unset included, leaves it off, so `DO_NOT_TRACK` and
/// `CI` need no rule of their own.
fn disabled(culm_telemetry: Option<&str>) -> bool {
    let Some(value) = culm_telemetry else {
        return true;
    };
    let value = value.trim().to_ascii_lowercase();
    return !matches!(value.as_str(), "1" | "true" | "on" | "yes");
}

/// Records one command, unless telemetry is disabled.
/// Called only from `main.rs`, never from the command modules: those are what
/// a test invokes directly to avoid spawning a process, and they must never
/// touch the network or this machine's real telemetry id as a side effect of
/// running `cargo test`.
pub fn record(outcome: Outcome) {
    if disabled(std::env::var("CULM_TELEMETRY").ok().as_deref()) {
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
    fn off_when_unset() {
        assert!(disabled(None));
    }

    #[test]
    fn any_plain_yes_turns_it_on() {
        for value in ["1", "true", "ON", "yes", " True "] {
            assert!(!disabled(Some(value)), "{value}");
        }
    }

    /* Anything short of a clear yes stays off, including an empty value and a
     * typo: guessing consent is the wrong direction to err in. */
    #[test]
    fn anything_else_leaves_it_off() {
        for value in ["0", "false", "off", "no", "", "2", "yess"] {
            assert!(disabled(Some(value)), "{value}");
        }
    }
}
