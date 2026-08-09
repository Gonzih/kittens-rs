# Spec-buildability probe output (2026-08-08)
#
# Produced by the blind implementation slice: Codex gpt-5.6-sol (ultra)
# implemented the record model + crash-repair scanner and the RLM IR +
# lowering from SPEC v0.8 and the published protocol crate ONLY (branch
# kc0-codex-slice; 23 tests, thumbv7em-clean; integrated into
# kittens-code-core). 43 entries: 12 blocker / 30 major / 1 minor.
# Every entry is a place the spec forced an implementer choice — the D2/D4
# freeze pass must disposition each one.

# KC0 slice buildability questions

This ledger records every place where kittens-code SPEC v0.8 did not provide
enough information to build this deliberately isolated slice without making a
choice. It is not a claim that the specification is frozen or that the choices
below are suitable for the eventual production driver.

## 1. [Blocker] Authority to implement an unfrozen specification

- **Needed:** A controlling contract that authorizes implementation.
- **Spec:** The v0.8 header says “Not yet frozen, no implementation authorized,” and D2/D4 remain open.
- **Choice:** Treat the explicit KC0-slice request as authority for an experimental buildability probe only; do not treat this crate as evidence that v0.8 is frozen.

## 2. [Blocker] Codec-independent `Record.payload` representation

- **Needed:** A concrete Rust type for the payload field in S3.
- **Spec:** S3 names `payload`; S2 names semantic classes but gives no payload sum type.
- **Choice:** Define a typed `RecordPayload`: typed header, accepted submission, emitted event, and config patch variants, with opaque byte vectors for effect outcomes and ordinary stream stages.

## 3. [Major] Overlapping record-kind classes

- **Needed:** One canonical kind for data that is simultaneously an accepted Op, Event, effect outcome, or stream stage.
- **Spec:** Config patches and approvals are accepted Ops, while tool terminals can be Events, effect outcomes, and stream terminals.
- **Choice:** Give every requested class its own `RecordKind`; the producer must choose exactly one canonical class for each append. This slice validates kind/payload agreement but cannot derive that producer policy from v0.8.

## 4. [Major] Accepted-op identity payload

- **Needed:** Whether an accepted-op record stores `Op` alone or its submission id too.
- **Spec:** P1 says every Op has a submission id; S2 only says “accepted Ops.”
- **Choice:** Store the protocol `Submission`, preserving both the id and the Op.

## 5. [Major] Effect-outcome and stream payload shapes

- **Needed:** Concrete data for generic effect outcomes and Started/Progress/Terminal stages.
- **Spec:** Neither v0.8 nor `kittens-code-protocol` publishes generic payload types for these records.
- **Choice:** Store their codec-independent application payloads as opaque `Vec<u8>` values; lifecycle validation uses `kind`, `txn`, and `epoch` only.

## 6. [Blocker] Crash-repair terminal shape

- **Needed:** A machine-readable terminal representing `aborted_by_crash`.
- **Spec:** S3 names the outcome, but the protocol only has the less-specific `ToolOutcome::Aborted`.
- **Choice:** Add a local `RepairTerminalCause::AbortedByCrash` value inside a distinct repair-terminal payload and record kind.

## 7. [Major] Unknown persisted record kinds

- **Needed:** A representation and scan rule for future record kinds.
- **Spec:** P2 says unknown state-bearing kinds require a schema-epoch bump, but does not define opaque `RecordKind` preservation.
- **Choice:** Use a closed enum. The slice assumes a future state-bearing kind arrives only under a higher header epoch, which is refused before body interpretation; same-epoch unknown-kind round trips remain outside this model.

## 8. [Major] Unknown Event preservation versus published serde types

- **Needed:** Lossless storage of future emitted Event variants.
- **Spec:** P2 requires clients to retain unknown Events opaquely, but the published tagged `Event` enum has no unknown/raw variant; `#[non_exhaustive]` does not make serde decoding unknown-tolerant.
- **Choice:** Use the published typed `Event` for this slice. A production codec must additionally retain raw event bytes to satisfy P2/G2c; this crate cannot prove that requirement.

## 9. [Major] Checksum algorithm and width

