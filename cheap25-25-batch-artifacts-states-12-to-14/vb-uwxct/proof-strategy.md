# Proof Strategy: vb-uwxct

## Beacon

| Field | Value |
|-------|-------|
| Bead | `vb-uwxct` |
| Title | Tests: make max-sequence/key tests reject only exact overflow (P1 bug) |
| Kind | TEST-ONLY REPAIR |
| State | 4 (Proof Planning) |
| Planner | proof-planner skill |
| Generated | 2026-07-01 |
| Isolated workdir | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-uwxct` |
| jj workspace | `cheap25-vb-uwxct` |

---

## 1. Scope

Proof planning covers **test-only** tightening of seven over-rejecting specimens so that
the production encoder contract — `Err(JournalError::SequenceOverflow) iff
seq.get() == u64::MAX` — is correctly observed by every specimen.

| Specimen | Path | Repair form |
|----------|------|-------------|
| proptest `run_event_key_lexicographic_ordering` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs:1326-1351` | tighten `s1, s2` to `0u64..u64::MAX` |
| proptest `sequence_bytes_roundtrip_through_key_encoding` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs:1355-1369` | tighten `seq_val` to `0u64..u64::MAX` |
| proptest `run_event_key_always_17_bytes` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs:1373-1386` | tighten `seq_val` to `0u64..u64::MAX` |
| proptest `run_event_key_always_has_correct_prefix` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs:1390-1401` | tighten `seq_val` to `0u64..u64::MAX` |
| proptest `different_runs_have_different_event_key_prefixes` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs:1405-1423` | tighten `s1, s2` to `0u64..u64::MAX` |
| proptest `same_run_different_seq_keys_differ_in_seq_bytes` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs:1427-1449` | tighten `s1, s2` to `0u64..u64::MAX` |
| Kani harness `assert_key_contracts` (called by `vb_eepg_typed_partitioned_ids`) | `crates/vb_storage/src/kani_typed_partitioned_ids.rs:43-115` | explicit match `Err(JournalError::SequenceOverflow) => assert!(seq_value == u64::MAX)` |

**Production is NOT touched.** `crates/vb_storage/src/keys.rs:480-496`
(`sequenced_run_key`) and its public delegators (`run_event_key`,
`run_snapshot_key`, `journal_key`) are already contract-correct. The
`JournalError::SequenceOverflow` variant is the canonical typed rejection.

---

## 2. Discovery Findings

| Check | Result |
|-------|--------|
| Production encoder at `keys.rs:480-496` | `if seq.get() == u64::MAX { return Err(JournalError::SequenceOverflow); }` — already contract-correct |
| Sibling tests `keys/tests.rs:469-526` | Already enforce `Err(SequenceOverflow)` only for `EventSeq(u64::MAX)`; reference-positive |
| Canonical proptest pattern `fjall_keyspace_manifest_tests.rs:129,131` | `s1 in 0u64..u64::MAX, s2 in 0u64..u64::MAX` — the exact repair shape |
| Six over-rejecting proptests at `restate_journal_tail_scan_fallback_tests.rs:1326-1449` | Full-range `u64` with `.expect(...)` — panics on `seq == u64::MAX` |
| Kani harness `kani_typed_partitioned_ids.rs:43-115` | `Err(_) => assert!(false)` for `run_event_key` — produces a vacuous Kani counterexample when `seq_value == u64::MAX` |
| Forbidden edits | Production encoder, `JournalError` enum, Verus spec mirror `extern_vb_storage_keys.rs`, `keys/tests.rs`, `proptests.rs`, `chunk_004.rs`, `kani_record_kind.rs` |
| `cargo test`, `cargo kani` | Available; Kani harness group is default-on for `vb_storage` (no extra feature flag) |
| `cargo clippy --all` | Available; lint zero-tolerance on source |

---

## 3. Risk Classification

The production encoder is correct. The test specimens over-reject by
panicking on a contractually-valid `Err`. All three active risks are
**test-suite correctness**, not production correctness:

| Risk | Verifier | Rationale |
|------|----------|-----------|
| Six proptests panic on the sentinel input instead of skipping | cargo-test (proptest) | proptest shrinks to `u64::MAX` and triggers `.expect` panic; canonical repair is `0u64..u64::MAX` |
| Kani harness produces a vacuous counterexample on `seq_value == u64::MAX` | kani (harness-only probe) | `Err(_) => assert!(false)` violates "trust but verify"; explicit match on `JournalError::SequenceOverflow` accepts the typed rejection |
| New forbidden surface (unwrap/expect/panic/[T]::last()/unchecked indexing) in the repaired specimens | source-lint | Holzman Rust rule; zero-tolerance source lint |

There is **no production code change**, so:

| Verifier | Status | Reason |
|----------|--------|--------|
| Verus | `not_applicable` | No production Rust change → no Verus obligation. Adding a Verus proof would be VACUUM (GOD RULE 2). |
| Flux-rs | `not_applicable` | No production Rust refinement target. |
| Loom | `not_applicable` | No concurrency surface introduced; `#[cfg(kani)]`-only harness. |
| Miri | `not_applicable` | No unsafe surface introduced; all touched files retain `#![forbid(unsafe_code)]`. |
| cargo-fuzz | `not_applicable` | No new parser/codec surface introduced. |

