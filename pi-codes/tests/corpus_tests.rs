//! Corpus tests: every committed capture line must lift into a typed
//! event (or a response envelope), and the captures pin measured wire
//! behaviors the docs get wrong or don't state.

use pi_codes::{PiEvent, PiMessage, RpcResponse};
use std::path::PathBuf;

fn corpus_lines(name: &str) -> Vec<serde_json::Value> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_cases")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("bad JSON line: {e}\n{l}")))
        .collect()
}

const CAPTURES: [&str; 2] = ["rpc_tool_use_0_84_4.jsonl", "rpc_tool_use_0_86_0.jsonl"];

/// Every line of both tool-use captures parses as a typed event or a response envelope; known lifecycle/tool types never fall back to Unknown.
#[test]
fn every_capture_line_is_typed() {
    let known = [
        "agent_start",
        "agent_end",
        "turn_start",
        "turn_end",
        "message_start",
        "message_update",
        "message_end",
        "tool_execution_start",
        "tool_execution_update",
        "tool_execution_end",
    ];
    for name in CAPTURES {
        for v in corpus_lines(name) {
            let t = v["type"].as_str().unwrap_or_default().to_string();
            if t == "response" {
                let _: RpcResponse = serde_json::from_value(v).expect("response envelope");
                continue;
            }
            let ev = PiEvent::from_value(v.clone()).expect("event parses");
            if known.contains(&t.as_str()) {
                assert!(
                    !matches!(ev, PiEvent::Unknown { .. }),
                    "{name}: known type {t} decoded as Unknown: {v}"
                );
            }
        }
    }
}

/// Each capture reaches agent_end and exercised all three built-in tools (write, read, bash) via real model tool calls.
#[test]
fn capture_reaches_terminal_with_all_tools() {
    for name in CAPTURES {
        let lines = corpus_lines(name);
        assert_eq!(
            lines.last().and_then(|v| v["type"].as_str()),
            Some("agent_end"),
            "{name} ends at agent_end"
        );
        let mut tools: Vec<String> = lines
            .iter()
            .filter(|v| v["type"] == "tool_execution_end")
            .filter_map(|v| v["toolName"].as_str().map(str::to_string))
            .collect();
        tools.sort();
        tools.dedup();
        assert_eq!(
            tools,
            ["bash", "read", "write"],
            "{name}: all three tools ran"
        );
    }
}

/// Measured wire quirk: RPC-mode tool_execution_end carries NO `args` field (the docs' AgentEvent type says it does) — our decoder must tolerate the absence. Still true on 0.86.0.
#[test]
fn tool_execution_end_omits_args_on_the_wire() {
    for name in CAPTURES {
        let ends: Vec<_> = corpus_lines(name)
            .into_iter()
            .filter(|v| v["type"] == "tool_execution_end")
            .collect();
        assert!(!ends.is_empty());
        for v in &ends {
            assert!(
                v.get("args").is_none(),
                "{name}: wire grew an args field on tool_execution_end — update the docs note"
            );
            let ev = PiEvent::from_value(v.clone()).unwrap();
            assert!(matches!(ev, PiEvent::ToolExecutionEnd { .. }));
        }
    }
}

/// A failed tool call surfaces as tool_execution_end with isError=true and the error text in result.content — not as a stream error.
#[test]
fn tool_errors_ride_the_result_not_the_stream() {
    let errored: Vec<_> = corpus_lines("rpc_tool_use_0_84_4.jsonl")
        .into_iter()
        .filter(|v| v["type"] == "tool_execution_end" && v["isError"] == true)
        .collect();
    assert!(
        !errored.is_empty(),
        "capture includes the parallel-call ENOENT recovery"
    );
    let ev = PiEvent::from_value(errored[0].clone()).unwrap();
    let PiEvent::ToolExecutionEnd {
        is_error, result, ..
    } = ev
    else {
        panic!("wrong variant")
    };
    assert!(is_error);
    assert!(result["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .contains("ENOENT"));
}

/// Assistant messages in each capture decode with typed content blocks; tool-call turns carry stopReason "toolUse" and the final turn "stop".
#[test]
fn assistant_messages_type_through_the_turn_ladder() {
    for name in CAPTURES {
        let mut stop_reasons = Vec::new();
        for v in corpus_lines(name) {
            if v["type"] != "message_end" {
                continue;
            }
            let ev = PiEvent::from_value(v).unwrap();
            let PiEvent::MessageEnd { message } = ev else {
                panic!("wrong variant")
            };
            if let PiMessage::Assistant { stop_reason, .. } = *message {
                stop_reasons.push(stop_reason);
            }
        }
        assert!(stop_reasons.iter().any(|s| s == "toolUse"), "{name}");
        assert_eq!(
            stop_reasons.last().map(String::as_str),
            Some("stop"),
            "{name}"
        );
    }
}

/// pi 0.86.0 opens every turn with a `system` message — streamed as message_start / message_end before the user message — carrying the full prompt `sections` and `toolsAdded` declarations; it also leads agent_end's messages. 0.84.4 never emitted one.
#[test]
fn system_message_leads_the_turn_on_0_86_0() {
    let lines = corpus_lines("rpc_tool_use_0_86_0.jsonl");
    let roles: Vec<String> = lines
        .iter()
        .filter(|v| v["type"] == "message_start")
        .filter_map(|v| v["message"]["role"].as_str().map(str::to_string))
        .collect();
    assert_eq!(&roles[..2], ["system", "user"], "system precedes user");

    let first = lines
        .iter()
        .find(|v| v["type"] == "message_end" && v["message"]["role"] == "system")
        .expect("system message_end present");
    let PiEvent::MessageEnd { message } = PiEvent::from_value(first.clone()).unwrap() else {
        panic!("wrong variant")
    };
    let PiMessage::System {
        sections,
        tools_added,
        tools_removed,
        extra,
        ..
    } = *message
    else {
        panic!("wrong role")
    };
    for section in ["preamble", "tools", "rules", "cwd", "docs"] {
        assert!(sections[section].is_string(), "section {section} present");
    }
    let mut names: Vec<&str> = tools_added
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    names.sort();
    assert_eq!(names, ["bash", "edit", "read", "write"]);
    assert!(tools_removed.is_empty());
    assert!(extra.is_empty(), "unmodeled system fields: {extra:?}");

    let end = lines.last().unwrap();
    let PiEvent::AgentEnd { messages } = PiEvent::from_value(end.clone()).unwrap() else {
        panic!("wrong variant")
    };
    assert!(matches!(messages[0], PiMessage::System { .. }));

    assert!(
        !corpus_lines("rpc_tool_use_0_84_4.jsonl")
            .iter()
            .any(|v| v["message"]["role"] == "system"),
        "0.84.4 predates system messages"
    );
}
