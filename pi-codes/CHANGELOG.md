# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.4] - 2026-10-05

### Added

- `PiCliBuilder::no_mcp` — the new `--no-mcp` flag (pi 1.0.4+): disable the
  built-in MCP support for the run, so no servers connect and there are no
  MCP tools.

### Changed

- Re-baseline the tested pin to pi **1.0.4** (from 1.0.3). The RPC wire
  surface is unchanged: `dist/modes/rpc/` and `json-event.d.ts` are
  byte-identical to 1.0.3, and the live suite passes unmodified. `pi --help`
  adds `--no-mcp`, and `--tools` / `--exclude-tools` now take `*` patterns
  (an allowlist keeps MCP tools unless an entry starts with `mcp__`; the
  denylist applies to MCP tools too). The `tools` / `exclude_tools` docs say
  so.

## [1.0.3] - 2026-10-05

### Changed

- Re-baseline the tested pin to pi **1.0.3** (from 1.0.2). The RPC wire
  surface is unchanged: `dist/modes/rpc/` and `json-event.d.ts` are
  byte-identical to 1.0.2, `pi --help` is byte-identical, and the live
  suite passes unmodified. Upstream 1.0.3 renames the Azure provider
  `azure-openai-responses` → `azure` (pass `--provider azure` now), adds
  Azure Foundry Chat Completions, and makes codemode `image()` save each
  image to a temp file.

## [1.0.2] - 2026-10-04

### Changed

- Re-baseline the tested pin to pi **1.0.2** (from 1.0.0; 1.0.1 was skipped).
  The RPC wire surface is unchanged: `dist/modes/rpc/` and `json-event.d.ts`
  are byte-identical to 1.0.0, `pi --help` is byte-identical, and the live
  suite passes unmodified. Upstream 1.0.1/1.0.2 touch MCP config/OAuth,
  model config and provider composition, the extension runner, and docs.

## [1.0.0] - 2026-10-02

### Changed

- Re-baseline the tested pin to pi **1.0.0** (from 0.99.2). The RPC wire
  surface is unchanged: `dist/modes/rpc/rpc-types.d.ts`, `json-event.d.ts`
  and the packaged `rpc.md`/`json.md`/`message-types.md` docs are
  byte-identical to 0.99.2, and the live suite passes unmodified. Upstream
  1.0.0 is a TUI/login/codemode release (fullscreen by default, Radius
  `/login`, `models.generateImages()`, MCP OAuth hardening).