---

## 4. Lane Applicability (per seed)

| Proof Seed | cargo-test | kani | source-lint | Verus | Flux | Loom | Miri | fuzz |
|------------|------------|------|-------------|-------|------|------|------|------|
| ps-vb-uwxct-001 (C1 lex-ordering) | required | — | required | — | — | — | — | — |
| ps-vb-uwxct-002 (C2 seq roundtrip) | required | — | required | — | — | — | — | — |
| ps-vb-uwxct-003 (C3 always-17-bytes) | required | — | required | — | — | — | — | — |
| ps-vb-uwxct-004 (C4 correct-prefix) | required | — | required | — | — | — | — | — |
| ps-vb-uwxct-005 (C5 different-runs) | required | — | required | — | — | — | — | — |
| ps-vb-uwxct-006 (C6 same-run-diff-seq) | required | — | required | — | — | — | — | — |
| ps-vb-uwxct-007 (C7 kani harness) | — | required | required | — | — | — | — | — |

`—` means `not_applicable` with concrete evidence (see `verifier-lane-decisions.jsonl`).

---

## 5. Obligation Status

Three required obligations, all behavior-non-affecting (test-only repair).

### 5.1 cargo-test lane (targeted regression)

| ID | Clause | Status | Command |
|----|--------|--------|---------|
| `PO-CARGO-TEST-001` | C1..C6 | `planned` | `cargo test -p workspace_tests --test restate_journal_tail_scan_fallback_tests -- --nocapture` |
| `PO-CARGO-LIB-001` | C0 (canonical-positive reference) | `planned` (reference) | `cargo test -p vb_storage --lib keys::tests::` |

`PO-CARGO-TEST-001` exercises all six tightened proptests after the `0u64..u64::MAX`
range shrink. `PO-CARGO-LIB-001` confirms the sibling tests
`run_event_key_rejects_event_seq_max_sentinel` (lines 497-505) and
`run_event_key_with_zero_seq` (lines 484-489) remain green — the canonical-positive
reference confirming the contract holds.

### 5.2 kani lane (sequence at write)

| ID | Clause | Status | Command |
|----|--------|--------|---------|
| `PO-KANI-001` | C7 | `planned` | `bash scripts/kani-list.sh vb_storage` followed by `cargo kani -p vb_storage --harness vb_eepg_typed_partitioned_ids` |

`PO-KANI-001` exercises the Kani harness `vb_eepg_typed_partitioned_ids` /
`assert_key_contracts`. After the explicit `match
Err(JournalError::SequenceOverflow) => assert!(seq_value == u64::MAX)` arm is added,
the harness accepts the typed rejection when `seq_hi == 0xFFFF && seq_lo == 0xFFFF`
and asserts the documented key layout invariants on the `Ok` case. No
`kani::assume(seq_value != u64::MAX)` is added (forbidden by bead scope).

