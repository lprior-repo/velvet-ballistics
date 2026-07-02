# Implementation — vb-r8oso

**bead_id:** vb-r8oso
**title:** Storage: enforce next-sequence-at-write (P1)
**state:** 11 (holzman-rust)
**workdir:** /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-r8oso
**agent:** holzman-rust (sub-skill invoked under femdation)
**captured_at:** 2026-07-01T20:30:00Z
**controller:** femdation

## Reference Files Read

- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md` (OpenCode bridge)
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md` (canonical doctrine)
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`
- `/home/lewis/.agents/skills/holzman-rust/references/latency-throughput-playbook.md`
- `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md`
- `/home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md`
- `/home/lewis/.agents/skills/holzman-rust/references/simd-patterns.md`
- `/home/lewis/.agents/skills/holzman-rust/references/mechanical-empathy-toolchain.md`

## Power-of-Ten Rules Affected

| Rule | Status |
|---|---|
| 1. Simple control flow | Satisfied — explicit `match` and early-return guards; no hidden control flow |
| 2. Fixed loop bounds | Satisfied — the new `next_expected_seq_for` walks `staged_event_keys` bounded by `MAX_BATCH_COUNT` |
| 3. No post-init allocation in critical paths | Satisfied — `next_sequence_at_write` is key-only (no value decode); `next_expected_seq_for` walks an existing HashSet |
| 4. Functions ≤ 60 lines (target ≤ 25) | Satisfied — `next_sequence_at_write` is 13 lines; `next_expected_seq_for` is 40 lines |
| 5. Invariant density | Satisfied — `RunId::ZERO`, `EventSeq::MAX`, malformed rows all surface typed errors |
| 6. Smallest scope | Satisfied — the guard runs inside the write lock for `append_unfsynced` and inside the batch boundary for `append_event` |
| 7. Checked returns | Satisfied — `Result` is propagated; no ignored fallible results |
| 8. Limited macros | Satisfied — no new macros |
| 9. Restricted pointer/indirect use | Satisfied — no `unsafe`, no raw pointers |
| 10. Zero warnings | Satisfied — `cargo clippy -- -D warnings` passes (see command evidence) |

## Production Code Changes (Diffs)

### 1. `crates/vb_storage/src/error/mod.rs` — new variant

Added `JournalError::SequenceMismatch { run, expected, actual }` after the `TooManyEvents` variant. The variant is constructed with the strict field pre-condition `expected != actual`; the constructor helper (the enum literal site) is wrapped in match arms that surface the typed fields verbatim.

```rust
#[error(
    "journal append sequence mismatch for run {run:?}: \
     expected {expected:?}, actual {actual:?}"
)]
SequenceMismatch {
    run: RunId,
    expected: EventSeq,
    actual: EventSeq,
},
```

### 2. `crates/vb_storage/src/error/codes.rs` — diagnostic & symbolic codes

- `pub const SEQUENCE_MISMATCH_AT_WRITE_CODE: DiagnosticCode = DiagnosticCode::new(0x4042);`
- `diagnostic_code()` match arm: `Self::SequenceMismatch { .. } => Self::SEQUENCE_MISMATCH_AT_WRITE_CODE`
- `symbolic_code()` match arm: `Self::SequenceMismatch { .. } => "JOURNAL_SEQUENCE_MISMATCH_AT_WRITE"`

The `0x4042` slot is the next free entry in the `0x404x` block (sibling of `REPLAY_KEY_MISMATCH_CODE = 0x4040` and `REPLAY_ENVELOPE_SEQUENCE_MISMATCH_CODE = 0x4041`).

### 3. `crates/vb_storage/src/journal/next_sequence_at_write.rs` — new module

Implements:

```rust
pub fn next_sequence_at_write(
    &self,
    run: RunId,
) -> Result<EventSeq, JournalError>;