- **Needed:** The algorithm and type of `Record.checksum`.
- **Spec:** S3 requires a checksum but pins neither.
- **Choice:** Use CRC-32/ISO-HDLC in a documented `Checksum(u32)` newtype because it is compact, deterministic, widely specified, and implementable in `no_std` without another dependency. It detects accidental/torn-record corruption; it is not authentication.

## 10. [Blocker] Exact checksum byte coverage

- **Needed:** The bytes meant by coverage “exactly `seq..payload`.”
- **Spec:** It names the logical fields but not endian order, tags, lengths, or a canonical serialization; S4's JSONL field ordering cannot define a codec-independent model.
- **Choice:** Hash `seq` as little-endian `u64`, a stable kind byte, the txn option tag and optional little-endian `EffectId`, little-endian `TurnEpoch`, then the canonical payload bytes. The checksum field itself is excluded.

## 11. [Major] Canonical bytes for typed protocol payloads

- **Needed:** Deterministic checksum input for `Submission`, `Event`, and `SessionConfigPatch` without choosing JSON.
- **Spec:** No canonical serde format is pinned for these values.
- **Choice:** Use an internal deterministic serde-token encoder with scalar tags, little-endian numbers, length prefixes, declaration-order enum indices, and explicit compound terminators/order. This is checksum canon for the slice, not a new wire codec.

## 12. [Major] Checksum validation boundary

- **Needed:** Whether `Good(record)` is already verified or the scanner verifies it.
- **Spec/request:** Scanner input is described as a decode outcome, but G2 also requires checksum-corruption tolerance.
- **Choice:** Define `Good` as decoder-accepted, including checksum validation, and defensively recompute it. Inspect the typed header epoch before that recheck so a higher epoch refuses first. A later mismatch is a decoder-contract scan error; only an explicit tail marker is tolerable.

## 13. [Major] Checksum failure away from the physical tail

- **Needed:** Distinguishing a tolerable corrupt tail from corruption in the middle of a log.
- **Spec:** Only a checksum-failed tail is tolerable; the pure iterator carries no look-behind proof that a failed record is physically last.
- **Choice:** Do not infer tail position from a checksum-invalid `Good` record: refuse it as a decoder-contract error. Tolerate only an explicit `Tail(ChecksumMismatch)` marker and require that marker to be the iterator's last outcome.

## 14. [Major] Header `codec` type

- **Needed:** A Rust type and evolution rule for S6's `codec` field.
- **Spec:** The field is named but has no enum, identifier grammar, or numeric id.
- **Choice:** Store an open `String`; the current driver may use `"jsonl"` without making it the only future value.

## 15. [Major] Header `created_at` type

- **Needed:** A representation for `Option<driver-claimed-time>`.
- **Spec:** No epoch, unit, precision, or text format is pinned.
- **Choice:** Store `Option<String>` as an opaque driver claim. The scanner does not interpret it.

## 16. [Blocker] Header placement and envelope invariants

- **Needed:** Which record must carry the header and what its `seq`, `txn`, and turn epoch must be.
- **Spec:** S6 says “log header record,” and §6 says validate its epoch first, but does not state envelope values.
- **Choice:** Require the first good record to have header kind and header payload. Do not constrain its sequence, txn, or turn epoch beyond ordinary structural checks because v0.8 pins no values for them.

## 17. [Major] Missing, duplicate, or corrupt headers

- **Needed:** Scanner behavior beyond the named higher-epoch refusal.
- **Spec:** No errors are specified for a missing header, a later second header, or a checksum-invalid header.
- **Choice:** Missing/non-header-first and duplicate headers are structural scan errors. A header must be inspectable before any tail can be tolerated.

## 18. [Major] Supported schema epoch value

- **Needed:** The epoch understood by this binary.
- **Spec:** S6 provides the field but v0.8 does not pin the current numeric value.
- **Choice:** Pass `supported_schema_epoch: u32` into the scanner; lower and equal epochs are accepted, higher epochs are refused.

## 19. [Major] Sequence start, gaps, and overflow

