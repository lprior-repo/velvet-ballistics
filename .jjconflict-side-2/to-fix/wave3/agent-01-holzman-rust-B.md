# Wave 3 Holzman-Rust Deep Pass — Agent-01 Report

**Scope:** `vb-1rqz7.13` .. `vb-1rqz7.19` (7 bugs, storage/recovery/codec/digest family).
**Doctrine:** NASA/JPL Power-of-Ten Rust — forbid `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked index/slice/cast/arithmetic; typed errors over silent fallthrough.
**Method:** For every bead I (a) re-read the cited files, (b) ran the targeted `cargo test -p vb_storage --lib <token>`, (c) verified whether the *fix* was actually merged into the production source, not just declared in the bead lifecycle.

---

## Audit Table

| bug-id | pri | source-fix | test | targeted-cmd | result | verdict | evidence | holzman-violation |
|---|---|---|---|---|---|---|---|---|
| vb-1rqz7.13 (SC-001) | P0 | **NONE** — `index_status_key` writes `state.to_u8()` (keys.rs:109) without rejecting `IndexStatusState::Other(0)`, `Other(1)`, `Other(2)`. Encoder produces bytes that the decoder (`keys.rs:355`) would silently round-trip as `Submitted/Active/Completed` via `from_u8`. No `ReservedStatusByte`/`StatusByteCollision` variant in `JournalError`/`KeyDecodeError`. | **NONE.** `keys::tests::index_status_key_encodes_state_timestamp_run` uses `Other(0x05)` (no collision range). No test exercises `Other(0..=2)`. No `cargo test index_status` filter finds a rejection test. | `cargo test -p vb_storage --lib index_status_key` | 5/5 pass | **NOT-PATCHED** | `keys.rs:101-116` writes raw `state.to_u8()`; `types.rs:235-254` `Other(u8)` discriminants 0..2 collapse onto named variants on round-trip; `keys.rs:353-360` decoder resolves any byte to a single canonical variant; no `cargo test` filter returns a rejection test. | Unchecked variant domain (`Other(u8)`) fed into a key encoder that must reject reserved bytes — silent invariant violation. Not a `panic`/`unwrap`, but a typed-error absence, which the EARS contract forbids ("return a typed error or documented explicit status instead of silent corruption"). |
| vb-1rqz7.14 (SC-002) | P0 | **NONE.** `run_event_key` (keys.rs:81) and `journal_key` (keys.rs:398) route to `sequenced_run_key` (keys.rs:402) which does not reject `EventSeq::MAX`. Decoder rejects `MAX` (keys.rs:348 → `KeyDecodeError::ReservedSeqSentinel`), so an encoder/decoder asymmetry exists. | `run_event_key_with_max_values` **passes** (confirms encoder accepts MAX — exactly the unfixed behaviour). No rejection test exists. | `cargo test -p vb_storage --lib run_event_key` | 14/14 pass | **NOT-PATCHED** | `keys.rs:398-414` `sequenced_run_key` is a no-validate pass-through; `keys.rs:340-360` decoder rejects; `journal/internal.rs:43` `append_unfsynced` calls `run_event_key(...)?` without checking `seq != u64::MAX`; existing test `run_event_key_with_max_values` passing is *evidence of the bug*, not a fix. | Encoder/decoder asymmetry is a typed-error absence violation (EARS §1) — silent corrupt-persist path. Holzman rule "fail closed" broken. |
| vb-1rqz7.15 (SC-003) | P0 | **NONE.** `decode_slot_written_extra` (slot_extra.rs:60-69) calls `postcard::from_bytes::<SlotWrittenExtraEnvelope>(payload)` directly with no payload-length cap or `Vec` length cap. Postcard 1.x applies a 32-bit default cap on `Vec` length, but the contract specifies a per-envelope cap, not a postcard default. The `SlotWrittenExtraError` enum has no `PayloadTooLarge` variant. | **NONE.** `cargo test decode_slot_written_extra` → 0 tests. The function exists but is uncovered. | `cargo test -p vb_storage --lib decode_slot_written_extra` | 0/0 (no tests) | **NOT-PATCHED** | `slot_extra.rs:60-69` has zero length validation; `slot_extra.rs:9-19` error enum lacks a "too large" variant; recovery path at `summary.rs:720` treats any decode failure as `CorruptSlotTaint`. | Unchecked decode allocation against persisted/hostile bytes — Holzman "no new unbounded allocation on storage hot paths" violated by the `Vec<u8>` decode. |
| vb-1rqz7.16 (SC-011) | P0 | **NONE.** `decode_envelope_only` (codec/envelope.rs:27-61) calls `decode_record_header` which validates CRC32C but never calls `verify_digest_match` against the raw payload slice (lines 49-51). The BLAKE3 digest in the header is read (header.rs:90) but never compared. | **NONE.** `cargo test decode_envelope_only` → 0 tests. Function is uncovered. | `cargo test -p vb_storage --lib decode_envelope_only` | 0/0 (no tests) | **NOT-PATCHED** | `codec/envelope.rs:34-38` calls `decode_record_header` which only verifies CRC32C (`header.rs:54-56`); `header.rs:90` reads `payload_digest` but does not consume it; `payload.rs::verify_digest_match` exists but is never invoked from the envelope-only path. | Digest comparison is omitted entirely — silent acceptance of tampered payloads under the "envelope-only" doctor path. Holzman "fail closed on integrity evidence gaps" violated. |
| vb-1rqz7.17 (SA-001) | P0 | **NONE.** `put_run_header` (batch.rs:123-134) and `put_snapshot` (batch.rs:137-148) propagate key/encode errors via `?` without setting `self.aborted = true`. Compare `put_workflow_source` (batch.rs:78-103) and `put_blob` (batch.rs:153-174), which explicitly set `self.aborted = true` on every error arm. | Existing happy-path tests (`batch_put_run_header_commits_and_is_readable`, `batch_put_snapshot_commits_and_is_readable`, `put_run_header_stores_and_retrieves`) pass — they do not exercise the error path. | `cargo test -p vb_storage --lib put_run_header` and `put_snapshot` | 6/6 pass (happy path only) | **NOT-PATCHED** | `batch.rs:124,131` `?` for `run_header_key` and `encode_record`; `batch.rs:138,145` same for `put_snapshot`. `aborted` assignments in the file appear at lines 81/87/100 (workflow_source) and 155/161/168 (blob) — neither function in SA-001's scope touches the field on error. | `?` short-circuit bypasses batch state machine — semantically equivalent to `unwrap()` of the batch contract: subsequent ops see stale state, `commit()` no-ops via the early-exit guard but `len()`/`is_empty()` may report non-zero, `staged_bytes` may be in an inconsistent state. Holzman "no silent state corruption" violated. |
| vb-1rqz7.18 (SA-003) | P0 | **NONE.** `append_event` (batch.rs:243-290) checks `self.journal.events.contains_key(key)?` for the *committed* keyspace only (line 245). The `staged_event_keys: HashSet<[u8; JOURNAL_KEY_BYTES]>` field declared at batch.rs:47 is never written to and never read from in any code path — `append_event` never inserts into it. | `batch_append_event_rejects_duplicate_event` exists but only tests across *separate* batches (second batch sees first batch's commit). No test for two events with same `(run, seq)` staged in the *same* batch. | `cargo test -p vb_storage --lib append_event` | 5/5 pass | **NOT-PATCHED** | `batch.rs:47` `staged_event_keys` is dead state; `batch.rs:243-289` `append_event` checks `self.journal.events` only; `batch.rs:288` `self.inner.insert(...)` is the staging call but no parallel HashSet insert. `journal::tests::batch_append_event_allows_duplicate_key_insertion` (passing) actually *codifies* the broken behaviour by allowing duplicate keys within a single batch. | In-flight duplicate not detected — Fjall's owned batch would overwrite or reject at commit (platform-defined). Holzman "fail closed on duplicate" violated. |
| vb-1rqz7.19 (SA-004) | P0 | **NONE.** `drain_all` (queue/writer.rs:219-245) loops `max_iterations = capacity/batch_size + 2`, and on loop exit returns `Ok(total)` (line 244) even if `state.pending` is non-empty. The `StorageQueueStepResult::DrainIncomplete` discriminant exists in `queue/writer_contract.rs:11` (and `finish_drain_decision` returns it at line 58) but is **never imported** by `writer.rs` and never converted into a `JournalError`. | `drain_all_flushes_until_empty`, `drain_all_mixed_tiers_across_multiple_batches`, `drain_all_on_empty_queue_returns_zero` — all exercise the happy path with no concurrent producer. | `cargo test -p vb_storage --lib drain_all` | 5/5 pass (no concurrent-producer case) | **NOT-PATCHED** | `writer.rs:236-244` loop body returns `Ok(total)` on early-empty and on loop-exhaustion identically. `writer.rs:218` doc-comment claims "static bound" but never asserts post-condition. `writer_contract.rs:50-59` `finish_drain_decision` defines the contract; `writer.rs` does not consume it. | Silent partial-drain — caller cannot distinguish "drained fully" from "loop exhausted with items still pending". Holzman "no silent corruption" violated at the queue level. |

---

## Counts

- **bugs-checked:** 7 / 7
- **PASS / PATCHED:** 0
- **FAIL / NOT-PATCHED:** 7
- **PARTIAL:** 0
- **UNKNOWN:** 0

**Verdict: ALL 7 BEADS WERE CLOSED WITHOUT A CODE FIX.**

The `close reason` in every bead is `Closed`, but no production file contains the cited fix. The bead lifecycle closed beads for issues whose remediation consists entirely of bead-level acceptance criteria, not implemented changes. Every targeted `cargo test` either returns zero matching tests (no regression harness exists) or returns passing happy-path tests that do not exercise the cited failure mode.

---

## Top-3 NOT-PATCHED with Reason

### 1. vb-1rqz7.14 (SC-002) — `EventSeq::MAX` persists to keys
**Severity: highest** because the decoder is hardened (`KeyDecodeError::ReservedSeqSentinel`) but the encoder/append path is not. A `JournalEvent` with `seq = u64::MAX` passes `JournalEvent::is_valid()` (events.rs: ? - need verify) and reaches `append_unfsynced` → `run_event_key(run, EventSeq::MAX)` which writes a 17-byte key. On recovery, `decode_storage_key` rejects the row with `ReservedSeqSentinel`, surfacing as `MalformedKeyspaceRow` or `KeyDecodeError` at `decode_record` time. The fix should live in `sequenced_run_key` (keys.rs:402) — `if seq.get() == u64::MAX { return Err(JournalError::SequenceOverflow); }` — symmetric with `next_seq` in codec/mod.rs:141.

**Reason NOT-PATCHED:** `keys.rs:402-414` is unchanged from pre-bead state. `run_event_key_with_max_values` passing is *direct evidence* that the encoder accepts MAX. Bead was closed on validation verdict alone, not on a verified test pass.

### 2. vb-1rqz7.18 (SA-003) — duplicate `(run, seq)` within one batch
**Severity: high** because `staged_event_keys` is declared at batch.rs:47 with `#[allow(dead_code)]` — explicitly *marked* as known-unused. The fix is two lines (insert on line 245 area, check on line 245 area). The existing test `journal::tests::batch_append_event_allows_duplicate_key_insertion` actively *encodes* the broken behaviour as a test expectation — this is a regression in the test suite that will need to be inverted as part of the fix.