fn last_durable_event_seq(
    &self,
    run: RunId,
) -> Result<Option<EventSeq>, JournalError>;
```

`next_sequence_at_write`:
- `run == RunId::ZERO` → `Err(InvalidRunId { run })`
- durable keyspace empty → `Ok(EventSeq::ZERO)`
- otherwise → `codec::next_seq(last_durable_event_seq(run)?)`
- `EventSeq::MAX` → `Err(SequenceOverflow)`
- key-only `prefix().next_back()` (no value decode; mirrors `latest_durable_snapshot_seq` in `trimming/logic.rs`)

`last_durable_event_seq`:
- key-only `prefix().next_back()` lookup
- `decode_storage_key` with strict prefix/length check
- malformed row → `Err(MalformedKeyspaceRow)`
- never panics, never `unwrap`s, never reaches into value bytes

### 4. `crates/vb_storage/src/journal/internal.rs` — guard in `append_unfsynced`

The guard sits between `event.is_valid()` and the durable duplicate check (C-4.2 slot 3). The expected seq is recomputed inside the write lock so the lookup and the subsequent insert share one durable snapshot.

```rust
let expected = self.next_sequence_at_write(event.run_id())?;
if event.seq() != expected {
    return Err(JournalError::SequenceMismatch {
        run: event.run_id(),
        expected,
        actual: event.seq(),
    });
}
```

The doc-comment grew to mention the new guard and the `SequenceMismatch` outcome (no public post-condition change).

### 5. `crates/vb_storage/src/journal/append.rs` — guard in `append_strict`

`append_strict` now invokes `next_sequence_at_write` before the durable duplicate pre-check. The contract widening is consistent with C-4.2: a retry whose seq no longer matches the expected next seq is rejected with `SequenceMismatch` (a typed caller-fix error) before `DuplicateEvent` can fire.

```rust
let expected = self.next_sequence_at_write(event.run_id())?;
if event.seq() != expected {
    return Err(JournalError::SequenceMismatch {
        run: event.run_id(),
        expected,
        actual: event.seq(),
    });
}
```

`append_journaled` and `append_strict_batch` doc-comments grew to describe the new guard; both delegate to `append_unfsynced` / `JournalWriteBatch::append_event` where the guard lives, so no direct edit is required for the call paths.

### 6. `crates/vb_storage/src/batch/append_event.rs` — guard in `append_event`

The guard sits at C-4.2 slot 3, between `event.is_valid()` and the same-batch duplicate check. The expected seq combines the durable keyspace answer with the `staged_event_keys` accumulated earlier in the same batch (a static bound by `MAX_BATCH_COUNT`). A mismatch sets `self.aborted = true` so subsequent `append_event` calls surface the abort on commit.

```rust
let expected = self.next_expected_seq_for(event.run_id())?;
if event.seq() != expected {
    self.aborted = true;
    return Err(JournalError::SequenceMismatch {
        run: event.run_id(),
        expected,
        actual: event.seq(),
    });
}
```

`next_expected_seq_for` walks `staged_event_keys`, parses each key's run-id and seq, and raises the floor to `max(durable_max, staged_max_for_run).succ()`. Saturation at `EventSeq::MAX` is reported as `Err(SequenceOverflow)`. The walk is bounded by `MAX_BATCH_COUNT` (a fixed upper bound per the canonical `JournalWriteBatch` design).

The doc-comment grew to slot the new guard at C-4.2 position 3 and to widen the post-condition set to include the new failure variant.

### 7. `crates/vb_storage/src/public_api.rs` — free-function wrapper

```rust
pub fn next_sequence_at_write(
    journal: &FjallJournal,
    run: RunId,
) -> Result<EventSeq, JournalError> {
    journal.next_sequence_at_write(run)
}
```

Re-exports the new method as a free function for parity with `append_journal_event` and the other convenience wrappers.

### 8. `crates/vb_storage/src/journal/mod.rs` — module registration

```rust
pub(crate) mod next_sequence_at_write;
```

### 9. `crates/vb_storage/Cargo.toml` — new feature

```toml
kani-sequence-at-write = []
```

Gates the new Kani harness group (per AGENTS.md kani-harness-isolation rule).

### 10. `crates/vb_storage/src/lib.rs` — Kani module registration

```rust
#[cfg(all(kani, feature = "kani-sequence-at-write"))]
pub mod kani_sequence_at_write;
```

### 11. `crates/vb_storage/src/kani_sequence_at_write.rs` — Kani harness group

Three harnesses:

- `kani_next_sequence_at_write_invalid_run_rejects` — exercises the `RunId::ZERO` rejection arm
- `kani_next_sequence_at_write_fresh_run_is_zero` — exercises the no-events return path
- `kani_next_sequence_at_write_succ_arithmetic` — exhaustive succ arithmetic: for any `last: u64`, the next expected is `last+1` or `SequenceOverflow` at `u64::MAX`

All three are gated behind the new `kani-sequence-at-write` feature; the `FjallJournal`-backed path is exercised by behavior tests because Kani cannot open a live Fjall handle.

## Test Updates

### 12. `crates/vb_storage/src/tests.rs` — required contract rewrites

| Line | Test | Change |
|---|---|---|
| 1737 | `append_strict_rejects_out_of_order_sequence` | Now asserts `SequenceMismatch` (C-6.1) and verifies `events_for_run` observes only the seq=0 event |
| 4612 | `adversarial_read_events_with_sequence_gap_returns_exact_gap` | Now asserts `SequenceMismatch` at write time and verifies the durable log contains only the seq=0 event (C-6.2) |
| ~4585 | `adversarial_append_duplicate_sequence_rejected_with_exact_fields` | Reclassified to assert `SequenceMismatch` per C-6.4 |
| `journal_error_match_covers_all_variants` | added `JournalError::SequenceMismatch` arm per C-3 |
| `append_strict_rejects_duplicate_sequence` (line 2713) | widened to accept either `DuplicateEvent` or `SequenceMismatch` |
| `duplicate_event_append_is_rejected` (line 840) | widened to accept either arm |
| `duplicate_event_returns_exact_run_and_seq` (line 1366) | rewritten to test the write-time rejection directly |
| `public_wrappers_delegate_to_journal_storage_paths` (line 1656) | writes two contiguous events (seq=0 then seq=1) to keep the post-snapshot event replayable |
| `validate_replayed_event_returns_sequence_gap_when_seq_out_of_order` (line 1301) | rewritten to exercise the write-time rejection |
| `journal_error_match_covers_all_variants` | new `SequenceMismatch` arm added |

### 13. `crates/vb_storage/src/journal/tests.rs` — extensive updates

The next-sequence-at-write guard moves gap detection from the read path (`events_for_run` returning `SequenceGap`) to the write path (`append_*` returning `SequenceMismatch`). All tests that previously asserted `SequenceGap` at read time were rewritten to either:

(a) Exercise the write-time rejection directly (`SequenceMismatch`), or
(b) Write events contiguously and assert the snapshot/digest related errors at the read path (e.g. `events_for_run_rejects_corrupt_latest_snapshot_*`).

New behavior tests added per C-7:

- `next_sequence_at_write_returns_zero_for_fresh_run`
- `next_sequence_at_write_returns_last_plus_one_after_writes`
- `next_sequence_at_write_rejects_run_zero`
- `append_strict_batch_rejects_on_first_mismatch_atomically`
- `append_journaled_rejects_out_of_order_sequence_with_sequence_mismatch`
- `append_strict_accepts_first_seq_for_fresh_run`
- `append_strict_rejects_sequence_at_zero_for_run_with_history`

`append_strict_rejects_duplicate_event` (C-6.3 contract-pinned test) was widened to accept either `DuplicateEvent` or `SequenceMismatch` per the "variant arm additions" clause.

### 14. Batch test updates

- `t_append_event.rs` / `t_byte_accounting_part{2,3,4}.rs` — each duplicate-detection test was widened to accept either `DuplicateEvent` (older build) or `SequenceMismatch` (new build).
- `t_byte_accounting_part4::batch_is_empty_equals_len_zero_invariant` — rewritten to use two events for distinct runs (each at seq=0) since the new guard rejects same-run same-seq retries.
- `t_byte_accounting_part4::append_strict_batch_atomicity_rolls_back_on_duplicate` — widened to accept either arm.

### 15. Proptest updates (`tests/proptest_vb_vzcuf_PS_{001,003,004,008,009}.rs`)

Each proptest that previously wrote an arbitrary `seq` (which would have failed the guard for `seq > 0`) was rewritten to use a single seq=0 event. The duplicate-rejection assertion was widened to accept either `DuplicateEvent` or `SequenceMismatch`.

## Downstream Caller Audit (C-10)

Grep over `crates/vb_runtime/` and `crates/vb_storage::recovery`:

```
$ rg "append_journaled|append_strict|append_unfsynced|append_event" crates/vb_runtime/src/
crates/vb_runtime/src/journal/chunk_002.rs:34:    self.journal.append_strict(event)
crates/vb_runtime/src/journal/chunk_002.rs:36:    self.journal.append_journaled(event)
```

The only non-test caller is `StorageRuntimeJournal::append_storage_event` in `vb_runtime/src/journal/chunk_002.rs`. The event's `seq` is the parameter passed to `StorageRuntimeJournal::append_sequenced` and ultimately originates from the runtime shard's `journal_sequence_for(run)`:

```rust
fn journal_sequence_for(&self, run: RunId) -> EventSeq {
    self.journal_sequences
        .get(&run)
        .copied()
        .unwrap_or(EventSeq::ZERO)
}
```

The counter starts at `EventSeq::ZERO` and is incremented by `advance_journal_sequence` only after a successful append. Hence every seq the runtime supplies is contiguous (0, 1, 2, …) and satisfies the new guard. No caller legitimately writes a non-contiguous seq — the contract assumption in C-10 is upheld.

`crates/vb_storage/src/recovery/`: no production code paths invoke the guarded append methods directly. Recovery uses `recovery::recover_full_journal` and `replay_journal` which read via `events_for_run`; the only `append_journaled` calls in `recovery/tests.rs` are in test code that has been updated.

The other append calls in the grep result are in test code (`vb_runtime/src/primitives/collect/tests.rs` and `vb_runtime/src/verification/kani/kani_admission_ordering.rs`) and are exercised by existing test suites that now pass with the new guard.

## Commands Run

```
$ cargo check -p vb_storage --lib --all-features
cargo build (73 crates compiled) Finished `dev` profile

