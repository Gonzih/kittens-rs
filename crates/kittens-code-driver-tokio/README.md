# kittens-code-driver-tokio

The std/Tokio driver for [kittens-code-core](../kittens-code-core): the host
that turns the sans-io engine into a running session. It owns the effect
world — the log appender, model clients, filesystem tools, and the run loop.

Controlling contract: [`docs/kittens-code/SPEC.md`](../../docs/kittens-code/SPEC.md)
sections 11–12 (frozen KC0).

## What it discharges

| Component | Role |
|---|---|
| `appender` | the single log-appender: framed JSONL, checksummed records, schema-epoch-validated crash repair with torn-tail truncation, an exclusive writer lock |
| `runner` | the owned-task funnel loop: whole-batch dispatch, durable-ack publication, `Runner::open` resumes a session from its log |
| `model` | the deterministic `JailClient` (scripted, offline) and, behind the `live` feature, a streaming Anthropic-dialect `LiveClient` with a tested retry/circuit-breaker policy |
| `tools` | filesystem `read`/`write`/`edit`/`grep` with path-law enforcement (absolute/traversal/symlink refused) and atomic writes |

## Features

- default: offline, no HTTP/TLS dependency in the tree.
- `live`: pulls `reqwest` (rustls) + SSE streaming for the real Anthropic
  endpoint. Optional and off by default.

## Status

Experimental `0.x` evidence release. Depends on `kittens-code-core`,
`kittens-code-protocol`, and `kittens` (with the `tokio` feature). The full
`kittens::reactor!` topology and the E1 eval rig are deferred KC0 scope; see
the SPEC.

Licensed under Apache-2.0 or MIT, at your option.
