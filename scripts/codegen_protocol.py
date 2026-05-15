#!/usr/bin/env python3
"""
Generate Rust protocol types + samples for codex-codes from the upstream JSON Schema.

Reads both schema bundles in codex-codes/tests/schemas/:
  - codex_app_server_protocol.v2.schemas.json    (flat, has all params/notif refs)
  - codex_app_server_protocol.schemas.json        (full, has ServerRequest envelope)

Walks every notification + client-request + server-request enumerated in the
envelopes, plus every transitively-referenced definition, and emits:

  - codex-codes/src/protocol/generated.rs       (every reachable type as a Rust struct/enum)
  - codex-codes/src/protocol/generated_dispatch.rs  (method lookups → typed Params)
  - codex-codes/src/protocol/generated_samples.rs   (one validating sample per method)

The hand-written `protocol.rs` keeps the curated names callers already depend on;
the generated module is the source of truth for everything else.
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
SCHEMA_DIR = ROOT / "codex-codes" / "tests" / "schemas"
V2 = json.loads((SCHEMA_DIR / "codex_app_server_protocol.v2.schemas.json").read_text())
FULL = json.loads((SCHEMA_DIR / "codex_app_server_protocol.schemas.json").read_text())

# v2 has all the inlined definitions for notification + client-request params.
# The full bundle is the only place ServerRequest is enumerated. Merge so we
# have everything reachable.
DEFS: dict[str, Any] = dict(V2["definitions"])
for name, schema in FULL["definitions"].items():
    DEFS.setdefault(name, schema)

# ──────────────────────────────────────────────────────────────────────────
# Envelope extraction
# ──────────────────────────────────────────────────────────────────────────


def envelope_methods(bundle_defs: dict[str, Any], envelope_name: str) -> list[tuple[str, str | None]]:
    """Yield (method, params_def_name|None) for every variant in {envelope}.oneOf."""
    env = bundle_defs.get(envelope_name, {})
    out = []
    for variant in env.get("oneOf", []):
        props = variant.get("properties", {})
        method_enum = props.get("method", {}).get("enum") or []
        params_ref = props.get("params", {}).get("$ref", "")
        params_def = params_ref.rsplit("/", 1)[-1] if params_ref else None
        if method_enum:
            out.append((method_enum[0], params_def))
    return out


SERVER_NOTIFS = envelope_methods(V2["definitions"], "ServerNotification")
CLIENT_REQS = envelope_methods(V2["definitions"], "ClientRequest")
SERVER_REQS = envelope_methods(FULL["definitions"], "ServerRequest")


# ──────────────────────────────────────────────────────────────────────────
# Reachable-definition closure
# ──────────────────────────────────────────────────────────────────────────


def collect_refs(node: Any, into: set[str]) -> None:
    if isinstance(node, dict):
        ref = node.get("$ref")
        if isinstance(ref, str) and ref.startswith("#/definitions/"):
            into.add(ref.rsplit("/", 1)[-1])
        for v in node.values():
            collect_refs(v, into)
    elif isinstance(node, list):
        for v in node:
            collect_refs(v, into)


def closure(seed: set[str]) -> set[str]:
    out: set[str] = set()
    frontier = set(seed)
    while frontier:
        n = frontier.pop()
        if n in out or n not in DEFS:
            continue
        out.add(n)
        nxt: set[str] = set()
        collect_refs(DEFS[n], nxt)
        frontier |= nxt - out
    return out


# Seed: every notification params, every request params, every ServerRequest params.
SEED: set[str] = set()
for method, params_def in SERVER_NOTIFS + CLIENT_REQS + SERVER_REQS:
    if params_def:
        SEED.add(params_def)

# Also pull in the response definitions referenced by ClientRequest variants:
for variant in V2["definitions"]["ClientRequest"]["oneOf"]:
    # ClientRequest variants are objects with `method` enum + `params` $ref.
    # Their responses live alongside them; we'll discover the *Response types
    # by walking the schema for any names ending in Response.
    pass

# Add any *Response definitions known to live in the bundle.
SEED |= {name for name in DEFS if name.endswith("Response")}

REACHABLE = closure(SEED)

print(f"server notifications: {len(SERVER_NOTIFS)}")
print(f"client requests:      {len(CLIENT_REQS)}")
print(f"server requests:      {len(SERVER_REQS)}")
print(f"reachable defs:       {len(REACHABLE)} of {len(DEFS)}")


# ──────────────────────────────────────────────────────────────────────────
# Rust-name mapping & camelCase helpers
# ──────────────────────────────────────────────────────────────────────────


KEYWORDS = {
    "type", "ref", "match", "self", "where", "for", "in", "if", "else", "fn",
    "let", "mut", "const", "static", "pub", "use", "mod", "struct", "enum",
    "impl", "trait", "as", "async", "await", "break", "continue", "loop",
    "move", "return", "true", "false", "unsafe", "yield",
}


def to_snake(name: str) -> str:
    s = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1_\2", name)
    s = re.sub(r"([a-z\d])([A-Z])", r"\1_\2", s)
    s = s.replace("-", "_").replace(".", "_").replace("/", "_")
    # Strip any non-ident chars (e.g. `$schema` → `schema`).
    s = re.sub(r"[^a-zA-Z0-9_]", "", s)
    s = s.lower()
    if not s:
        s = "field"
    if s[0].isdigit():
        s = "_" + s
    if s in KEYWORDS:
        s = s + "_"
    return s


def rust_name(schema_name: str) -> str:
    """Best-effort safe Rust type name from a schema definition name."""
    # Strip namespaces / generic-style brackets.
    n = schema_name
    n = n.replace("Option<", "Opt").replace(">", "").replace("<", "_")
    n = re.sub(r"[^A-Za-z0-9_]", "_", n)
    if n and n[0].isdigit():
        n = "_" + n
    return n


# ──────────────────────────────────────────────────────────────────────────
# Rust-type expression for a schema node
# ──────────────────────────────────────────────────────────────────────────


def is_nullable_type(t: Any) -> bool:
    return isinstance(t, list) and "null" in t


def strip_null(t: list[str]) -> str:
    others = [x for x in t if x != "null"]
    return others[0] if len(others) == 1 else "string"


def schema_to_rust(node: Any) -> str:
    """Best-effort: schema node -> Rust type. Returns `Value` for things we can't model."""
    if not isinstance(node, dict):
        return "Value"

    # $ref → use the referenced Rust type.
    ref = node.get("$ref")
    if isinstance(ref, str) and ref.startswith("#/definitions/"):
        return rust_name(ref.rsplit("/", 1)[-1])

    # anyOf with null → Option<T>
    if "anyOf" in node:
        variants = node["anyOf"]
        non_null = [v for v in variants if v.get("type") != "null"]
        has_null = len(non_null) < len(variants)
        if len(non_null) == 1:
            inner = schema_to_rust(non_null[0])
            return f"Option<{inner}>" if has_null else inner
        # Two-or-more non-null branches with no tag — punt to Value.
        return "Option<Value>" if has_null else "Value"

    # oneOf with no discriminator we can extract — punt.
    if "oneOf" in node:
        return "Value"

    t = node.get("type")
    if isinstance(t, list):
        if "null" in t:
            inner = strip_null(t)
            return f"Option<{schema_to_rust({'type': inner, **{k: v for k, v in node.items() if k != 'type'}})}>"
        # Multiple non-null types — Value.
        return "Value"

    if t == "string":
        if "enum" in node:
            # Could be a unit enum elsewhere; here just emit String for inline use.
            return "String"
        return "String"
    if t == "integer":
        return "i64"
    if t == "number":
        return "f64"
    if t == "boolean":
        return "bool"
    if t == "array":
        items = node.get("items", {})
        if isinstance(items, list):
            return "Vec<Value>"
        return f"Vec<{schema_to_rust(items)}>"
    if t == "object":
        # Inline object without a $ref — has additionalProperties or properties.
        ap = node.get("additionalProperties")
        if isinstance(ap, dict):
            return f"std::collections::BTreeMap<String, {schema_to_rust(ap)}>"
        return "Value"
    if t == "null":
        return "Option<Value>"
    # Fallback.
    return "Value"


