# kittens-code — operator design inputs (captured 2026-08-08, verbatim intent)

These are constraints stated by the project operator before/during the research fan-out.
They are inputs to the research doc, labeled as operator constraints (not yet evidence).

1. Crate: `kittens-code`, a coding agent harness. Everything based on `kittens`
   (reactor kernel) and `kittens-tui` (separate harness is building the kittens-based
   TUI rendering abstraction — do not duplicate; treat as a dependency boundary).
2. Research targets: Grok Build, Claude Code, Codex, all open-source pieces.
   Operator's prior: Grok's TUI is best in class.
3. RLM is a core vector. Prime Intellect prime-agent = example of a purely
   RLM-based harness. https://www.primeintellect.ai/blog/prime-agent
4. Operator's prior RLM lessons (from their own past harnesses):
   - RLM query language must be SIMPLE and familiar to LLMs: closer to shell/grep
     than a programming language. Python is too much; Lua is acceptable; simpler is
     better. REDUCE REDUCE REDUCE.
   - RLM does NOT replace context compression — both together. Agent always has RLM
     query access to full context, while live window is compressed continuously.
   - Constant reminders work best: one-sentence nudge that RLM is available.
   - Two lookup speeds: quick topic lookup via embeddings; slower search when needed.
   - Mental model: CPU cache levels. Give the harness context to orient itself in
     its own context (L1 = live window, L2 = embedding index, L3 = full-transcript
     slow search).
5. Cross-harness context reads: a harness should be able to do easy lookups into
   OTHER harnesses' context — this unlocks swarm capability / agents reading each
   other's thoughts. Must be a MODULAR piece, pluggable in/out during evals.
6. Portability hard requirement: core must run on microcontrollers and WASM.
   Virtual IO + virtual filesystem, independent of Rust std.
7. Embeddings (added mid-research): RLMs rely on embeddings, but they do NOT need
   to be the most accurate — "super good enough" is the engineering principle.
   Must work on all platforms; the embedding system is pluggable per execution
   target, like the rest of the virtual-IO layer.
8. Process requirements: structure crate hierarchy well so other harnesses can
   execute slowly against it; refine research until confident SOTA + latest
   thinking is captured; then spec, refined the same way; keep lineage of thinking
   in both docs so other harnesses can hydrate, validate, and refine.
9. Redaction: prior org repos are referenced as "prior internal harness
   experiments"; the org name is never written in research artifacts.
