// === Toolchain versions

/* Decision 9 as amended 2026-11-04 (schema 3): `major.minor` only, which is
 * what deciding a support matrix needs, and less of a fingerprint than a full
 * version with its patch level, build hash, or vendor suffix. Detected only
 * after telemetry is known to be on, all tools at once, under a hard deadline:
 * a slow or hung tool reports `None` rather than holding up the command. */

use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use super::Tools;

/// Each tool and the arguments that make it print its version.
/// `go` has no `--version` flag; `go version` is its spelling.
const PROBES: [(&str, &[&str]); 5] = [
    ("rustc", &["--version"]),
    ("cargo", &["--version"]),
    ("node", &["--version"]),
    ("pnpm", &["--version"]),
    ("go", &["version"]),
];

/// Asks every tool for its version at once and keeps whatever answers within `budget`.
pub fn detect(budget: Duration) -> Tools {
    let deadline = Instant::now() + budget;
    let (tx, rx) = mpsc::channel();
    for (tool, args) in PROBES {
        let tx = tx.clone();
        std::thread::spawn(move || {
            let version = Command::new(tool)
                .args(args)
                .stdin(Stdio::null())
                .stderr(Stdio::null())
                .output()
                .ok()
                .filter(|output| output.status.success())
                .and_then(|output| major_minor(&String::from_utf8_lossy(&output.stdout)));
            let _ = tx.send((tool, version));
        });
    }
    drop(tx);

    let mut tools = Tools::default();
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let Ok((tool, version)) = rx.recv_timeout(remaining) else {
            break;
        };
        let slot = match tool {
            "rustc" => &mut tools.rustc,
            "cargo" => &mut tools.cargo,
            "node" => &mut tools.node,
            "pnpm" => &mut tools.pnpm,
            _ => &mut tools.go,
        };
        *slot = version;
    }
    return tools;
}

/// The first `major.minor` in a tool's version output, such as `1.99` from `rustc 1.99.1 (58cf8f9c3 2026-10-13)`.
/// Accepts the `v` prefix node uses and the `go` prefix `go version` uses; anything else yields `None`.
pub fn major_minor(output: &str) -> Option<String> {
    for token in output.split_whitespace() {
        let token = token
            .strip_prefix("go")
            .or_else(|| token.strip_prefix('v'))
            .unwrap_or(token);
        let mut parts = token.split('.');
        let (Some(major), Some(minor)) = (parts.next(), parts.next()) else {
            continue;
        };
        /* `minor` can carry a suffix in some builds (`1.23rc1`); keep the
         * leading digits only, and never more than four of them. */
        let minor: String = minor.chars().take_while(char::is_ascii_digit).collect();
        let plausible = |part: &str| {
            !part.is_empty() && part.len() <= 4 && part.chars().all(|c| c.is_ascii_digit())
        };
        if plausible(major) && plausible(&minor) {
            return Some(format!("{major}.{minor}"));
        }
    }
    return None;
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_tool_s_real_output_parses() {
        let cases = [
            ("rustc 1.99.1 (58cf8f9c3 2026-10-13)", "1.99"),
            ("cargo 1.99.1 (a1b2c3d4e 2026-10-01)", "1.99"),
            ("v24.22.1", "24.22"),
            ("10.4.1", "10.4"),
            ("go version go1.23.4 linux/amd64", "1.23"),
            ("go version go1.24rc1 darwin/arm64", "1.24"),
        ];
        for (output, expected) in cases {
            assert_eq!(major_minor(output).as_deref(), Some(expected), "{output}");
        }
    }

    /* Never more than major.minor: a patch level, hash, or date must not leak through. */
    #[test]
    fn nothing_beyond_major_minor_is_kept() {
        let parsed = major_minor("rustc 1.99.1-nightly (58cf8f9c3 2026-10-13)").unwrap();
        assert_eq!(parsed, "1.99");
    }

    #[test]
    fn unreadable_output_is_none() {
        for output in ["", "command not found", "version unknown", "12345.1"] {
            assert_eq!(major_minor(output), None, "{output}");
        }
    }

    #[test]
    fn a_zero_budget_returns_at_once_without_panicking() {
        let started = Instant::now();
        let _ = detect(Duration::ZERO);
        assert!(started.elapsed() < Duration::from_millis(200));
    }
}
