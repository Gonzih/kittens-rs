# Claude Code Architecture — Public-Source Research Report (compiled 2026-08-08)

## 1. Open-source status

- **Fact** — Claude Code CLI (`@anthropic-ai/claude-code`) is closed-source, shipped as an obfuscated/bundled npm package (Bun build). No official source repo; `anthropics/claude-code` on GitHub is issues/docs/examples only.
- **Fact (event)** — 2026-03-31: v2.1.88 accidentally shipped a 59.8MB sourcemap → full deobfuscated source recovered (~1,900 TS files, ~512K lines). Anthropic called it "packaging error, not a breach." Mirrors exist; 41K+ forks. Sources: theregister.com/2026/03/31/anthropic_claude_code_source_code/, thehackernews.com/2026/04/claude-code-tleaked-via-npm-packaging.html. Legal status murky — treat leak-derived writeups as *architecture intelligence*, never copy code.
- **Fact** — Claude Agent SDK IS open and usable as reference: `anthropics/claude-agent-sdk-python` and `anthropics/claude-agent-sdk-typescript` (MIT repos; usage governed by Anthropic Commercial ToS). Both wrap the closed CLI as a subprocess — the SDK is the official surface of the harness, not a reimplementation. Also open: `claude-agent-sdk-demos`, `claude-code-action`.
- **Fact** — Officially published internals: engineering blog posts + docs at code.claude.com/docs (memory/context-window/subagent pages read like specs).

## 2. Architecture patterns

- **Fact (Anthropic-described) / Observation (teardown detail)** — Single-threaded master loop (internal codename `nO`): one flat message list; loop = state → model → execute tool calls → append results → repeat until no tool calls. Deliberately no swarm-by-default; chosen for debuggability. Sources: zenml.io llmops-database claude-code entry; blog.promptlayer.com master-agent-loop (2025).
- **Fact** — Subagent model: Agent/Task tool spawns child instances with fresh context; results return as one tool-result message. Nesting to depth 3 (`CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH`); at limit the Agent tool is withheld. Background execution default as of v2.1.198; background subagents get reduced tool set. code.claude.com/docs/en/sub-agents (fetched 2026-08-08).
- **Fact** — TodoWrite tool = model-visible plan state (re-surfaced via reminders). Hooks = user shell commands at lifecycle events (PreToolUse/PostToolUse/SubagentStart/etc.); exit code 2 feeds stderr back to model; `additionalContext` JSON injects context. Skills = lazily-loaded instruction packages (one-line index in context; body loads on invoke; index NOT re-injected after compact). Slash commands = prompt templates. MCP: tool names listed, full schemas deferred behind ToolSearch (loaded on demand).
- **Fact** — Permission system: modes default/plan/acceptEdits/dontAsk/bypassPermissions/auto; allow/ask/deny rule cascade; plan mode = read-only + ExitPlanMode gate. **Observation (leak)** — 7-stage decision pipeline; "auto mode" small-model classifier (2-stage: 64-token binary verdict then 4K-token reasoning, ~1h cached); ~4,400-line recursive-descent bash AST parser for command safety (blocks eval/source/exec, Unicode-whitespace tricks, array-subscript RCE) — karanprasad.com teardown (2026, leak-derived; unverified).
- **Observation (leak)** — System prompt assembled from ~7 static cached sections + ~13 dynamic sections, deliberate cache-break boundary after MCP instructions; 600+ runtime feature flags (`tengu_*`), incl. `tengu_cached_microcompact`.

## 3. Context management