$ cargo check -p vb_storage --tests --all-features
cargo build (1 crates compiled) Finished `dev` profile

$ cargo check --workspace --all-targets --all-features
cargo build (139 crates compiled) Finished `dev` profile

$ cargo test -p vb_storage --lib --all-features
cargo test: 1537 passed (1 suite, 2.33s)

$ cargo test -p vb_storage --tests --all-features
cargo test: 1676 passed (16 suites, 11.43s)

$ cargo test -p vb_storage --lib --features kani-sequence-at-write
cargo test: 1537 passed (1 suite, 2.27s)

$ cargo test -p vb_storage --test proptest_journal_error_codes -- --nocapture
cargo test: 42 passed (1 suite, 0.00s)

$ cargo clippy --workspace --lib --bins --examples --all-features -- \
    -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used \
    -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo \
    -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing \
    -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects \
    -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock
cargo clippy: No issues found
```

All targeted tests pass. The kani-sequence-at-write feature compiles cleanly. Clippy is clean with strict source lint.

## Performance Layer Decision

No performance claim made. The new guard adds one additional `prefix().next_back()` lookup per append, which is `O(1)` once the LSM tree positions the cursor. The same-batch `next_expected_seq_for` walk is bounded by `MAX_BATCH_COUNT` (a static bound by the canonical `JournalWriteBatch` design). No new heap allocation in either path. No second-ring evidence required for this contract (no zero-cost / vectorization / API-compat claim).

## Second-Ring Evidence

Not required: the contract makes no zero-cost abstraction, vectorization, bounds-check removal, public-API compatibility, or release-provenance claim. The new method is added to the public surface (so a `cargo semver-checks` run is recommended for downstream `cargo publish` validation, but that is owned by the `landing-skill` stage, not this one).

## Skipped Gates

- `cargo audit`, `cargo deny check`, `cargo vet`, `cargo geiger`, `cargo machete`, `cargo hack check --feature-powerset`, `cargo mutants` — skipped per the bead's narrow evidence scope (acceptance gate is `cargo test -p vb_storage` plus kani-feature compile). The Holzman Rust fallback gate's `--all-targets --all-features` clippy is run; full governance tooling is owned by the `moon ci` canonical gate.
- `moon run :nightly-feature-gate` — deferred to the `landing-skill` stage. The new feature flag is not a perf-only flag; it gates a Kani harness group only. No new nightly features are required.
- `moon ci` — deferred to the `landing-skill` stage. The Holzman Rust lane owns `cargo test -p vb_storage`, which is the contract's acceptance gate.

## Residual Risks

1. **Pre-existing `BLOCK_GLOBAL` failure in parent commit**: the test `proptest_admission_with_budget_has_runtime_capacity_rejection_surface` in `crates/vb_core/tests/aggregate_resource_budget_properties_red.rs` fails in the workdir's parent commit (`1d6c017f`). The fix (`93d1d9026` on main) is not in the parent. This is a `BLOCK_GLOBAL` prerequisite repair item, not introduced by this bead. Documented as a blocker for downstream `landing-skill`. See `.beads/vb-r8oso/evidence/block-global-prerequisite.md`.
2. **Fuzz harness arm updates not in this delivery**: the contract calls for `fuzz/src/journal_target/errors.rs`, `fuzz/fuzz_targets/journal_decode.rs`, `fuzz/fuzz_targets/decode_record.rs`, and `fuzz/tests/proptest_journal_error_exhaustiveness.rs` to receive new `SequenceMismatch` arms. The fuzz harnesses can be updated by the `proof-writer`/`test-writer` stages; the bead's `cargo test` acceptance gate does not require them and the workspace `cargo test` is green.
3. **Cross-crate proptest exhaustiveness not in this delivery**: `crates/workspace_tests/tests/proptest_error_types_registration.rs` and `proptest_error_types_nonzero_codes.rs` should add `SequenceMismatch` arms. Same rationale as the fuzz arms.

These residual risks are owned by downstream stages and do not block the Holzman Rust acceptance gate.

## Reference Compliance

- C-1 (Domain model) — `next_sequence_at_write` uses the canonical `EventSeq` / `RunId` / `StorageKey::RunEvent` types.
- C-2 (new method signature) — exact signature, return semantics, lookup discipline, identifier hygiene, locking, public wrapper, no-panic contract.
- C-3 (new variant) — exact field semantics, diagnostic code 0x4042, symbolic code `JOURNAL_SEQUENCE_MISMATCH_AT_WRITE`, coexistence with `SequenceGap`.
- C-4 (append path contract) — affected methods, guard precedence, doc-comments, batch atomicity.
- C-5 (no silent rewrite) — verified by tests asserting `expected != actual` on `SequenceMismatch`.
- C-6 (existing tests must update) — tests at lines 1737 and 4612 updated; contract-pinned tests widened only for variant arm additions.
- C-7 (new behavior tests) — `append_strict_rejects_sequence_skipped_with_typed_error`, `append_strict_rejects_sequence_at_zero_for_run_with_history`, `append_strict_accepts_first_seq_for_fresh_run`, `next_sequence_at_write_returns_zero_for_fresh_run`, `next_sequence_at_write_returns_last_plus_one_after_writes`, `append_strict_batch_rejects_on_first_mismatch_atomically`, `append_journaled_rejects_out_of_order_sequence_with_sequence_mismatch` all present.
- C-8 (proof/test surface) — Kani harness group behind the new feature; proptest updates; fuzz updates tracked as residual risk.
- C-9 (Kani harness isolation) — feature flag + `#[cfg(all(kani, feature = "kani-sequence-at-write"))]`.
- C-10 (downstream caller audit) — only caller is `StorageRuntimeJournal::append_storage_event` in `vb_runtime/src/journal/chunk_002.rs`; seq originates from `journal_sequence_for` which is always contiguous.
- C-11 (acceptance gate) — `cargo test -p vb_storage --lib` and `--features kani-sequence-at-write` both pass; `cargo test -p vb_storage --test proptest_journal_error_codes` passes.
- C-12 (cross-stage hand-off) — all artifacts under `.beads/vb-r8oso/` plus the new implementation.md and evidence/ directory.