- pi 1.0.0 rejects `--provider` without `--model` (`Error: --provider
  requires --model`, exit 1) instead of silently running the default model
  from another provider (earendil-works/pi#10236). `PiCliBuilder::provider`
  now documents that it must be paired with `PiCliBuilder::model`; the crate
  passes both flags through unchanged, so a builder that set only
  `provider` fails at spawn with pi's own error.

## [0.99.2] - 2026-10-01

### Changed

- Re-baseline the tested pin to pi **0.99.2** (from 0.99.1). `pi --help` and
  the RPC docs are unchanged (doc edits cover codemode deferred tools,
  `pi mcp add --oauth-client-name/--description`, `/reload` of
  `defaultTools`, and Anthropic workload identity federation); the live
  suite passes unmodified — pin-only release.

## [0.99.1] - 2026-09-30

Re-baseline the tested pin to pi **0.99.1** (from 0.87.1; upstream jumped
0.87.1 → 0.99.0 → 0.99.1 with no releases in between) and model the RPC
wire drift. The RPC command set (`dist/modes/rpc/rpc-types.d.ts`) and the
event name set in the packaged docs are unchanged; everything below is
additive on the wire. The unmodified 0.87.1 crate still passes its live
tier against 0.99.1.

### Added

- `InputDisposition` (`handled` / `queued` / `started`, open set) and
  `RpcResponse::disposition()`. pi 0.99.0 acknowledges a successful
  `prompt`, `steer` or `follow_up` with `data.disposition`
  (earendil-works/pi#9098, #9803): `started` when the prompt began a run,
  `queued` when pi held the input during a run, `handled` when an
  extension command or input handler consumed it — in which case no run
  started and there is no `agent_settled` to wait for. Earlier CLIs sent a
  bare success envelope, for which the accessor returns `None`.
- `PiMessage::Assistant::thinking_level` (wire `thinkingLevel`): the pi
  thinking level the agent loop requested for the response. Previously the
  key would have landed in `extra`.
- A fresh RPC tool-use corpus captured against 0.99.1
  (`test_cases/rpc_tool_use_0_99_1.jsonl`); the corpus tests run over all
  three captures and pin the prompt disposition, the typed thinking level
  and the `structuredContent` twin on built-in `bash` results.

### Changed

- **Breaking:** `PiEvent::ToolExecutionStart`, `::ToolExecutionUpdate` and
  `::ToolExecutionEnd` gain `parent_tool_call_id: Option<String>` (wire
  `parentToolCallId`). pi 0.99.0 lets a tool run other tools
  (`ctx.executeTool()`, used by the new `codemode` tool and MCP calls made
  from its scripts); those nested calls emit tool-execution events with
  the calling tool's id as parent and a `<parent id>/<n>` `toolCallId`,
  and never appear as tool calls or tool results in the transcript.
  Patterns that list every field need a `..`.

### Not modeled

- The new `pi mcp add|remove|list|login|logout` management subcommand, the
  `-e builtin:<name>` extension form and `--no-extensions` now also
  disabling built-in extensions: `PiCliBuilder` has no extension flags;
  pass them through `extra_args`.
- `nestedCalls` on a calling tool's `toolResult` message and the new
  `type` discriminator (`chat` / `image` / `classifier`) on catalog models
  stay reachable through the `extra` maps.
- `docs/session-format.md` additions (`thinkingLevel` on stored assistant
  messages, `pi.virtual-model-state` custom entries): this crate does not
  parse session files.

## [0.87.1] - 2026-09-23

### Changed

- Re-baseline the tested pin to pi **0.87.1** (from 0.87.0). No wire
  changes on the RPC surface: `pi --help` is byte-identical between the
  two releases, the RPC command set in the packaged docs is unchanged,
  and the full live tier passes unmodified. Upstream's 0.87.1 changes are
  model-catalog additions (Claude Opus 5.5, GPT-6 Sol/Luna, Grok 4.7 as
  the xAI default) and bug fixes. The packaged docs were reorganized in
  this release (`rpc.md` split into `rpc-commands.md`,
  `rpc-extension-ui.md` and `message-types.md`; `json.md` rewritten as
  the canonical event reference), so the byte-diff signal this changelog
  usually cites is noisy from here on; compare command and event name
  sets instead.

## [0.87.0] - 2026-09-22

### Changed

- Re-baseline the tested pin to pi **0.87.0** (from 0.86.1). No wire
  changes on the RPC surface: the packaged `docs/{json,rpc}.md` and the
  `pi --help` output are byte-identical between the two releases, and
  the full live tier passes unmodified. Upstream's 0.87.0 changes are
  SDK-side (`finishTurn` replaces `shouldStopAfterTurn`, canonical
  `SessionManager` context, `context_with_system` and
  `agent_before_settle` extension events, per-model image input limits)
  plus a new append-only `context_edit` session entry in
  `docs/session-format.md`. This crate does not parse session files, so
  the new entry type needs no modeling.

## [0.86.1] - 2026-09-21

### Changed

- Re-baseline the tested pin to pi **0.86.1** (from 0.86.0). No wire
  changes: the packaged `docs/{json,rpc,session-format,sdk}.md` are
  byte-identical between the two releases, and the full live tier
  passes unmodified. Upstream's only user-visible addition is a `meta`
  provider (Muse Spark models via `/login meta` or `META_API_KEY`);
  the crate passes provider names through as strings, so
  `.provider("meta")` works with no type changes.

## [0.86.0] - 2026-09-20

### Added

- `PiMessage::System` — pi 0.86.0 moved the system prompt and tool
  loadout into the transcript (earendil-works/pi#9548). Every turn now
  opens with a `role: "system"` message (`message_start` /
  `message_end` before the user message, and first in `agent_end`'s
  `messages`) carrying named prompt `sections`, `toolsAdded`
  declarations, and `toolsRemoved` references. Without this variant the
  stream failed at the first frame of every model turn with
  `unknown variant "system"`.
- A fresh RPC tool-use capture, `test_cases/rpc_tool_use_0_86_0.jsonl`,
  alongside the 0.84.4 one; the corpus tests now run over both, and a
  new test pins the leading system message's sections and tool set.

### Changed

- Re-baseline the tested pin to pi **0.86.0** (from 0.85.1). The full
  live tier passes against 0.86.0 once the variant lands: six
  credential-free RPC checks, the streamed model turn, and the
  model-tool conformance trio. The `tool_execution_end`-without-`args`
  quirk is still present and now pinned on both captures.

## [0.85.1] - 2026-09-06

### Changed

- Re-baseline the tested pin to pi **0.85.1** (from 0.84.4): the full
  live tier passes unchanged — six credential-free RPC checks, a
  streamed model turn, the model-tool conformance trio (disk-verified),
  and the 5 corpus tests over the committed 0.84.4 tool-use capture,
  which still parses fully typed. Pin-only release.

## [0.84.4] - 2026-09-02

### Changed

- Pin forward out of alpha: the crate version now names the tested pi
  release per the version-means-tested convention. Evidence: the full
  live tier passes 10/10 against pi 0.84.4 — six credential-free RPC
  checks, a streamed model turn, and the model-tool conformance trio
  (read a planted nonce, write a requested nonce, bash with a
  disk-visible side effect; write/bash verified on disk) — plus the
  committed 117-record tool-use corpus, fully typed.

## [0.0.1] - 2026-09-01

### Added

- Initial **alpha** release, tested against pi 0.84.4. The crate
  version intentionally does not yet track the tested CLI release;
  it will jump to the pi version once the API settles and the pin
  is moved forward.
  (`@earendil-works/pi-coding-agent`; requires Node 22+):
  - `PiCliBuilder` — typed argv for `--mode json` / `--mode rpc`
    invocations (provider, model, session, tools, thinking, extras).
  - `rpc` — the headless command protocol: typed `RpcCommand`s with id
    correlation, the response envelope, `AgentState` / `BashResult` /
    message views, and `RpcCommand::Raw` passthrough for unmodeled
    commands.
  - `io` — `PiEvent` (lifecycle + tool events, `Unknown` preserves
    payloads) and `PiMessage` (user / assistant / toolResult /
    bashExecution) with content blocks and usage accounting.
  - `PiRpcClient` — async (Tokio) client honoring the strict LF-only
    JSONL framing contract.
  - Live integration tier that is credential-free: drives a real
    `pi --mode rpc` process through state, model catalog, bash round
    trips (typed results landing in `get_messages`), and clean failure
    envelopes.
