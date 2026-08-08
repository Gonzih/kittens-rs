# Codex CLI Architecture Research

**Pinned commit:** `936f5eb3ee223ab34dcb221fa7c5f9943c8092bd` (openai/codex, main, 2026-08-08). All paths below relative to `codex-rs/`.

## 1. Workspace structure

**Fact:** Single Cargo workspace `codex-rs/` with ~110 member crates (`Cargo.toml` members list), plus Bazel build files at repo root. Legacy TS wrapper lives in `codex-cli/` (npm packaging only). Key crates:

- `core` — the agent engine: session, turns, tool orchestration, compaction, model client. Largest crate by far.
- `protocol` — wire types shared by all frontends: `Op` (submissions) and `EventMsg` (events) in `protocol/src/protocol.rs`, plus items, approvals, plan tool, config types. Serde-heavy, versioned with v1/v2 aliases (`task_started` alias `turn_started`).
- `tui` — ratatui frontend. `exec` — headless/CI frontend with human and JSONL event processors (`exec/src/event_processor_with_jsonl_output.rs`).
- `app-server` + `app-server-protocol` + `app-server-client` — JSON-RPC 2.0 server over stdio/UDS powering the VS Code extension (`app-server/README.md`: "Similar to MCP... bidirectional JSON-RPC").
- `apply-patch` — standalone patch-format parser/applier (own parser + `streaming_parser.rs`).
- `sandboxing`, `linux-sandbox`, `bwrap`, `windows-sandbox-rs`, `execpolicy` — sandbox stack.
- `rollout`, `rollout-trace` — session persistence/replay. `mcp-server`, `rmcp-client`, `codex-mcp` — MCP both directions.
- `tools` — tool schema/spec types (`ToolSpec`, JSON schema, freeform grammar tools).
- Many small leaf crates: `login`, `file-search`, `ollama`, `otel`, `process-hardening`, `utils/*`.

**Observation:** Decomposition axis = frontends (tui/exec/app-server) share `protocol` and talk to `core`; everything OS-ish (sandbox, pty, keyring) is quarantined in leaf crates. `protocol` is deliberately dependency-light so frontends need not link `core`.

## 2. Agent loop

**Fact:** SQ/EQ architecture. Frontends hold a `CodexThread` (`core/src/codex_thread.rs:197`) with `submit(Op) -> submission_id` (line 242) and `next_event() -> Event` (line 546) — an async channel pair. `submission_loop` (`core/src/session/handlers.rs`, spawned in `core/src/session/mod.rs:821`) consumes submissions and mutates the session.

**Fact:** Turn = repeated sampling loop; `run_turn` (`core/src/session/turn.rs:153`) doc-comment: model returns function calls → execute → feed outputs into next sampling request; assistant-message-only response ends the turn. Turn steps: drain async hook results → pre-sampling compaction (`run_pre_sampling_compact`) → resolve required MCP servers from input mentions → capture `StepContext` (model-visible world state) → inject skills/plugins → sample.

**Fact:** Cancellation via `tokio_util::sync::CancellationToken` threaded through every phase (`turn.rs:158`); `Op::Interrupt` aborts the turn (`protocol.rs:534`, emits `TurnAborted`) without killing background terminals; `Op::CleanBackgroundTerminals` does that separately.

**Fact:** Streaming: model client uses SSE (`eventsource-stream` in `core/Cargo.toml:87`) against the Responses API; deltas surface as events (`AgentMessage`, `AgentReasoning*`, `ExecCommandOutputDelta` at `protocol.rs:1399`, `TokenCount`).

**Fact:** Tool dispatch: `ToolRouter` (`core/src/tools/router.rs:68`) → `ToolRegistry` (`core/src/tools/registry.rs:267`) → per-tool handlers in `core/src/tools/handlers/` (shell, unified_exec, apply_patch, plan, view_image, request_user_input, mcp, multi_agents, sleep, tool_search). `core/src/tools/parallel.rs` exists — parallel tool call support. Tasks are typed (`core/src/tasks/{regular,compact,review,user_shell}.rs`).

## 3. TUI

**Fact:** ratatui + crossterm with `event-stream` + bracketed paste (`tui/Cargo.toml:71,82`).
**Fact (key finding):** The TUI no longer links `codex-core` — it depends on `codex-app-server-client` and drives a spawned app-server over JSON-RPC (`tui/src/app_server_session.rs`: "App-server session facade used by the TUI event loop"). So TUI, VS Code, and SDK all sit behind the same app-server protocol boundary.
**Fact:** Internal fan-in event bus: `tui/src/app_event.rs` + `app_event_sender.rs`; rendering state in `chatwidget/`, `history_cell/`, `bottom_pane/`; `insert_history.rs` writes finished cells into terminal scrollback (render-once, scrollback-owned history).

## 4. Tool surface