### 5.3 source-lint lane (Holzman Rust zero-tolerance)

| ID | Clause | Status | Command |
|----|--------|--------|---------|
| `PO-LINT-SRC-001` | C1..C7 | `planned` | `bash scripts/forbidden-scan.sh` + `bash scripts/check-source-length.sh` + `cargo clippy --workspace --all-targets -- -D warnings` |

`PO-LINT-SRC-001` confirms zero new `unwrap()`, `expect()`, `panic!`, `todo!`,
`unimplemented!`, `dbg!`, `assert!(false)`, `[T]::last()`, or unchecked indexing in
the six proptests and the Kani harness. Source-length gate continues to hold (the
repair only narrows proptest ranges — net line change is negative).

### 5.4 Deferred to State 12 (Gauntlet closure)

| ID | Status | Command |
|----|--------|---------|
| `PO-MOON-CI-001` | `deferred` (state 12) | `moon run :verify-fast` or equivalent |

---

## 6. Trusted Base

The trusted base is small because **no production code is changed** and the
repair is purely a test-range shrink plus a typed-error match.

| ID | Marker | Kind | Reason | Compensating Evidence |
|----|--------|------|--------|------------------------|
| TBR-001 | Canonical proptest range `0u64..u64::MAX` from `fjall_keyspace_manifest_tests.rs:129,131` | `external_body` (reference) | The repair reuses an already-accepted proptest strategy; production contract is already verified by `keys/tests.rs:497-505`. | `PO-CARGO-LIB-001`; sibling canonical-positive tests at `crates/vb_storage/src/keys/tests.rs:469-526`. |
| TBR-002 | `JournalError::SequenceOverflow` variant identity | `assume` (named) | The variant is the canonical typed-error contract; production is correct per `keys.rs:485-487`. | `PO-CARGO-LIB-001`; canonical-positive reference at `crates/vb_storage/src/keys/tests.rs:497-505`. |
| TBR-003 | `SymbolicKeyInputs` packed via `(hi<<16) | lo` for `run_raw` / `seq_raw` | `assume` (named) | Kani `kani::Arbitrary` derive generates arbitrary u16 pairs; the packing formula is the documented reconstruction of the u64 value. | Existing Kani probe `kani_list.json` for `vb_storage`; harness compile-and-list pass. |
| TBR-004 | Kani harness calls `keys::run_event_key` directly (production binding) | `external_body` | The harness already calls the production `pub fn run_event_key` symbol at `crates/vb_storage/src/keys.rs:81-83`; no mirror or shadow type is introduced. | `PO-KANI-001` raw Kani PASS; `crates/vb_storage/src/kani_typed_partitioned_ids.rs:63-70` directly invokes production. |

**No `unsafe`, `assume`, `axiom`, `admit`, or `external_body` in executable
proof code.** All four trusted-base entries are modeling/reference debt — not
proof artifacts.

---

## 7. Waiver Candidates

This bead issues **zero behavior-affecting waiver candidates**. Every
behavior-affecting risk is closed by the three planned obligations above.

| ID | Waived lane | Reason | Compensating evidence |
|----|-------------|--------|------------------------|
| WC-001 | Verus | No production Rust change in scope; a Verus proof would be VACUUM (GOD RULE 2). | `PO-CARGO-LIB-001` (canonical-positive unit test); `PO-LINT-SRC-001`. |
| WC-002 | Flux-rs | No production Rust refinement target in scope. | Same as WC-001. |
| WC-003 | Loom | No concurrency surface introduced (test-only shrink + harness typed-error match). | `crates/vb_storage/src/kani_typed_partitioned_ids.rs:1 #![cfg(kani)]` and `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs` (no concurrent primitives). |
| WC-004 | Miri | No unsafe surface introduced; touched files retain `#![forbid(unsafe_code)]`. | `crates/vb_storage/src/kani_typed_partitioned_ids.rs:2 #![forbid(unsafe_code)]`; `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs` is a `proptest!` block with safe Rust only. |
| WC-005 | cargo-fuzz | No new parser/codec surface introduced; production encoder is unchanged. | `keys.rs:480-496` is read-only; existing `fuzz/` targets exercise the encoder path (no change in harness surface). |

