//! Integration tests for Codex app-server interactions.
//!
//! These tests require a real Codex CLI installation and are only run
//! when the `integration-tests` feature is enabled.
//!
//! Run with: `cargo test -p codex-codes --features integration-tests --test live_client_tests`

#![cfg(feature = "integration-tests")]

use codex_codes::{
    AsyncClient, ClientInfo, InitializeCapabilities, InitializeParams, Notification, ServerMessage,
    SyncClient, ThreadStartParams, TurnStartParams, UserInput,
};

// ── Version check ───────────────────────────────────────────────────

#[tokio::test]
async fn test_codex_cli_version() {
    codex_codes::version::check_codex_version_async()
        .await
        .expect("Failed to check Codex CLI version");
}

// ── Async client: initialize + thread_start ─────────────────────────

#[tokio::test]
async fn test_async_client_start_and_thread_start() {
    let mut client = AsyncClient::start()
        .await
        .expect("Failed to start app-server");

    let thread = client
        .thread_start(&ThreadStartParams::default())
        .await
        .expect("Failed to start thread");

    assert!(
        !thread.thread_id().is_empty(),
        "thread_id must not be empty"
    );

    client.shutdown().await.expect("Failed to shutdown");
}

// ── Async client: full turn lifecycle ───────────────────────────────

#[tokio::test]
async fn test_async_client_basic_turn() {
    let mut client = AsyncClient::start()
        .await
        .expect("Failed to start app-server");

    let thread = client
        .thread_start(&ThreadStartParams::default())
        .await
        .expect("Failed to start thread");

    client
        .turn_start(&TurnStartParams {
            thread_id: thread.thread_id().to_string(),
            input: vec![UserInput::Text {
                text: "What is 2 + 2? Reply with just the number.".to_string(),
            }],
            model: None,
            reasoning_effort: None,
            sandbox_policy: None,
        })
        .await
        .expect("Failed to start turn");

    let mut found_answer = false;
    let mut turn_completed = false;
    let mut message_count = 0;

    while let Some(msg) = client.next_message().await.expect("Failed to read message") {
        message_count += 1;

        match msg {
            ServerMessage::Notification(Notification::AgentMessageDelta(d)) => {
                if d.delta.contains('4') {
                    found_answer = true;
                }
            }
            ServerMessage::Notification(Notification::TurnCompleted(_)) => {
                turn_completed = true;
                break;
            }
            ServerMessage::Notification(_) => {}
            ServerMessage::Request { id, .. } => {
                // Auto-accept any approval requests
                client
                    .respond(id, &serde_json::json!({"decision": "accept"}))
                    .await
                    .expect("Failed to respond");
            }
        }

        if message_count > 100 {
            break;
        }
    }

    assert!(turn_completed, "Turn should have completed");
    assert!(found_answer, "Response should contain '4'");

    client.shutdown().await.expect("Failed to shutdown");
}

// ── Async client: custom initialization ─────────────────────────────

#[tokio::test]
async fn test_async_client_custom_initialize() {
    use codex_codes::AppServerBuilder;

    let mut client = AsyncClient::spawn(AppServerBuilder::new())
        .await
        .expect("Failed to spawn app-server");

    let resp = client
        .initialize(&InitializeParams {
            client_info: ClientInfo {
                name: "codex-codes-test".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: Some("Integration Test".to_string()),
            },
            capabilities: Some(InitializeCapabilities {
                experimental_api: false,
                opt_out_notification_methods: None,
            }),
        })
        .await
        .expect("Failed to initialize");

    assert!(
        !resp.user_agent.is_empty(),
        "user_agent should not be empty"
    );

    // Verify we can use the client after custom initialization
    let thread = client
        .thread_start(&ThreadStartParams::default())
        .await
        .expect("Failed to start thread after custom init");

    assert!(!thread.thread_id().is_empty());

    client.shutdown().await.expect("Failed to shutdown");
}

// ── Sync client: initialize + thread_start ──────────────────────────

#[test]
fn test_sync_client_start_and_thread_start() {
    let mut client = SyncClient::start().expect("Failed to start app-server");

    let thread = client
        .thread_start(&ThreadStartParams::default())
        .expect("Failed to start thread");

    assert!(
        !thread.thread_id().is_empty(),
        "thread_id must not be empty"
    );
}

// ── Sync client: full turn lifecycle ────────────────────────────────

