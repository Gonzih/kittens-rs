# Open-Source Coding Agent Landscape Survey (excl. Codex / Grok Build / Claude Code)
All stars/dates verified via `gh` CLI on 2026-08-08 unless labeled Observation (web sources).

## 1. OpenCode — sst/opencode → transferred to **anomalyco/opencode**
- Fact: 195,049 stars; TypeScript (zero Go remaining); latest v1.18.15 (2026-08-07); very active.
- Fact: Monorepo ~30 packages incl. `server`, `protocol`, `sdk`, `tui`, `desktop`, `web`, `slack`, `codemode`, `containers`, `control-plane`, `acp`.
- Observation: Hard client/server split — core agent runs as local HTTP+SSE server; TUI/desktop/web/CI clients attach; headless + remote-control by design.
- Observation: Go/Bubble Tea TUI replaced ~v1.0 by **OpenTUI** (anomalyco/opentui): Zig native core with C ABI + TypeScript bindings.
- Fact: Core src has `acp/`, `mcp/`, `lsp/`, `snapshot/`, `permission/`, `plugin/`, `skill/`, `provider/` modules — 75+ LLM providers (models.dev catalog).
- Observation: `codemode` package = tool-calls-as-generated-code pattern (Cloudflare "Code Mode" style).

## 2. Goose — block/goose → transferred to **aaif-goose/goose** (Linux Foundation "Agentic AI Foundation")
- Fact: 52,558 stars; Rust; v1.45.0 (2026-07-29); active daily.
- Fact: Crates: `goose` (core), `goose-cli`, `goose-mcp`, `goose-providers` + `goose-provider-types`, `goose-sdk` + `goose-sdk-types`, `goose-acp-macros`, `goose-local-inference`, `goose-download-manager`. Clean core/types/frontends separation.
- Fact (README): desktop + CLI + embeddable API; "Built in Rust for performance and portability"; 15+ providers.
- Fact (README): Extension model MCP-native — every extension is an MCP server (70+); no bespoke plugin API.
- Fact (README): Uses **ACP as a provider mechanism** — consumes Claude/ChatGPT/Gemini subscriptions by driving other agents via ACP (ACP as both server and client role).
- Fact: `CUSTOM_DISTROS.md` — build-your-own branded distributions.
- Observation: Most relevant structural prior art: Rust workspace with types-only crates split from impl crates; std-bound (tokio).

## 3. Aider — Aider-AI/aider
- Fact: 48,055 stars; Python; last release v0.86.0 (2025-08-09); last push 2026-05-22 — **effectively dormant ~1 year**.
- Fact: Repo map = tree-sitter symbol extraction + graph ranking (PageRank over symbol references), token-budgeted, sent instead of whole files.
- Fact: Edit formats pluggable enum: `whole`, `diff` (search/replace), `diff-fenced`, `udiff`, editor-* variants for architect mode; per-model defaults benchmarked.
- Fact: Git-native — auto-commits each AI edit; undo = git revert.
- Observation: Single-process, no server/protocol layer, no MCP; enduring contributions: repo-map + edit-format benchmark discipline.

## 4. Cline / Roo Code
- Cline Fact: 65,881 stars; TypeScript; active (2026-08-08); "SDK, IDE extension, or CLI"; desktop app shipping.
- Cline Fact: Plan/Act dual-mode (separately configurable models per mode); checkpoints = shadow git repo snapshotting workspace per step, diff/restore without touching user git.
- Cline Fact: MCP marketplace + "MCP server creation by the agent itself".
- Roo Code Fact: fork of Cline; 24,352 stars; last push 2026-05-15 — **stalled ~3 months**. Added modes system (custom personas/permission profiles), orchestrator/subtask delegation (Boomerang tasks).

## 5. Gemini CLI — google-gemini/gemini-cli
- Fact: 106,423 stars; TypeScript; v0.54.4 (2026-08-07); very active.
- Fact: Packages: `cli` (React/Ink UI), `core` (agent loop, tools, auth), `sdk`, `a2a-server`, `devtools`, `vscode-ide-companion`, `test-utils`.
- Fact: **`a2a-server` = Agent2Agent (A2A) protocol endpoint** — only surveyed agent with first-class agent-to-agent exposure.
- Fact: Extension model = `gemini-extension.json` bundles MCP servers + GEMINI.md context + custom commands; installable from git; also raw MCP config and ACP (Zed/JetBrains).
- Observation: core/cli split is soft (in-process), weaker than OpenCode's.