All five waivers are **non-behavior-affecting** — they cover verifier lanes
that are absent from the test-only repair surface, not production semantics.

---

## 8. Forbidden Actions (re-stated from contract.md)

The proof plan MUST NOT cause downstream agents to:

1. Modify `crates/vb_storage/src/keys.rs:480-496` (production encoder is already correct).
2. Modify the `JournalError` enum or any `JournalError` variant.
3. Modify `verification/verus/extern_vb_storage_keys.rs` (spec mirror out of scope).
4. Touch the unit tests at `crates/vb_storage/src/keys/tests.rs:469-526`.
5. Touch `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs:123-146` (canonical-positive reference).
6. Touch `crates/vb_runtime/src/journal/tests/chunk_004.rs:964-973` (event validity — separate invariant).
7. Touch `crates/vb_storage/src/proptests.rs` or `crates/vb_storage/src/kani_record_kind.rs`.
8. Add a blanket `kani::assume(seq_value != u64::MAX)` to the Kani harness (would mask the sentinel in proof model).
9. Add new dependencies.
10. Use `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, `assert!(false)`, or `[T]::last()` / unchecked indexing in any specimen.

---

## 9. Execution Order

```
State 4 (this state):
  1. proof-strategy.md            ← THIS ARTIFACT
  2. verifier-lane-matrix.md
  3. verifier-lane-decisions.jsonl
  4. proof-coverage-matrix.md
  5. proof-obligations.planned.jsonl
  6. trusted-base-plan.md
  7. waiver-candidates.jsonl

State 4b (proof-plan-reviewer):
  - Disposition each lane decision; reject if any decision contradicts
    section 8 above; verify the Kani obligation targets production
    `keys::run_event_key` (not a shadow model).

State 5 (proof-writer — but only test edits in this bead):
  - proptest range shrink (6 specimens, 1 line each: `s: u64` → `s in 0u64..u64::MAX`).
  - Kani harness: replace `Err(_) => assert!(false)` with explicit match
    on `JournalError::SequenceOverflow` ⇒ `assert!(seq_value == u64::MAX)`;
    retain `Err(_) => assert!(false)` only for non-SequenceOverflow variants
    (defensive arm).

State 8 (formal-verifier — test execution):
  - PO-CARGO-TEST-001 (cargo-test targeted)
  - PO-CARGO-LIB-001 (cargo lib unit reference)
  - PO-KANI-001 (Kani harness probe)
  - PO-LINT-SRC-001 (forbidden-scan + source-length + clippy)

State 12 (Gauntlet):
  - PO-MOON-CI-001 (deferred)
```

---

## 10. Summary

- **3 required proof obligations** (cargo-test, kani, source-lint) covering
  contract clauses C0..C7.
- **1 deferred obligation** (`PO-MOON-CI-001`) for State 12 closure.
- **5 non-behavior-affecting waivers** (Verus, Flux, Loom, Miri, fuzz) — all
  with concrete evidence refs in `verifier-lane-decisions.jsonl`.
- **0 behavior-affecting waivers** (GOD RULE 2 satisfied — no Verus
  obligation is created because no production code is changed).
- **4 trusted-base entries** (all reference/assume, no `unsafe`/`assume` in
  executable proof code).
- **6 proptests + 1 Kani harness** to repair; production encoder
  `keys.rs:480-496` is reference-only and stays untouched.
- All repair work is test-only; canonical-positive reference pattern
  (`fjall_keyspace_manifest_tests.rs:129,131`) is reused as the repair shape.

The plan is consistent with `contract.md`, `codebase-map.md`,
`delivery-scope.jsonl`, `proof-seeds.jsonl`, and
`traceability-matrix.jsonl`. The Kani harness binding remains implementation-bound
(no shadow model is introduced); the explicit match arm
(`Err(JournalError::SequenceOverflow) => assert!(seq_value == u64::MAX)`)
preserves the typed-error contract from production.