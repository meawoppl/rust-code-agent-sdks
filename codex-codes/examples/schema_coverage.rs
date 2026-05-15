//! Codex app-server protocol coverage scorecard.
//!
//! Walks the upstream JSON Schema bundle (`codex_app_server_protocol.v2.schemas.json`)
//! and reports, for every method in `ServerNotification.oneOf` and
//! `ClientRequest.oneOf`:
//!
//!   ✓  modeled in `codex-codes` AND a hand-rolled sample validates against
//!      the schema's params definition (catches wire drift)
//!   ◐  modeled in `codex-codes` but we don't have a sample yet (so we don't
//!      know whether our struct's serde shape matches the wire — grow the
//!      sample registry below to fix)
//!   ✗  not modeled in `codex-codes` at all
//!
//! ## Run
//!
//! ```
//! cargo run --example schema_coverage
//! ```
//!
//! Uses the snapshot schema at `tests/schemas/codex_app_server_protocol.v2.schemas.json`
//! by default; override with `CODEX_SCHEMA_PATH=/path/to/freshly-generated.json`
//! (e.g. what `codex app-server generate-json-schema --out DIR` writes).
//!
//! ## Exit codes
//!
//! * `0` — no drift detected on any *validated* sample (unmodeled methods are
//!   informational, not failures)
//! * `1` — at least one modeled sample failed to validate; the wire shape has
//!   drifted from our typed struct
//!
//! ## Adding samples
//!
//! Each modeled method should have an entry in `samples::server_notification`
//! or `samples::client_request` that returns a representative serialized
//! payload constructed from real Rust types. New variants without samples
//! show up as ◐; that's a backlog item, not a failure.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

use jsonschema::Validator;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    /// Method modeled and a sample serialized JSON validates against the schema.
    Validated,
    /// Method modeled but no sample registered.
    NoSample,
    /// Method modeled, sample serialized, but did NOT match the schema.
    Drift,
    /// Method not modeled at all.
    NotModeled,
}

impl Status {
    fn glyph(self) -> &'static str {
        match self {
            Status::Validated => "✓",
            Status::NoSample => "◐",
            Status::Drift => "⚠",
            Status::NotModeled => "✗",
        }
    }

    fn descr(self) -> &'static str {
        match self {
            Status::Validated => "modeled, sample validates",
            Status::NoSample => "modeled, no sample yet",
            Status::Drift => "modeled, sample does NOT validate (DRIFT)",
            Status::NotModeled => "not modeled",
        }
    }
}

#[derive(Debug)]
struct Row {
    method: String,
    params_def: String,
    status: Status,
    errors: Vec<String>,
}