# ──────────────────────────────────────────────────────────────────────────
# Rust type emission per definition
# ──────────────────────────────────────────────────────────────────────────


def emit_struct(name: str, schema: dict[str, Any]) -> str:
    props = schema.get("properties") or {}
    required = set(schema.get("required") or [])
    rs = []
    rs.append("#[derive(Debug, Clone, Serialize, Deserialize)]")
    rs.append('#[serde(rename_all = "camelCase")]')
    rs.append(f"pub struct {rust_name(name)} {{")
    if not props:
        # Empty struct - allow extra fields via a flatten map.
        rs.append("    #[serde(flatten, default, skip_serializing_if = \"serde_json::Map::is_empty\")]")
        rs.append("    pub extra: serde_json::Map<String, Value>,")
        rs.append("}")
        return "\n".join(rs)
    for field_name in sorted(props):
        f_schema = props[field_name]
        rs_field = to_snake(field_name)
        rs_type = schema_to_rust(f_schema)
        is_opt = rs_type.startswith("Option<") or field_name not in required
        if is_opt and not rs_type.startswith("Option<"):
            rs_type = f"Option<{rs_type}>"
        rs.append("    #[serde(")
        attrs = []
        if rs_field != field_name:
            attrs.append(f'rename = "{field_name}"')
        if is_opt:
            attrs.append("default")
            attrs.append('skip_serializing_if = "Option::is_none"')
        rs[-1] = "    #[serde(" + ", ".join(attrs) + ")]"
        rs.append(f"    pub {rs_field}: {rs_type},")
    rs.append("}")
    return "\n".join(rs)