#[test]
fn test_sync_client_basic_turn() {
    let mut client = SyncClient::start().expect("Failed to start app-server");

    let thread = client
        .thread_start(&ThreadStartParams::default())
        .expect("Failed to start thread");

    client
        .turn_start(&TurnStartParams {
            thread_id: thread.thread_id().to_string(),
            input: vec![UserInput::Text {
                text: "What is 2 + 2? Reply with just the number.".to_string(),
            }],
            model: None,
            reasoning_effort: None,
            sandbox_policy: None,
        })
        .expect("Failed to start turn");

    let mut found_answer = false;
    let mut turn_completed = false;
    let mut message_count = 0;

    for result in client.events() {
        let msg = result.expect("Failed to read message");
        message_count += 1;

        match msg {
            ServerMessage::Notification(Notification::AgentMessageDelta(d)) => {
                if d.delta.contains('4') {
                    found_answer = true;
                }
            }
            ServerMessage::Notification(Notification::TurnCompleted(_)) => {
                turn_completed = true;
                break;
            }
            ServerMessage::Notification(_) | ServerMessage::Request { .. } => {}
        }

        if message_count > 100 {
            break;
        }
    }

    assert!(turn_completed, "Turn should have completed");
    assert!(found_answer, "Response should contain '4'");
}

// ── Async client: multi-turn conversation ───────────────────────────

#[tokio::test]
async fn test_async_client_multi_turn() {
    let mut client = AsyncClient::start()
        .await
        .expect("Failed to start app-server");

    let thread = client
        .thread_start(&ThreadStartParams::default())
        .await
        .expect("Failed to start thread");

    // First turn: establish context
    client
        .turn_start(&TurnStartParams {
            thread_id: thread.thread_id().to_string(),
            input: vec![UserInput::Text {
                text: "Remember the number 42. Just say OK.".to_string(),
            }],
            model: None,
            reasoning_effort: None,
            sandbox_policy: None,
        })
        .await
        .expect("Failed to start first turn");

    // Drain until turn completes
    let mut message_count = 0;
    while let Some(msg) = client.next_message().await.expect("read") {
        message_count += 1;
        match msg {
            ServerMessage::Notification(Notification::TurnCompleted(_)) => break,
            ServerMessage::Notification(_) => {}
            ServerMessage::Request { id, .. } => {
                client
                    .respond(id, &serde_json::json!({"decision": "accept"}))
                    .await
                    .ok();
            }
        }
        if message_count > 100 {
            break;
        }
    }

    // Second turn: check context is maintained
    client
        .turn_start(&TurnStartParams {
            thread_id: thread.thread_id().to_string(),
            input: vec![UserInput::Text {
                text: "What number did I ask you to remember? Reply with just the number."
                    .to_string(),
            }],
            model: None,
            reasoning_effort: None,
            sandbox_policy: None,
        })
        .await
        .expect("Failed to start second turn");

    let mut found_42 = false;
    let mut message_count = 0;
    while let Some(msg) = client.next_message().await.expect("read") {
        message_count += 1;
        match msg {
            ServerMessage::Notification(Notification::AgentMessageDelta(d)) => {
                if d.delta.contains("42") {
                    found_42 = true;
                }
            }
            ServerMessage::Notification(Notification::TurnCompleted(_)) => break,
            ServerMessage::Notification(_) => {}
            ServerMessage::Request { id, .. } => {
                client
                    .respond(id, &serde_json::json!({"decision": "accept"}))
                    .await
                    .ok();
            }
        }
        if message_count > 100 {
            break;
        }
    }

    assert!(found_42, "Agent should remember 42 from the first turn");

    client.shutdown().await.expect("Failed to shutdown");
}

// ── Async client: event stream API ──────────────────────────────────

#[tokio::test]
async fn test_async_client_event_stream() {
    let mut client = AsyncClient::start()
        .await
        .expect("Failed to start app-server");

    let thread = client
        .thread_start(&ThreadStartParams::default())
        .await
        .expect("Failed to start thread");

    client
        .turn_start(&TurnStartParams {
            thread_id: thread.thread_id().to_string(),
            input: vec![UserInput::Text {
                text: "Say hello.".to_string(),
            }],
            model: None,
            reasoning_effort: None,
            sandbox_policy: None,
        })
        .await
        .expect("Failed to start turn");

    let mut stream = client.events();
    let mut got_turn_started = false;
    let mut got_turn_completed = false;
    let mut message_count = 0;

    while let Some(result) = stream.next().await {
        let msg = result.expect("Failed to read event");
        message_count += 1;

        if let ServerMessage::Notification(n) = &msg {
            match n {
                Notification::TurnStarted(_) => got_turn_started = true,
                Notification::TurnCompleted(_) => {
                    got_turn_completed = true;
                    break;
                }
                _ => {}
            }
        }

        if message_count > 100 {
            break;
        }
    }

    assert!(got_turn_started, "Should have received turn/started");
    assert!(got_turn_completed, "Should have received turn/completed");
}