## 6. Minimal harnesses
- **pi** — badlogic/pi-mono → **earendil-works/pi**: Fact 85,567 stars, TypeScript, v0.84.1 (2026-08-07), active. "Harness layer as product" — 4 default tools (read/write/edit/bash), tiny system prompt, everything else via TS extensions/skills/themes; deliberately NO MCP, no subagents, no plan mode, no permission popups; tree-structured sessions + compaction; 4 run modes: interactive TUI, print/JSON, **RPC mode for process embedding**, SDK. Layers: unified LLM API → agent loop (pi-agent-core AgentHarness + hooks) → TUI → coding agent.
- **Thorsten Ball "How to Build an Agent"** (ampcode.com, 2025): ~300 lines Go — loop + 3 tools is the whole trick; canonical minimal-loop reference.
- **sketch.dev** (boldsoftware/sketch): Go agent; signature move: agent runs **inside per-task Docker container** with git as sync channel to host; "outer/inner" agent split. Low public activity 2026.
- **Amp** (Sourcegraph): not open source; thread-sharing model (URL-shareable threads = team context sharing), subagents ("oracle" reviewer), no model picker. Observation only.

## 7. Other Rust-native coding agents
- **Forge** — antinomyhq: Fact 7,481 stars, Rust, v2.13.21 (2026-07-31), active. Multi-agent config (forge.yaml), provider abstraction, shell-centric TUI.
- **kwaak** — bosun-ai/kwaak: Fact 331 stars, Rust, v0.19.0 (2025-08-16), push 2026-01-27 — near-dormant. **Multiple autonomous agents in parallel in Docker**; built on `swiftide` (Rust indexing/RAG crate); ratatui UI. Best Rust prior art for swarm-in-sandbox.
- **tenx** — cortesi/tenx: 39 stars, dead 2025-06. Sharp small Rust library (libtenx) with dialect/patch abstractions + integrated validation loop.
- **pi_agent_rust** — Dicklesworthstone: 1,538 stars, push 2026-08-06; Rust port of pi, "zero unsafe". Single-author, quality unverified.
- Observation: "Claw Code" (claims 195k stars from Claude Code leak) — unverified, SEO-spam-shaped; do not cite.
- **Gap: no Rust coding agent besides kittens targets no_std or WASM (niche empty).**

## 8. Cross-cutting: protocols, swarms, constrained environments
- **ACP (Agent Client Protocol)**: agentclientprotocol/agent-client-protocol (Rust-primary, 3,909 stars, schema v1.20.0, 2026-07-21). JSON-RPC over stdio, agent⇄editor. 25+ agents; JetBrains native Dec 2025; ACP Agent Registry Jan 2026; headline of Zed 1.0 (Apr 2026). Adopters: Gemini CLI, OpenCode, Goose, Cline, Copilot CLI, Auggie. **De facto standard client boundary.**
- **A2A (Agent2Agent, Google/LF)**: Gemini CLI ships a2a-server. Amp shared threads = proprietary context sharing. Roo Boomerang + Cline subagents = in-process delegation, not protocol.
- **MCP**: universal tool-side protocol except pi (deliberate omission) and aider (dormant pre-adoption).
- **WASM/constrained**: no mainstream coding agent runs its core in WASM. Adjacent: amla-sandbox (WASM+WASI capability-enforced tool sandbox), Wasm sandboxing of MCP servers (2026 pattern), OpenTUI C-ABI native core, ruvllm-esp32 (INT8/4 inference on ESP32) — adjacent momentum, no coding-agent occupant.
- **VFS/sandbox prior art**: Cline shadow-git checkpoints; OpenCode `snapshot/` + `containers/`; sketch.dev Docker w/ git-as-sync; kwaak Docker-per-agent; OpenHands runtime abstraction (actions → pluggable local/Docker/remote runtime); WASI preopens/capability FS as cleanest virtual-FS model; aider treats git as the versioned FS.

## Shortlist: 5 highest-value design patterns for kittens-code
1. **ACP as the mandatory client boundary** — a no_std kernel speaking ACP over a byte-stream trait gets Zed/JetBrains/Neovim frontends for free and enforces the client/server split.
2. **Goose's types/impl crate split** — mirrors how a no_std core crate stays dependency-free while std frontends layer on.
3. **pi's "4 tools + hooks, nothing else in core" minimalism** — read/write/edit/exec plus extension seam is a complete kernel surface; MCP/subagents/plan mode belong outside the kernel.
4. **WASI-style capability VFS as the virtual-IO contract** (preopened dirs, deny-by-default) — simultaneously a security boundary AND portability layer to WASM/MCU.
5. **Shadow-snapshot checkpointing decoupled from user git** (Cline/OpenCode) — copy-on-write layer over the VFS trait; undo/branch semantics on hosts with no git and no OS.