def emit_string_enum(name: str, schema: dict[str, Any]) -> str:
    """Bare-string enum: {"enum": ["a","b","c"], "type": "string"}."""
    variants = schema["enum"]
    rs = []
    rs.append("#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]")
    rs.append("pub enum " + rust_name(name) + " {")
    for v in variants:
        # Variant must be a valid Rust ident; preserve original via rename.
        v_str = str(v)
        variant_ident = re.sub(r"[^A-Za-z0-9_]", "_", v_str)
        if variant_ident and variant_ident[0].isdigit():
            variant_ident = "_" + variant_ident
        variant_ident = variant_ident[:1].upper() + variant_ident[1:] if variant_ident else "Unknown"
        rs.append(f'    #[serde(rename = "{v_str}")]')
        rs.append(f"    {variant_ident},")
    rs.append("}")
    return "\n".join(rs)


def emit_tagged_enum(name: str, schema: dict[str, Any]) -> str:
    """oneOf with each variant having a `type: {enum: ["..."]}` discriminator."""
    rs = []
    rs.append("#[derive(Debug, Clone, Serialize, Deserialize)]")
    rs.append('#[serde(tag = "type", rename_all = "camelCase")]')
    rs.append(f"pub enum {rust_name(name)} {{")
    seen_idents: set[str] = set()
    for v in schema["oneOf"]:
        v_props = v.get("properties", {})
        tag_enum = v_props.get("type", {}).get("enum")
        if not tag_enum:
            # Variant without a string-tag discriminator — fall through.
            continue
        tag = tag_enum[0]
        variant_ident = re.sub(r"[^A-Za-z0-9_]", "_", tag)
        if variant_ident and variant_ident[0].isdigit():
            variant_ident = "_" + variant_ident
        variant_ident = variant_ident[:1].upper() + variant_ident[1:]
        # Avoid duplicate variant names (rare but possible).
        base = variant_ident
        i = 2
        while variant_ident in seen_idents:
            variant_ident = f"{base}{i}"
            i += 1
        seen_idents.add(variant_ident)

        # Collect non-`type` fields as struct-like variant body.
        other_props = {k: vp for k, vp in v_props.items() if k != "type"}
        required = set(v.get("required", []))
        if not other_props:
            if tag != to_snake(variant_ident):
                rs.append(f'    #[serde(rename = "{tag}")]')
            rs.append(f"    {variant_ident},")
        else:
            if tag != to_snake(variant_ident):
                rs.append(f'    #[serde(rename = "{tag}")]')
            rs.append(f"    {variant_ident} {{")
            for fn in sorted(other_props):
                fs = other_props[fn]
                rs_field = to_snake(fn)
                rs_type = schema_to_rust(fs)
                is_opt = rs_type.startswith("Option<") or fn not in required
                if is_opt and not rs_type.startswith("Option<"):
                    rs_type = f"Option<{rs_type}>"
                attrs = []
                if rs_field != fn:
                    attrs.append(f'rename = "{fn}"')
                if is_opt:
                    attrs.append("default")
                    attrs.append('skip_serializing_if = "Option::is_none"')
                if attrs:
                    rs.append("        #[serde(" + ", ".join(attrs) + ")]")
                rs.append(f"        {rs_field}: {rs_type},")
            rs.append("    },")
    rs.append("}")
    return "\n".join(rs)


