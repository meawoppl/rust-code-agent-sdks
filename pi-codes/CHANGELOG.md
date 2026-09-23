# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