- **Needed:** Whether sequence numbers are contiguous, where they begin, and how repair handles `u64::MAX`.
- **Spec:** The appender writes in strict sequence order and resume seeds counters from maxima, but contiguity and origin are unstated.
- **Choice:** Require strictly increasing sequence numbers while permitting gaps and any initial value. Repairs use consecutive values after the last good record; overflow is a scan error.

## 20. [Major] Full transaction validation

- **Needed:** Rules for missing txn ids, Progress without Started, duplicate starts/terminals, and epoch mismatch.
- **Spec:** S3 explicitly names “no Terminal without Started,” while its `Started → Progress* → exactly one Terminal` wording implies stronger constraints.
- **Choice:** Lifecycle records require `Some(txn)`; enforce a unique open Started, no Progress/Terminal without it, exactly one terminal, and a constant start epoch. Both ordinary and repair terminals close the transaction.

## 21. [Major] Deterministic repair construction

- **Needed:** Repair order, sequence, txn, epoch, payload, and checksum.
- **Spec:** It only says to append one repair terminal per incomplete transaction.
- **Choice:** Preserve Started encounter order, assign contiguous sequences after the last good record, reuse each Started txn and epoch, encode `AbortedByCrash`, and calculate an ordinary checksum.

## 22. [Blocker] Meaning and timing of the replayable sequence

- **Needed:** Whether replay includes pending repair terminals and when a driver may consume it.
- **Spec:** §6 requires repair append and `Persisted` confirmation before replay; the requested pure scanner must also yield replayable records.
- **Choice:** Return repairs separately and return `replayable = valid prefix + repairs`. Documentation makes that sequence conditional: the driver must not replay it until those repairs have been durably appended.

## 23. [Major] Torn/corrupt marker suffix behavior

- **Needed:** Whether a marker is terminal and what happens to later iterator items.
- **Spec:** It calls the condition a tail but gives no outcome payload or suffix rule.
- **Choice:** `DecodeOutcome::Tail(TailFault)` stops scanning only when it is the iterator's final outcome; any following item is a structural scan error. The marker itself is never replayed.

## 24. [Major] Rewind marker shape and replay semantics

- **Needed:** Its target and whether the scanner elides earlier records.
- **Spec:** S1 names a marker, but rewind is candidate/not imported and no shape or replay algorithm is defined.
- **Choice:** Model `RewindMarker { retain_through_seq }` so the declaration has data, but do not perform elision in this isolated crash scanner.

## 25. [Blocker] Append-only law versus a retained torn JSONL tail

- **Needed:** A legal way to append crash repairs and future records after ignoring a partial/corrupt last JSONL frame.
- **Spec:** S1 forbids delete/rewrite APIs; S3 says ignore the bad tail; S4 chooses a single JSONL stream; §6 then requires repairs through the same appender. Appending after retained partial JSON bytes makes the next open encounter the same corruption before the appended records.
- **Choice:** The pure slice stops at the tail and can only return a repair plan. This remains unresolved for the driver: v0.8 must authorize tail truncation, define a resynchronizable/segmented append format, or otherwise amend S1 before persisted repair is implementable.

## 26. [Major] Concrete Rust forms for Q2 notation

- **Needed:** Definitions of `Str`, `Ref<Chunks>`, `Ref<any>`, output markers, Range, and owned Query.
- **Spec:** Q2 provides semantic notation, not Rust shapes.
- **Choice:** Use `String`, zero-sized `Records`/`Chunks`/`Any` markers, `Ref<T>` with a one-based `u32` slot and `PhantomData`, a `u64` Range, and an alloc-backed query vector.

## 27. [Major] Missing protocol `EventKind`

- **Needed:** The type and accepted spellings for `grep --kind`.
- **Spec:** Q2 names `EventKind`; `kittens-code-protocol` publishes only the open `Event` enum.
- **Choice:** Use a transparent open `EventKind(String)` and preserve any syntactically valid identifier instead of guessing a closed list from today's Event variants.

## 28. [Blocker] Which outputs may be used as `Sel::Ref`

- **Needed:** Ref-type validation for record-reading verbs.
- **Spec:** Q2 calls inputs typed but only explicitly states that `AskEach` needs Chunks; Count, Digest, DigestList, and Answer are not meaningful record selections.
- **Choice:** `Sel::Ref` is `Ref<Records>` only, `AskEach.chunks` is `Ref<Chunks>`, and Final may name any earlier successful output.