def emit_type(name: str, schema: dict[str, Any]) -> str:
    """Pick the right shape for a definition and emit Rust."""
    if "enum" in schema and schema.get("type") == "string":
        return emit_string_enum(name, schema)
    if "oneOf" in schema and all(
        isinstance(v, dict) and v.get("properties", {}).get("type", {}).get("enum")
        for v in schema["oneOf"]
    ):
        return emit_tagged_enum(name, schema)
    if schema.get("type") == "object" or "properties" in schema:
        return emit_struct(name, schema)
    if "anyOf" in schema:
        # Top-level anyOf — model as a transparent newtype over Value for now.
        return f"""#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct {rust_name(name)}(pub Value);"""
    # Fallback: opaque newtype over Value.
    return f"""#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct {rust_name(name)}(pub Value);"""


# ──────────────────────────────────────────────────────────────────────────
# Sample generation: walk a schema and produce a minimal valid JSON Value
# ──────────────────────────────────────────────────────────────────────────


def sample_for(schema: Any, depth: int = 0) -> Any:
    if not isinstance(schema, dict) or depth > 10:
        return None

    ref = schema.get("$ref")
    if isinstance(ref, str) and ref.startswith("#/definitions/"):
        target = DEFS.get(ref.rsplit("/", 1)[-1])
        if target is None:
            return {}
        return sample_for(target, depth + 1)

    if "anyOf" in schema:
        for v in schema["anyOf"]:
            if v.get("type") == "null":
                continue
            return sample_for(v, depth + 1)
        return None

    if "oneOf" in schema:
        return sample_for(schema["oneOf"][0], depth + 1)

    if "enum" in schema:
        return schema["enum"][0]

    t = schema.get("type")
    if isinstance(t, list):
        if "null" in t and len(t) == 2:
            other = [x for x in t if x != "null"][0]
            return sample_for({"type": other, **{k: v for k, v in schema.items() if k != "type"}}, depth + 1)
        return None

    if t == "string":
        return "x"
    if t == "integer":
        return 0
    if t == "number":
        return 0.0
    if t == "boolean":
        return False
    if t == "array":
        return []
    if t == "object":
        props = schema.get("properties") or {}
        required = set(schema.get("required") or [])
        out = {}
        for fn, fs in props.items():
            if fn in required:
                out[fn] = sample_for(fs, depth + 1)
        return out
    return None


# ──────────────────────────────────────────────────────────────────────────
# Emit generated.rs
# ──────────────────────────────────────────────────────────────────────────


# Names we deliberately keep hand-written in protocol.rs — don't emit duplicates.
HAND_WRITTEN: set[str] = {
    # Curated, hand-edited types kept in protocol.rs / io/items.rs:
    "ClientInfo", "InitializeCapabilities", "InitializeParams", "InitializeResponse",
    "ThreadInfo", "ThreadStartParams", "ThreadStartResponse",
    "ThreadArchiveParams", "ThreadArchiveResponse",
    "TurnStartParams", "TurnStartResponse",
    "TurnInterruptParams", "TurnInterruptResponse",
    "UserInput", "Turn", "TurnStatus", "TurnError",
    "TokenCounts", "TokenUsage", "RateLimits", "RateLimitWindow",
    "ThreadStatus", "ThreadStartedNotification", "ThreadStatusChangedNotification",
    "ThreadTokenUsageUpdatedNotification", "TurnStartedNotification",
    "TurnCompletedNotification", "ItemStartedNotification", "ItemCompletedNotification",
    "AgentMessageDeltaNotification", "CmdOutputDeltaNotification",
    "FileChangeOutputDeltaNotification", "ReasoningDeltaNotification",
    "ErrorNotification", "AccountRateLimitsUpdatedNotification",
    "McpServerStartupStatusUpdatedNotification", "RemoteControlStatusChangedNotification",
    "CommandExecutionApprovalParams", "CommandExecutionApprovalResponse",
    "FileChangeApprovalParams", "FileChangeApprovalResponse",
    "CommandApprovalDecision", "FileChangeApprovalDecision",
    # Notification types we already hand-wrote in increment 1 + 2:
    "FileChangePatchUpdatedNotification", "PlanDeltaNotification",
    "TurnPlanUpdatedNotification", "TurnPlanStep", "TurnPlanStepStatus",
    "TurnDiffUpdatedNotification", "ReasoningSummaryPartAddedNotification",
    "ReasoningTextDeltaNotification", "McpServerOauthLoginCompletedNotification",
    "AccountLoginCompletedNotification", "DeprecationNoticeNotification",
    "GuardianWarningNotification", "WarningNotification",
    "ThreadArchivedNotification", "ThreadClosedNotification",
    "ThreadUnarchivedNotification", "ThreadGoalClearedNotification",
    "ThreadNameUpdatedNotification", "SkillsChangedNotification",
    "FsChangedNotification", "ConfigWarningNotification",
    # Notification stubs from increment 5 (the 29 Value-wrappers) — we'll
    # REGENERATE these as fully-typed structs and replace the stubs.
    # So keep them OUT of HAND_WRITTEN to allow regen.
    # Item types kept in io/items.rs:
    "ThreadItem", "AgentMessageItem", "CommandExecutionItem", "CommandExecutionStatus",
    "ReasoningItem", "FileChangeItem", "FileUpdateChange", "PatchChangeKind",
    "PatchApplyStatus", "McpToolCallItem", "McpToolCallStatus", "McpToolCallResult",
    "McpToolCallError", "WebSearchItem", "TodoItem", "TodoListItem", "ErrorItem",
    "UserMessageItem", "UserMessageContent",
    # Config option types:
    "ApprovalMode", "ModelReasoningEffort", "SandboxMode", "ThreadOptions", "WebSearchMode",
    # JSON-RPC types:
    "RequestId",
    # Envelope types — don't emit; they're dispatch-only.
    "ClientNotification", "ClientRequest", "ServerNotification", "ServerRequest",
    "JSONRPCMessage", "JSONRPCRequest", "JSONRPCNotification", "JSONRPCResponse",
    "JSONRPCError", "JSONRPCErrorError",
}


