//! Subagent wrapping coverage.
//!
//! These tests guarantee that every frame a subagent (`Task` tool / `local_agent`)
//! session emits is **fully wrapped** by the typed model — no field is silently
//! dropped, and no `system` subtype falls back to an untyped `Value`.
//!
//! - The fixture test runs always (CI-safe): it replays real captured subagent
//!   sessions from `test_cases/subagent_sessions/` through [`audit_frame`].
//! - The live test (behind `integration-tests`) deploys a real subagent via the
//!   Claude CLI and audits every frame it produces in real time.

use std::fs;
use std::path::PathBuf;

use claude_codes::{audit_frame, ClaudeOutput};
use serde_json::Value;

/// Directory of checked-in subagent session captures (one JSON frame per line).
fn subagent_session_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_cases/subagent_sessions")
}

/// Parse a `.jsonl` capture into one `Value` per non-empty line.
fn read_frames(path: &PathBuf) -> Vec<Value> {
    let content =
        fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| {
            serde_json::from_str(l)
                .unwrap_or_else(|e| panic!("invalid JSON in {}: {e}", path.display()))
        })
        .collect()
}

/// Every frame in every checked-in subagent capture must be fully wrapped.
#[test]
fn captured_subagent_sessions_are_fully_wrapped() {
    let dir = subagent_session_dir();
    let captures: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read dir {}: {e}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("jsonl"))
        .collect();

    assert!(
        !captures.is_empty(),
        "no subagent captures found in {}",
        dir.display()
    );

    let mut failures: Vec<String> = Vec::new();
    let mut total_frames = 0usize;

    for capture in &captures {
        let name = capture.file_name().unwrap().to_string_lossy();
        for (idx, frame) in read_frames(capture).iter().enumerate() {
            total_frames += 1;
            let audit = audit_frame(frame);
            if !audit.fully_wrapped {
                for issue in &audit.issues {
                    failures.push(format!(
                        "{name} line {}: [{}] {issue}",
                        idx + 1,
                        audit.message_type
                    ));
                }
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {total_frames} subagent frame(s) are not fully wrapped:\n  - {}",
        failures.len(),
        failures.join("\n  - ")
    );
}

/// Sanity-check the fixtures actually exercise the subagent surface, so the
/// wrapping test above can't pass vacuously on a capture with no subagent frames.
#[test]
fn captured_sessions_contain_real_subagent_frames() {
    let dir = subagent_session_dir();
    let mut saw_task_started_agent = false;
    let mut saw_task_notification_usage = false;
    let mut saw_task_updated = false;

    for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("read dir {}: {e}", dir.display())) {
        let path = entry.unwrap().path();
        if path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
            continue;
        }
        for frame in read_frames(&path) {
            let Ok(ClaudeOutput::System(sys)) = serde_json::from_value::<ClaudeOutput>(frame)
            else {
                continue;
            };
            if let Some(started) = sys.as_task_started() {
                // A real subagent launch names its subagent type.
                if started.subagent_type.is_some() {
                    saw_task_started_agent = true;
                }
            }
            if let Some(notif) = sys.as_task_notification() {
                if notif.usage.is_some() {
                    saw_task_notification_usage = true;
                }
            }
            if sys.as_task_updated().is_some() {
                saw_task_updated = true;
            }
        }
    }

    assert!(
        saw_task_started_agent,
        "no `task_started` frame with a `subagent_type` — fixtures don't cover subagent launch"
    );
    assert!(
        saw_task_notification_usage,
        "no `task_notification` frame with `usage` — fixtures don't cover subagent token accounting"
    );
    assert!(
        saw_task_updated,
        "no `task_updated` frame — fixtures don't cover subagent lifecycle updates"
    );
}

/// Live end-to-end: deploy a real subagent through the Claude CLI and audit
/// every raw frame it produces. Gated behind `integration-tests` because it
/// spawns the real CLI and runs a subagent.
#[cfg(feature = "integration-tests")]
mod live {
    use claude_codes::{audit_frame, AsyncClient, ClaudeCliBuilder, ClaudeInput};
    use uuid::Uuid;

    /// Drive a subagent and assert no frame it emits is less than fully wrapped.
    #[tokio::test]
    async fn live_subagent_session_is_fully_wrapped() {
        let child = ClaudeCliBuilder::new()
            .model("sonnet")
            .allow_recursion()
            .dangerously_skip_permissions(true)
            .spawn()
            .await
            .expect("Failed to spawn Claude");
        let mut client = AsyncClient::new(child).expect("Failed to create client");

        let prompt = "Use the Task tool to launch a single general-purpose subagent whose \
            entire job is to compute 6 times 7 and report back just the number. After it \
            returns, tell me the answer.";
        client
            .send(&ClaudeInput::user_message(prompt, Uuid::new_v4()))
            .await
            .expect("Failed to send subagent prompt");

        let mut frames = 0usize;
        let mut failures: Vec<String> = Vec::new();
        let mut saw_task_started = false;
        let mut saw_task_notification = false;

        // Read raw frames until the turn's `result` arrives (or a safety cap).
        loop {
            let raw = match client.receive_raw().await {
                Ok(v) => v,
                Err(_) => break, // EOF / connection closed
            };
            frames += 1;

            let ty = raw.get("type").and_then(|v| v.as_str());
            if ty == Some("system") {
                match raw.get("subtype").and_then(|v| v.as_str()) {
                    Some("task_started") => saw_task_started = true,
                    Some("task_notification") => saw_task_notification = true,
                    _ => {}
                }
            }

            let audit = audit_frame(&raw);
            if !audit.fully_wrapped {
                for issue in &audit.issues {
                    failures.push(format!(
                        "[{}] {issue}\n      frame: {raw}",
                        audit.message_type
                    ));
                }
            }

            if ty == Some("result") || frames > 300 {
                break;
            }
        }
        client.shutdown().await.ok();

        assert!(frames > 0, "received no frames from the CLI");
        assert!(
            saw_task_started,
            "no `task_started` frame — the subagent never launched"
        );
        assert!(
            saw_task_notification,
            "no `task_notification` frame — the subagent never completed"
        );
        assert!(
            failures.is_empty(),
            "{} live subagent frame(s) are not fully wrapped:\n  - {}",
            failures.len(),
            failures.join("\n  - ")
        );
    }
}