## 29. [Blocker] Closed `Instr` versus per-line error bindings

- **Needed:** Q9 continuation without adding an error instruction or error `Out` to Q2's closed IR.
- **Spec:** `Query = [Instr]`, but every erroring line must bind an error value and lowering must continue.
- **Choice:** Keep `Instr` and `Out` closed. Represent a query as `Vec<Binding>`, where each binding contains either `BoundValue::Instr` or `BoundValue::Error`.

## 30. [Major] References to an inline-error slot

- **Needed:** The type of Q9's bound error value and whether `final %N` can name it.
- **Spec:** It says the error is bound, but `Out` has no Error and `Ref<any>` is otherwise unspecified.
- **Choice:** An error binding has no declared successful output. Every later reference, including Final's `Ref<Any>`, rejects that slot with `BadRef`.

## 31. [Major] Slot numbering, blank lines, and slot capacity

- **Needed:** Whether blanks bind, whether `%0` exists, and a maximum line number.
- **Spec:** `%N` is unrestricted digits; QueryTrace uses a one-based line, while Q9 does not define blank-line recovery.
- **Choice:** Number nonempty physical lines from `%1`; empty or ASCII-whitespace-only lines do not consume slots; `%0`, self, forward, missing, and overflowed refs are `BadRef`. Slots use `u32`; no script hard maximum is pinned.

## 32. [Minor] Whitespace and line endings

- **Needed:** Token separators, leading/trailing whitespace, CRLF, and final EOF behavior.
- **Spec:** Appendix A defines no whitespace production and formally requires `newline` after every line.
- **Choice:** ASCII space and tab are separators; leading/trailing separators are accepted; LF and CRLF are accepted; a final newline is optional; comments are not supported.

## 33. [Major] Contradictory string escape grammar

- **Needed:** Exact backslash handling.
- **Spec:** Q2 permits only `\"` and `\\`; Appendix A's general character arm also admits an unescaped backslash and does not clearly encode `\\`.
- **Choice:** Q2 prose wins: decode only `\"` and `\\`; reject every other escape, unterminated quote, and raw CR/LF as `Parse`; preserve other UTF-8 scalars.

## 34. [Major] Flag value attachment and ordering

- **Needed:** Whether `--ctx 2` equals `--ctx=2`, whether flags may move among positionals, and duplicate behavior.
- **Spec:** `flag = "--" ident [ "=" value ]`, but no KC0 verb has a boolean flag and the semantic table gives no ordering.
- **Choice:** The lexer permits an omitted value, but every known flag semantically requires `=value`; missing/wrong values and duplicate or unknown flags are `BadFlag`. Flags may appear anywhere between positional tokens.

## 35. [Blocker] Exact per-verb arity, order, flags, and defaults

- **Needed:** A single accepted text surface for the Q2 fields.
- **Spec:** Appendix A only supplies generic `arg`; the semantics table does not bind fields to positions or defaults.
- **Choice:** Use: `grep STRING [SEL] [--ctx=N] [--kind=IDENT]` with `ctx=0`; `slice [SEL]`; `head N [SEL]`; `tail N [SEL]`; `count [STRING] [SEL]`; turns/bytes `partition [SEL] --by=... --size=N`; regex `partition [SEL] --by=regex STRING`; `ask [SEL] STRING [--sample-k=N]`; `ask-each %N STRING`; `final STRING|%N`. Omitted selectors mean Whole. Final is neither required nor constrained to the last line; later lines still lower. Regex syntax is otherwise opaque except the explicit inline-flag rejection described in Q6.

## 36. [Major] String coercions and numeric edge values

- **Needed:** Whether bare identifiers/numbers become `Str`, and whether zero is legal for numeric fields.
- **Spec:** Generic `value` includes ident and number, while typed forms require `Str`; no positivity rules are stated.
- **Choice:** Only quoted strings lower to `Str`; bare identifiers are used only for enum-like flag values. `ctx`, `n`, `size`, and `sample-k` accept zero. Width overflow is `BadFlag` for flag fields and `Parse` for positional `u32` counts.