def emit_generated_module() -> str:
    """Produce the contents of generated.rs."""
    out: list[str] = []
    out.append("// AUTO-GENERATED by scripts/codegen_protocol.py — DO NOT EDIT BY HAND.")
    out.append("// Run `python3 scripts/codegen_protocol.py` to regenerate.")
    out.append("//")
    out.append("// This module defines the wire types reachable from every method in the")
    out.append("// upstream codex app-server v2 schema bundle. Hand-curated types live in")
    out.append("// `protocol.rs` and `io/items.rs` and are re-exported alongside these.")
    out.append("")
    out.append("#![allow(unused_imports, non_camel_case_types, clippy::large_enum_variant, clippy::enum_variant_names, clippy::empty_docs)]")
    out.append("")
    out.append("use serde::{Deserialize, Serialize};")
    out.append("use serde_json::Value;")
    out.append("")
    out.append("// Hand-written / curated types that generated code can $ref into.")
    out.append("use crate::io::items::{")
    out.append("    AgentMessageItem, CommandExecutionItem, CommandExecutionStatus, ErrorItem,")
    out.append("    FileChangeItem, FileUpdateChange, McpToolCallError, McpToolCallItem,")
    out.append("    McpToolCallResult, McpToolCallStatus, PatchApplyStatus, PatchChangeKind,")
    out.append("    ReasoningItem, ThreadItem, TodoItem, TodoListItem, WebSearchItem,")
    out.append("    UserMessageContent, UserMessageItem,")
    out.append("};")
    out.append("use crate::io::options::{")
    out.append("    ApprovalMode, ModelReasoningEffort, SandboxMode, ThreadOptions, WebSearchMode,")
    out.append("};")
    out.append("use crate::jsonrpc::RequestId;")
    out.append("use crate::protocol::{")
    out.append("    AccountLoginCompletedNotification, AccountRateLimitsUpdatedNotification,")
    out.append("    AgentMessageDeltaNotification, ClientInfo, CmdOutputDeltaNotification,")
    out.append("    CommandApprovalDecision, CommandExecutionApprovalParams,")
    out.append("    CommandExecutionApprovalResponse, ConfigWarningNotification,")
    out.append("    DeprecationNoticeNotification, ErrorNotification, FileChangeApprovalDecision,")
    out.append("    FileChangeApprovalParams, FileChangeApprovalResponse,")
    out.append("    FileChangeOutputDeltaNotification, FileChangePatchUpdatedNotification,")
    out.append("    FsChangedNotification, GuardianWarningNotification, InitializeCapabilities,")
    out.append("    InitializeParams, InitializeResponse, ItemCompletedNotification,")
    out.append("    ItemStartedNotification, McpServerOauthLoginCompletedNotification,")
    out.append("    McpServerStartupStatusUpdatedNotification, PlanDeltaNotification, RateLimits,")
    out.append("    RateLimitWindow, ReasoningDeltaNotification,")
    out.append("    ReasoningSummaryPartAddedNotification, ReasoningTextDeltaNotification,")
    out.append("    RemoteControlStatusChangedNotification, SkillsChangedNotification,")
    out.append("    ThreadArchiveParams, ThreadArchiveResponse, ThreadArchivedNotification,")
    out.append("    ThreadClosedNotification, ThreadGoalClearedNotification, ThreadInfo,")
    out.append("    ThreadNameUpdatedNotification, ThreadStartParams, ThreadStartResponse,")
    out.append("    ThreadStartedNotification, ThreadStatus, ThreadStatusChangedNotification,")
    out.append("    ThreadTokenUsageUpdatedNotification, ThreadUnarchivedNotification,")
    out.append("    TokenCounts, TokenUsage, Turn, TurnCompletedNotification,")
    out.append("    TurnDiffUpdatedNotification, TurnError, TurnInterruptParams,")
    out.append("    TurnInterruptResponse, TurnPlanStep, TurnPlanStepStatus,")
    out.append("    TurnPlanUpdatedNotification, TurnStartParams, TurnStartResponse,")
    out.append("    TurnStartedNotification, TurnStatus, UserInput, WarningNotification,")
    out.append("};")
    out.append("")
    # Emit definitions sorted by name (deterministic).
    for name in sorted(REACHABLE):
        if name in HAND_WRITTEN:
            continue
        schema = DEFS.get(name)
        if not schema:
            continue
        try:
            rust = emit_type(name, schema)
        except Exception as e:  # noqa: BLE001
            rust = (
                f"// codegen skipped {name}: {e}\n"
                f"#[derive(Debug, Clone, Serialize, Deserialize, Default)]\n"
                f"#[serde(transparent)]\n"
                f"pub struct {rust_name(name)}(pub Value);"
            )
        out.append(rust)
        out.append("")
    return "\n".join(out)