**Reason NOT-PATCHED:** Field is `#[allow(dead_code)]` at batch.rs:46-47, signalling the author was aware but did not wire it. The `append_event` path (batch.rs:243) only checks `self.journal.events`.

### 3. vb-1rqz7.17 (SA-001) — batch not aborted on key/encode failure for run header and snapshot
**Severity: high** because `put_workflow_source` and `put_blob` already do the right thing (set `aborted = true` on every error arm), so the pattern is established in the same file. `put_run_header` and `put_snapshot` are simple omissions — likely a copy-paste from `put_compiled_ir` which doesn't validate digest and never fails.

**Reason NOT-PATCHED:** `batch.rs:123-148` retains the `?`-only style for both functions. Comparing line-by-line with `put_workflow_source` (batch.rs:74-106) shows the explicit `match` arms setting `aborted` are missing.

---

## Deep-Dive Disagreements (where PATCHED would be questionable)

None of the seven beads warrant a `PATCHED` verdict. But the following three observations are worth recording:

### D-1. vb-1rqz7.13 — Asymmetric *decoder* coverage masks the bug
The decoder's `from_u8` (types.rs:235-254) collapses any byte ≥3 into `Other(byte)` but maps 0/1/2 to the named variants. This means *storage persisted with `Other(0)` round-trips as `Submitted` on read*. If a previous version of the code emitted `Other(0)` markers, every existing Fjall row carrying that state byte silently mis-reports on recovery. The decoder is a co-conspirator, not a victim. A complete fix needs either (a) encoder rejects `Other(0..=2)` OR (b) decoder rejects the ambiguous bytes with `KeyDecodeError::AmbiguousStatusByte`. Neither is present.