**Fact:** Built-ins (handlers dir): `shell` and `unified_exec` (PTY-backed persistent shell sessions via `codex-utils-pty`, process manager + head/tail output buffer in `core/src/unified_exec/`), `apply_patch`, `update_plan` (todo checklist, `protocol/src/plan_tool.rs`), `view_image`, `request_user_input`, `request_permissions`, `get_context_remaining`, `new_context_window`, `tool_search`, multi-agent spawn/communication (`multi_agents_v2`), MCP resource/search, dynamic tools, plugin install.
**Fact:** Schemas: `ToolSpec` enum in `tools/src/tool_spec.rs:22` — Function (JSON schema, hand-built in `*_spec.rs` files, no derive macros) or `Freeform` grammar tools. `apply_patch` is exposed as a freeform tool with an embedded **Lark grammar** (`core/src/tools/handlers/apply_patch_spec.rs:5,23-24`, `apply_patch.lark`) — model output constrained by grammar, not JSON. Patch format is the `*** Begin Patch` envelope (`apply-patch/src/parser.rs:37`).
**Fact:** MCP: client side (`rmcp-client`, per-server prewarm/refresh in `core/src/session/mcp*.rs`), and Codex itself exposed as an MCP server (`mcp-server`).

## 5. Context management

**Fact:** Persistence = rollout JSONL: `rollout/src/recorder.rs` ("Persist Codex session rollouts (.jsonl) so sessions can be replayed"), append-only per-session files under a sessions dir, with a SQLite index (`rollout/src/state_db.rs`, `session_index.rs`), compression, and a reverse JSONL scanner for tail reads. Resume reconstructs history from rollout (`core/src/session/rollout_reconstruction.rs`).
**Fact:** Compaction: `core/src/compact.rs` — summarization via `SUMMARIZATION_PROMPT` (from `prompts` crate); replaces history with summary; variants control whether initial context is reinjected next turn; runs pre-sampling per turn (`run_pre_sampling_compact` in `turn.rs`), auto or manual, plus remote/server-side compaction variants (`compact_remote*.rs`) and a token budget (`compact_token_budget.rs`, `session/token_budget.rs`). Emits `EventMsg::ContextCompacted`.
**Fact:** In-memory history in `core/src/context_manager/history.rs` with normalization; truncation via `codex-utils-output-truncation` (`TruncationPolicy`, `approx_token_count`). `ThreadRolledBack` event supports dropping last N user turns.

## 6. Sandboxing & approvals

**Fact:** `sandboxing` crate: macOS = spawn through `/usr/bin/sandbox-exec` (hardcoded path against PATH tampering, `seatbelt.rs:30`) with `.sbpl` policy templates in-crate; Linux = re-exec self as `codex-linux-sandbox` (arg0 trick, `arg0` crate) running bubblewrap + seccomp + `PR_SET_NO_NEW_PRIVS` (`linux-sandbox/src/bwrap.rs`); Landlock kept as legacy fallback (`linux-sandbox/src/landlock.rs:4`); Windows has its own crate.
**Fact:** `SandboxPolicy` (`protocol.rs:1004`): `DangerFullAccess` / `ReadOnly{network_access}` / `ExternalSandbox` / `WorkspaceWrite{writable_roots,...}`. `AskForApproval` (`protocol.rs:917`): `UnlessTrusted` / `OnRequest` (default, model decides) / `Granular(config)` / `Never`. Approval is an Op/Event round-trip (`Op::ExecApproval` correlated by submission id). `execpolicy` crate = rule-based command classification; `guardian` = automatic approval reviewer (emits `GuardianWarning`).

## 7. std/OS coupling (no_std relevance)

**Fact:** Zero no_std intent. `core` is saturated with tokio (mpsc/oneshot/select/spawn), `CancellationToken`, `std::fs`, `reqwest`+SSE, process spawning, PTY, SQLite. Concurrency model is task-parallel tokio, not a reactor. **Observation:** The portable seam is `protocol`: pure serde data types with almost no IO — that crate shape (Op/Event enums + item types) is directly reusable on a no_std kernel; everything else assumes an OS.

## Steal / Reject for kittens-code

**Steal:**
1. SQ/EQ split — frontend↔core as two typed queues (`Op` in, `Event` out) with submission-id correlation; approvals as ordinary Op/Event round-trips. Maps cleanly onto a no_std reactor's message channels.
2. Dependency-light `protocol` crate as the only shared contract; frontends never link the engine.
3. Turn-as-sampling-loop invariant (function call → execute → resample; message-only → done) with a single cancellation token threaded through all phases.
4. Rollout = append-only JSONL event log as source of truth; resume by replay/reconstruction. Trivially virtual-IO-able.
5. apply_patch as grammar-constrained freeform tool + standalone parser crate with streaming parser.
6. Sandbox policy as data (`SandboxPolicy`/`AskForApproval` enums in protocol), execution mechanism per-platform behind one `SandboxManager` trait-ish seam.
7. `unified_exec` head/tail output buffering for long-running process transcripts.

**Reject:**
1. Crate sprawl (~110 members, many single-file crates like `sleep` handler siblings) — organizational overhead a small crate can't afford.
2. tokio-everywhere coupling; Codex has no executor abstraction — kittens' reactor kernel should own scheduling.
3. Hand-written per-tool `*_spec.rs` JSON schemas — fine at their scale, but a declarative schema derive is cheaper.
4. TUI-behind-JSON-RPC-subprocess (app-server) — right for multi-frontend product, pure overhead for an embeddable crate; keep in-process channel option.
5. SQLite session index — filesystem/virtual-store index suffices and blocks portability.