## 37. [Major] Static versus data-dependent range validation

- **Needed:** Integer width, equal bounds, overflow, and “out of unit bounds” checks without a store.
- **Spec:** Ranges are inclusive-start/exclusive-end and protocol `BadRange` mentions out-of-unit bounds, but lowering receives no transcript metadata.
- **Choice:** Store both bounds as `u64`; accept `start == end` as an empty range; reject malformed, overflowing, unknown-unit, and `start > end` ranges as `BadRange`. Defer transcript-size bounds to execution.

## 38. [Major] Exact `VerbErrorCause` partition

- **Needed:** Stable error oracles for arity, token, flag, ref, range, and regex failures.
- **Spec:** P8 names five broad causes but does not map individual validation failures.
- **Choice:** Use `BadRef` for missing/forward/wrong-output refs; `BadRange` for recognized malformed/inverted/overflowing ranges; `BadFlag` for duplicate/unknown/missing/wrong-shaped flags; `Parse` for lexical failures, unknown verbs, arity/type mismatches, unsupported escapes, and rejected inline regex flags. The error's verb is the first token or empty when none can be read.

## 39. [Blocker] Making `VerbErrorCause::Budget` reachable during lowering

- **Needed:** The requested rejection suite must reach every cause, but this crate does not execute aggregate meters.
- **Spec:** Q5/P8 describe Budget as an in-script aggregate-meter error; only verb count is statically knowable here, and no hard maximum is pinned.
- **Choice:** Provide `lower_script_with_verb_limit(script, max_verbs)` and an unlimited convenience entry point. Every prior nonempty binding, including an error binding, consumes the supplied limit. Each later line is fully validated first; an otherwise-valid line at or over the limit binds Budget, so a syntax/type oracle is not masked, and lowering continues.

## 40. [Blocker] Experimental crate versus the topology-admission law

- **Needed:** Permission to add `kc0-slice` as a workspace crate without first adding it to the normative crate topology.
- **Spec:** T4 says new crates require a spec change, T5 requires an admission-ledger oracle, and §3 lists `kittens-code-core` rather than this evidence-only crate; the task simultaneously forbids edits to the existing docs.
- **Choice:** Treat `kc0-slice` as a deliberately isolated experiment authorized by the task, with this ledger as its local decision record. It does not claim satisfaction or amendment of T4/T5 and must not silently become the production core crate.

## 41. [Major] Complete Q6 regex validation under the allowed dependencies

- **Needed:** Full validation of the pinned regex 1.x dialect, including syntax, inline-flag rejection, and unsupported constructs.
- **Spec:** Q6 assigns this to query validation, while D7 leaves the no-std dialect open; the requested manifest permits only protocol and serde dependencies, not the `regex` parser named for the std driver.
- **Choice:** Keep patterns opaque after lexing and reject obvious unescaped inline-flag prefixes such as `(?i)` with a small dependency-free check. Full regex syntax/dialect validation remains executor/driver work, so this slice does not claim the complete Q6 gate.

## 42. [Major] Checksum canon across older schema epochs

- **Needed:** A newer binary must verify checksums written by every older accepted schema epoch.
- **Spec:** Replay accepts lower epochs, but no per-epoch checksum canon or preserved covered-byte representation exists; additive protocol fields can change the serde-token sequence used by this slice.
- **Choice:** The implemented canon is valid only for the current modeled shapes. Production replay must version the checksum encoder by schema epoch or verify preserved canonical persisted bytes; this slice does not establish backward checksum compatibility.

## 43. [Major] Admission boundary for constructed or deserialized IR

- **Needed:** The same range, partition, and reference invariants when IR arrives from future typed calls or serde rather than text lowering.
- **Spec:** Q2 says all surfaces lower to the typed IR and assigns validation at lowering, but it does not define constructors or a trusted-deserialization boundary.
- **Choice:** Keep the enum fields and serde forms public for this probe; only `lower_script` is an admission checker. Callers that construct or deserialize IR must validate it before execution. A production core needs one canonical validating constructor/decoder before admitting non-text surfaces.