fn main() -> ExitCode {
    let path = std::env::var("CODEX_SCHEMA_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/schemas/codex_app_server_protocol.v2.schemas.json")
        });

    let bundle: Value = match std::fs::read_to_string(&path)
        .map_err(|e| format!("read: {}", e))
        .and_then(|s| serde_json::from_str(&s).map_err(|e| format!("parse: {}", e)))
    {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: could not load schema at {}: {}", path.display(), e);
            return ExitCode::from(2);
        }
    };

    let modeled_notifications = samples::modeled_notification_methods();
    let modeled_requests = samples::modeled_client_request_methods();
    let notif_samples = samples::server_notification_samples();
    let req_samples = samples::client_request_samples();

    let server_rows = walk_envelope(
        &bundle,
        "ServerNotification",
        &modeled_notifications,
        &notif_samples,
    );
    let request_rows = walk_envelope(&bundle, "ClientRequest", &modeled_requests, &req_samples);

    print_report(&path, &server_rows, &request_rows);

    let drift_any = server_rows
        .iter()
        .chain(&request_rows)
        .any(|r| r.status == Status::Drift);
    if drift_any {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn walk_envelope(
    bundle: &Value,
    envelope_name: &str,
    modeled: &std::collections::HashSet<&'static str>,
    sample_registry: &BTreeMap<&'static str, Value>,
) -> Vec<Row> {
    let envelope = match &bundle["definitions"][envelope_name] {
        Value::Object(o) => o,
        _ => {
            eprintln!("warning: schema has no definition for {}", envelope_name);
            return Vec::new();
        }
    };
    let Some(Value::Array(variants)) = envelope.get("oneOf") else {
        eprintln!(
            "warning: definitions.{}.oneOf missing or not an array",
            envelope_name
        );
        return Vec::new();
    };

    let mut rows: Vec<Row> = variants
        .iter()
        .filter_map(|v| extract_envelope_variant(v))
        .collect();

    // Sort by method for stable output.
    rows.sort_by(|a, b| a.method.cmp(&b.method));

    for row in rows.iter_mut() {
        if !modeled.contains(row.method.as_str()) {
            row.status = Status::NotModeled;
            continue;
        }
        match sample_registry.get(row.method.as_str()) {
            None => {
                row.status = Status::NoSample;
            }
            Some(sample) => match validate_against_def(bundle, &row.params_def, sample) {
                Ok(()) => row.status = Status::Validated,
                Err(errs) => {
                    row.status = Status::Drift;
                    row.errors = errs;
                }
            },
        }
    }

    rows
}

fn extract_envelope_variant(v: &Value) -> Option<Row> {
    let obj = v.as_object()?;
    let props = obj.get("properties")?.as_object()?;

    // method: { enum: ["..."] }
    let method = props
        .get("method")?
        .get("enum")?
        .as_array()?
        .first()?
        .as_str()?
        .to_string();

    // params: { $ref: "#/definitions/SomeType" }  -- some variants omit params.
    let params_def = props
        .get("params")
        .and_then(|p| p.get("$ref"))
        .and_then(|r| r.as_str())
        .and_then(|s| s.strip_prefix("#/definitions/"))
        .unwrap_or("<no params>")
        .to_string();

    Some(Row {
        method,
        params_def,
        status: Status::NotModeled,
        errors: Vec::new(),
    })
}

/// Compile a single-definition schema against the full bundle's `definitions`
/// table so `$ref` lookups resolve, then validate `sample`.
fn validate_against_def(bundle: &Value, def_name: &str, sample: &Value) -> Result<(), Vec<String>> {
    let wrapped = serde_json::json!({
        "$ref": format!("#/definitions/{}", def_name),
        "definitions": bundle["definitions"].clone(),
    });
    let validator = match Validator::new(&wrapped) {
        Ok(v) => v,
        Err(e) => return Err(vec![format!("compile error: {}", e)]),
    };
    let errs: Vec<String> = validator
        .iter_errors(sample)
        .map(|e| format!("{} ({})", e, e.instance_path()))
        .collect();
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs)
    }
}

fn print_report(path: &std::path::Path, server: &[Row], requests: &[Row]) {
    println!("Codex app-server protocol coverage — codex-codes scorecard");
    println!("============================================================");
    println!("schema: {}", path.display());
    println!();
    print_section("Server → Client Notifications", server);
    print_section("Client → Server Requests", requests);

    let total = server.len() + requests.len();
    let modeled = server
        .iter()
        .chain(requests)
        .filter(|r| {
            matches!(
                r.status,
                Status::Validated | Status::NoSample | Status::Drift
            )
        })
        .count();
    let validated = server
        .iter()
        .chain(requests)
        .filter(|r| r.status == Status::Validated)
        .count();
    let drift = server
        .iter()
        .chain(requests)
        .filter(|r| r.status == Status::Drift)
        .count();
    println!("Overall:");
    println!(
        "  modeled:        {modeled}/{total} ({:.0}%)",
        100.0 * modeled as f32 / total.max(1) as f32
    );
    println!(
        "  with sample:    {validated}/{modeled} ({:.0}%)",
        100.0 * validated as f32 / modeled.max(1) as f32
    );
    if drift > 0 {
        println!("  DRIFT:          {drift}");
    }
}

