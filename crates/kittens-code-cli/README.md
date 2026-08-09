# kittens-code-cli

`kittens-code-cli` is the KC0 headless composition root. Its binary is named
`kittens-code`. It reads one `kittens_code_protocol::op::Op` JSON object per
stdin line and writes each newly published `Event` as one flushed stdout line.

Bootstrap settings use CLI arguments first, then environment, then defaults:

| Setting | Argument | Environment | Default |
|---|---|---|---|
| Session log | `--log PATH` | `KITTENS_CODE_LOG` | `./kittens-code-session.jsonl` |
| Workspace root | `--root PATH` | `KITTENS_CODE_ROOT` | current directory |
| Backend | `--backend jail\|live` | `KITTENS_CODE_BACKEND` | `jail` |
| Jail scenario | `--scenario PATH` | `KITTENS_CODE_SCENARIO` | `./kittens-code-scenario.json` |

A jail scenario is a JSON array of `JailStep` objects. The jail is deterministic
and performs no network IO. The `live` backend requires a build with
`--features live` plus `KITTENS_CODE_API_KEY` and `KITTENS_CODE_MODEL_ID`.
`KITTENS_CODE_ENDPOINT` optionally replaces the default Anthropic endpoint.

Malformed non-empty input lines produce a non-persisted `config_invalid` error
event and the loop continues. Empty lines are ignored. `shutdown` is submitted
and drained before exit; EOF triggers a final drain.