// ── Strict typed-message audit ──────────────────────────────────────
//
// Runs a turn that exercises a command-execution item plus the usual thread
// and turn lifecycle, then asserts that **every** notification and server
// request received deserializes into its typed variant — i.e. `Unknown` must
// not appear. The dispatch in [`codex_codes::messages`] is strict on known
// methods (deserialization failure surfaces as a client error) and tolerant
// on unknown methods (routes to `Unknown` without error); this test catches
// both regressions: a known method whose payload no longer fits the typed
// struct fails the client call before we get here, and a brand-new method
// that we haven't modeled fails the assertion below.
#[tokio::test]
async fn test_typed_message_audit_strict() {
    use std::collections::BTreeMap;

    let mut client = AsyncClient::start()
        .await
        .expect("Failed to start app-server");

    let thread = client
        .thread_start(&ThreadStartParams::default())
        .await
        .expect("Failed to start thread");

    client
        .turn_start(&TurnStartParams {
            thread_id: thread.thread_id().to_string(),
            input: vec![UserInput::Text {
                text: "Run `ls` in the current directory, then briefly describe what you saw."
                    .to_string(),
            }],
            model: None,
            reasoning_effort: None,
            sandbox_policy: None,
        })
        .await
        .expect("Failed to start turn");

    let mut typed_counts: BTreeMap<&'static str, u32> = BTreeMap::new();
    let mut unknown_methods: BTreeMap<String, u32> = BTreeMap::new();
    let mut message_count = 0;

    while let Some(msg) = client.next_message().await.expect("read") {
        message_count += 1;
        match msg {
            ServerMessage::Notification(n) => {
                let variant = match &n {
                    Notification::ThreadStarted(_) => "ThreadStarted",
                    Notification::ThreadStatusChanged(_) => "ThreadStatusChanged",
                    Notification::ThreadTokenUsageUpdated(_) => "ThreadTokenUsageUpdated",
                    Notification::TurnStarted(_) => "TurnStarted",
                    Notification::TurnCompleted(_) => "TurnCompleted",
                    Notification::ItemStarted(_) => "ItemStarted",
                    Notification::ItemCompleted(_) => "ItemCompleted",
                    Notification::AgentMessageDelta(_) => "AgentMessageDelta",
                    Notification::CmdOutputDelta(_) => "CmdOutputDelta",
                    Notification::FileChangeOutputDelta(_) => "FileChangeOutputDelta",
                    Notification::ReasoningDelta(_) => "ReasoningDelta",
                    Notification::Error(_) => "Error",
                    Notification::AccountRateLimitsUpdated(_) => "AccountRateLimitsUpdated",
                    Notification::McpServerStartupStatusUpdated(_) => {
                        "McpServerStartupStatusUpdated"
                    }
                    Notification::RemoteControlStatusChanged(_) => "RemoteControlStatusChanged",
                    Notification::Unknown { method, .. } => {
                        *unknown_methods.entry(method.clone()).or_insert(0) += 1;
                        continue;
                    }
                };
                *typed_counts.entry(variant).or_insert(0) += 1;
                if matches!(&n, Notification::TurnCompleted(_)) {
                    break;
                }
            }
            ServerMessage::Request { id, request } => {
                if request.is_unknown() {
                    unknown_methods
                        .entry(request.method().to_string())
                        .and_modify(|c| *c += 1)
                        .or_insert(1);
                }
                client
                    .respond(id, &serde_json::json!({"decision": "accept"}))
                    .await
                    .expect("respond");
            }
        }

        if message_count > 500 {
            break;
        }
    }

    eprintln!("\n── Typed message audit ──────────────────────────────────");
    eprintln!("Typed variants seen ({} kinds):", typed_counts.len());
    for (variant, n) in &typed_counts {
        eprintln!("  {:4}× Notification::{}", n, variant);
    }
    eprintln!("─────────────────────────────────────────────────────────\n");

    assert!(
        unknown_methods.is_empty(),
        "Wire methods with no typed binding (audit must be empty): {:?}",
        unknown_methods
    );

    client.shutdown().await.expect("shutdown");
}
