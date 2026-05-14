//! Background drain for the app-server's stderr pipe.
//!
//! The Codex app-server emits a heavy stream of tracing output to stderr —
//! roughly 200 KB/s during a normal session. Because the kernel pipe buffer
//! is ~64 KB, anything that leaves stderr `piped()` but unread will block
//! the child process within a fraction of a second.
//!
//! This module spawns a tiny background task (tokio for async, std::thread
//! for sync) whose only job is to read stderr line by line and forward each
//! line through the `log` crate. Codex's own log level appears as a token
//! in the line (e.g. ` INFO `, ` WARN `, ` ERROR `), so we route it to the
//! matching `log::*` macro after stripping ANSI color codes. Default
//! filtering through `RUST_LOG` then lets callers tune verbosity.

use log::{debug, error, trace, warn};

/// Module path used when forwarding stderr lines through the `log` crate.
const TARGET: &str = "codex_codes::stderr";

/// Strip ANSI CSI escape sequences (`ESC [ ... m` and similar) from a line.
///
/// Codex's tracing output is colorized, which would otherwise pollute log
/// records. This is a minimal hand-rolled scanner — no regex dependency.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            // Skip ESC [, then params (digits and ';'), then a final byte.
            i += 2;
            while i < bytes.len() {
                let c = bytes[i];
                i += 1;
                if !(c.is_ascii_digit() || c == b';') {
                    break;
                }
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

/// Forward a stderr line through the `log` crate at the level encoded in
/// the line itself. Falls back to `trace!` for lines we can't classify.
pub(crate) fn forward_line(raw: &str) {
    let line = strip_ansi(raw);
    let trimmed = line.trim_end_matches(['\n', '\r']);
    if trimmed.is_empty() {
        return;
    }

    // Codex tracing format puts the level after the timestamp:
    //   "2026-05-14T19:06:35.114314Z  INFO codex_client::custom_ca: ..."
    // We probe for the level token. Order matters: check ERROR/WARN first
    // so we don't downgrade a real warning.
    if trimmed.contains(" ERROR ") {
        error!(target: TARGET, "{}", trimmed);
    } else if trimmed.contains(" WARN ") {
        warn!(target: TARGET, "{}", trimmed);
    } else if trimmed.contains(" DEBUG ") {
        debug!(target: TARGET, "{}", trimmed);
    } else {
        // INFO and anything unrecognized — codex emits huge volumes of INFO
        // tracing, so default to trace! to keep RUST_LOG=info quiet.
        trace!(target: TARGET, "{}", trimmed);
    }
}

/// Spawn a tokio task that drains `stderr` until EOF.
#[cfg(feature = "async-client")]
pub(crate) fn spawn_async(stderr: tokio::process::ChildStderr) -> tokio::task::JoinHandle<()> {
    use tokio::io::{AsyncBufReadExt, BufReader};

    tokio::spawn(async move {
        let mut reader = BufReader::new(stderr);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break,
                Ok(_) => forward_line(&line),
                Err(_) => break,
            }
        }
    })
}

/// Spawn a std::thread that drains `stderr` until EOF.
#[cfg(feature = "sync-client")]
pub(crate) fn spawn_sync(stderr: std::process::ChildStderr) -> std::thread::JoinHandle<()> {
    use std::io::{BufRead, BufReader};

    std::thread::Builder::new()
        .name("codex-stderr-drain".to_string())
        .spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => forward_line(&line),
                    Err(_) => break,
                }
            }
        })
        .expect("failed to spawn stderr drain thread")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ansi_removes_color_codes() {
        let raw = "\x1b[32m INFO\x1b[0m hello \x1b[2mworld\x1b[0m";
        assert_eq!(strip_ansi(raw), " INFO hello world");
    }

    #[test]
    fn test_strip_ansi_passthrough() {
        assert_eq!(strip_ansi("plain text"), "plain text");
    }

    #[test]
    fn test_strip_ansi_handles_complex_params() {
        let raw = "\x1b[1;32;4mbold green underline\x1b[0m";
        assert_eq!(strip_ansi(raw), "bold green underline");
    }
}