def emit_samples_module() -> str:
    """Map each method → a JSON sample that should validate against its params schema."""
    out: list[str] = []
    out.append("// AUTO-GENERATED by scripts/codegen_protocol.py — DO NOT EDIT BY HAND.")
    out.append("")
    out.append("use serde_json::{json, Value};")
    out.append("")
    out.append("/// Notification samples keyed by JSON-RPC method.")
    out.append("pub fn server_notification_samples() -> Vec<(&'static str, Value)> {")
    out.append("    vec![")
    for method, params_def in sorted(SERVER_NOTIFS):
        if params_def and params_def in DEFS:
            s = sample_for(DEFS[params_def])
        else:
            s = {}
        out.append(f"        ({json.dumps(method)}, json!({json.dumps(s)})),")
    out.append("    ]")
    out.append("}")
    out.append("")
    out.append("/// Client-request samples keyed by JSON-RPC method.")
    out.append("pub fn client_request_samples() -> Vec<(&'static str, Value)> {")
    out.append("    vec![")
    for method, params_def in sorted(CLIENT_REQS):
        if params_def and params_def in DEFS:
            s = sample_for(DEFS[params_def])
        else:
            s = {}
        out.append(f"        ({json.dumps(method)}, json!({json.dumps(s)})),")
    out.append("    ]")
    out.append("}")
    out.append("")
    out.append("/// Server-request (approval flow) samples keyed by JSON-RPC method.")
    out.append("pub fn server_request_samples() -> Vec<(&'static str, Value)> {")
    out.append("    vec![")
    for method, params_def in sorted(SERVER_REQS):
        if params_def and params_def in DEFS:
            s = sample_for(DEFS[params_def])
        else:
            s = {}
        out.append(f"        ({json.dumps(method)}, json!({json.dumps(s)})),")
    out.append("    ]")
    out.append("}")
    return "\n".join(out)


# ──────────────────────────────────────────────────────────────────────────
# Write files
# ──────────────────────────────────────────────────────────────────────────


OUT_DIR = ROOT / "codex-codes" / "src" / "protocol_generated"
OUT_DIR.mkdir(exist_ok=True)
(OUT_DIR / "mod.rs").write_text(
    "// AUTO-GENERATED by scripts/codegen_protocol.py — DO NOT EDIT BY HAND.\n"
    "pub mod types;\n"
    "pub mod samples;\n"
)
(OUT_DIR / "types.rs").write_text(emit_generated_module())
(OUT_DIR / "samples.rs").write_text(emit_samples_module())

print(f"wrote {OUT_DIR}/mod.rs / types.rs / samples.rs")