fn print_section(title: &str, rows: &[Row]) {
    let total = rows.len();
    let modeled = rows
        .iter()
        .filter(|r| {
            matches!(
                r.status,
                Status::Validated | Status::NoSample | Status::Drift
            )
        })
        .count();
    let pct = 100.0 * modeled as f32 / total.max(1) as f32;
    println!("{title} ({modeled}/{total} modeled, {pct:.0}%)");
    println!("{}", "-".repeat(title.len() + 40));

    for r in rows {
        println!(
            "  {}  {:<46} → {:<40} {}",
            r.status.glyph(),
            r.method,
            r.params_def,
            r.status.descr()
        );
        for e in &r.errors {
            println!("     {}", e);
        }
    }
    println!();
}

/// Registry of which methods we model and representative wire samples.
///
/// Adding a sample is the cheap way to grow our drift-detection coverage:
/// instantiate the typed struct with realistic field values, return it as a
/// JSON `Value`. The schema validator then checks the wire shape matches.
mod samples {
    use codex_codes::protocol::methods;
    use codex_codes::ErrorNotification;
    use serde_json::Value;
    use std::collections::{BTreeMap, HashSet};

    /// Method strings our `Notification` enum has a typed variant for. Must
    /// stay in sync with [`codex_codes::messages::Notification`].
    pub(super) fn modeled_notification_methods() -> HashSet<&'static str> {
        [
            methods::THREAD_STARTED,
            methods::THREAD_STATUS_CHANGED,
            methods::THREAD_TOKEN_USAGE_UPDATED,
            methods::TURN_STARTED,
            methods::TURN_COMPLETED,
            methods::ITEM_STARTED,
            methods::ITEM_COMPLETED,
            methods::AGENT_MESSAGE_DELTA,
            methods::CMD_OUTPUT_DELTA,
            methods::FILE_CHANGE_OUTPUT_DELTA,
            methods::REASONING_DELTA,
            methods::ERROR,
            methods::ACCOUNT_RATE_LIMITS_UPDATED,
            methods::MCP_SERVER_STARTUP_STATUS_UPDATED,
            methods::REMOTE_CONTROL_STATUS_CHANGED,
        ]
        .into_iter()
        .collect()
    }

    /// Method strings our client request surface (Params/Response pairs) covers.
    /// Must stay in sync with `client_async.rs` / `client_sync.rs`.
    pub(super) fn modeled_client_request_methods() -> HashSet<&'static str> {
        [
            methods::INITIALIZE,
            methods::INITIALIZED,
            methods::THREAD_START,
            methods::THREAD_ARCHIVE,
            methods::TURN_START,
            methods::TURN_INTERRUPT,
            methods::TURN_STEER,
        ]
        .into_iter()
        .collect()
    }

    /// Hand-rolled wire samples for server-notification params. The map's key
    /// is the JSON-RPC `method`; the value is what `serde_json::to_value` on
    /// the corresponding typed struct produces.
    ///
    /// **Seed set only** — grow this as we add typed variants. Methods absent
    /// from this map show up as ◐ ("modeled, no sample") in the report.
    pub(super) fn server_notification_samples() -> BTreeMap<&'static str, Value> {
        let mut m: BTreeMap<&'static str, Value> = BTreeMap::new();

        // Seed sample: the wire shape of `ErrorNotification` is just `{message}`
        // and is the easiest first drift check.
        m.insert(
            methods::ERROR,
            serde_json::to_value(ErrorNotification {
                error: "something blew up".into(),
                thread_id: Some("th_abc".into()),
                turn_id: Some("tn_xyz".into()),
                will_retry: false,
            })
            .expect("ErrorNotification serializes"),
        );

        // Grow this map for every notification you'd like drift-protected.
        // Pattern:
        //
        //     m.insert(
        //         methods::THREAD_STARTED,
        //         serde_json::to_value(ThreadStartedNotification { thread: ThreadInfo { ... } })
        //             .expect("ThreadStartedNotification serializes"),
        //     );

        m
    }

    /// Hand-rolled wire samples for client-request params.
    pub(super) fn client_request_samples() -> BTreeMap<&'static str, Value> {
        // Seed: none yet — grow this for the request side.
        BTreeMap::new()
    }
}