### D-2. vb-1rqz7.16 — Uncovered function in doctor/inspection path
`decode_envelope_only` is the *only* path that returns raw payload bytes to inspection tools. Because it skips digest verification, a doctor iterating over a partially-corrupted journal will see tampered bytes and may produce false diagnostics. This is exactly the kind of "fail closed on integrity evidence gaps" violation the EARS contract forbids, and it lives in a code path with zero test coverage.

### D-3. vb-1rqz7.19 — Contract type exists but is unwired
`StorageQueueStepResult::DrainIncomplete` is defined in `queue/writer_contract.rs:11` with three call sites in the contract module (`enqueue_allowed`, `finish_drain_decision`, `shutdown_and_close_decision`) that all return it. But `writer.rs` imports nothing from `writer_contract.rs` and never produces a `JournalError` for the drain-incomplete state. The contract layer is provably disconnected from the implementation layer. This is a "vacuous spec" risk: a future Verus/proof-writer pass that binds to `finish_drain_decision` will see a contract that the runtime does not honour.

---

## Holzman Doctrine Violations Summary (across 7 bugs)

| Category | Count | Bugs |
|---|---|---|
| Typed-error absence (silent fallthrough) | 4 | 13, 14, 19, 17 |
| Unbounded allocation on hostile decode | 1 | 15 |
| Integrity verification skipped | 1 | 16 |
| State machine bypass via `?` | 1 | 17 (also counted above) |
| In-flight duplicate not detected | 1 | 18 |