- **Fact** — Auto-compaction: near threshold, history summarized into structured summary block; startup content (system prompt, CLAUDE.md, rules, auto memory, MCP tool list) *re-injected from disk*, not summarized. Skill-index is the one thing not reloaded. Window 100K–1M configurable. code.claude.com/docs/en/context-window.
- **Observation (leak, karanprasad)** — thresholds: `effective = window − min(maxOutputTokens, 20K)`; auto-compact at `effective − 13K` (~167K on 200K); warning at `effective − 20K`. Six-tier escalation: (1) microcompact — drop tool results >60min old, (2) cache-preserving edits, (3) reactive truncation on API error, (4) pre-extracted session memory, (5) full 9-section narrative summary in forked subprocess, (6) reset.
- **Fact** — system-reminder injection: CLAUDE.md content, todo state, env/git info, hook output injected as system-reminder blocks attached to USER messages (not system prompt) — docs + teardowns (agiflow.io, weaxsey.org 2025-10-12).
- **Fact** — CLAUDE.md hierarchy (broad→specific, concatenated never overriding): managed policy → ~/.claude/CLAUDE.md → ancestor dirs → project → CLAUDE.local.md; subdirectory CLAUDE.md + path-scoped `.claude/rules/*.md` (glob frontmatter) lazy-load on file access; `@path` imports 4-hop. code.claude.com/docs/en/memory.
- **Fact** — Auto memory: model-written MEMORY.md index (first 200 lines / 25KB loaded per session) + on-demand topic files; write-time limit enforcement nudges model.
- **Fact** — API-level: **context editing** (beta context-management-2025-06-27) — server-side clearing of old tool_use/result pairs; **memory tool** (GA, Claude 4+) — client-implemented file-store tool. Anthropic recommends server-side compaction. platform.claude.com docs; claude.com/blog/context-management (2025-09).

## 4. Multi-agent

- **Fact** — Subagent isolation: fresh context; own system prompt (definition body + env, NOT full parent prompt), delegation message, CLAUDE.md hierarchy + git snapshot (Explore/Plan skip these), preloaded skills, sibling roster reminder. Does NOT see parent history/read files/auto memory. Exception: **fork** subagent inherits full parent conversation. `isolation: worktree` = isolated git worktree.
- **Fact** — Frontmatter: name, description, tools, disallowedTools, model, permissionMode, mcpServers, hooks, maxTurns, skills, initialPrompt, memory, effort, background, isolation, color.
- **Fact** — Agent teams / swarms: shipped 2026-02-06 research preview (Boris Cherny). Lead + teammates, each own context (+optionally own worktree); shared task list with dependency tracking; inbox-style SendMessage; teammates self-claim tasks. code.claude.com/docs/en/agent-teams; addyosmani.com/blog/claude-code-agent-teams/. Distinct from hub-and-spoke subagents.
- **Fact** — anthropic.com/engineering/multi-agent-research-system (2025-06-13): Opus lead + Sonnet subagents beat single-agent by 90.2% on internal eval; lessons: explicit delegation prompts, effort scaling rules, parallel tool calls.

## 5. Claude Agent SDK primitives

Docs: code.claude.com/docs/en/agent-sdk/overview (2026-08-08). SDK = "Claude Code as a library".
- `query()` streaming generator (v1) / Session API `send()`/`stream()` (v2).
- In-process custom tools via `createSdkMcpServer`/`@tool` (MCP-shaped, no subprocess) + built-ins.
- Hooks, programmatic `canUseTool` permission callback + modes.
- Subagents programmatic (`agents` option) or `.claude/agents/*.md`; sessions: resume, **fork**, persistence.
- Engineering posts: building-agents-with-the-claude-agent-sdk (2025-09-29) — loop "gather context → take action → verify work → repeat"; "Effective context engineering for AI agents" (2025-09-29); "Effective harnesses for long-running agents" (initializer agent + incremental coding agent across context windows).

## Design decisions relevant to kittens-code

1. Single flat message loop, no default swarm — parallelism only via bounded child agents; debuggability is the stated reason.
2. Two-tier compaction: cheap microcompact (age-out old tool results, cache-preserving) before expensive full summarization; trigger at `window − reserved_output − margin`. Keep startup content out of the summarizable region — re-inject from disk after compact.
3. Reminder injection channel: mutable state in system-reminder blocks attached to user messages, static system prompt → maximize prompt-cache hits. Deliberate cache boundary between static and dynamic sections.
4. Subagent isolation contract: child gets {own system prompt, delegation message, config hierarchy, env snapshot} — never parent transcript (except explicit fork). Returns exactly one summary message. Depth-capped. Background-by-default with reduced tool set.
5. Memory = plain files with size-capped index; write-time enforcement with corrective errors, not silent truncation.
6. Config as concatenation, not override; lazy path-scoped rules.
7. Permissions as data + escalation pipeline; shell safety needs real command parsing (AST), not regex.
8. Deferred tool schemas (names-only + on-demand load) to keep MCP bloat out of context.
9. Legit code references: the two MIT SDK repos + demos only; leak-derived repos are architecture intel only.