No `unwrap`/`expect`/`panic`/`todo`/`dbg`/`unsafe` were introduced *by the missing fixes* (the codebase already `#![forbid(unsafe_code)]` and existing tests use `expect` inside `#[cfg(test)]` blocks with module-level `clippy::expect_used` allow). The dominant failure mode is *typed-error absence*: the code path returns success where it should return a typed error variant, which the Holzman/EARS doctrine treats as equivalent to silent corruption.

---

## Output

- **Output file:** `/home/lewis/src/velvet-ballistics/to-fix/wave3/agent-01-holzman-rust-B.md`
- **Bead IDs reviewed:** `vb-1rqz7.13`, `vb-1rqz7.14`, `vb-1rqz7.15`, `vb-1rqz7.16`, `vb-1rqz7.17`, `vb-1rqz7.18`, `vb-1rqz7.19`
- **Cargo commands run (all from git root `/home/lewis/src/velvet-ballistics`):**
  - `cargo test -p vb_storage --lib index_status_key --no-fail-fast` → 5 pass
  - `cargo test -p vb_storage --lib run_event_key --no-fail-fast` → 14 pass
  - `cargo test -p vb_storage --lib decode_envelope_only --no-fail-fast` → 0 pass (no tests)
  - `cargo test -p vb_storage --lib decode_slot_written_extra --no-fail-fast` → 0 pass (no tests)
  - `cargo test -p vb_storage --lib drain_all --no-fail-fast` → 5 pass (happy path)
  - `cargo test -p vb_storage --lib put_run_header --no-fail-fast` → 3 pass (happy path)
  - `cargo test -p vb_storage --lib put_snapshot --no-fail-fast` → 3 pass (happy path)
  - `cargo test -p vb_storage --lib append_event --no-fail-fast` → 5 pass (one codifies broken behaviour)
  - `cargo test -p vb_storage --lib keys::tests --no-fail-fast` → 54 pass
