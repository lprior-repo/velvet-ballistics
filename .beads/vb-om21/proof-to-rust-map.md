# Proof-to-Rust Map — vb-om21 State 7

**Schema Version:** proof-to-rust-map/v1
**Bead:** vb-om21
**State:** 7 (proof-to-implementation)
**Bead Classification:** TEST-FIRST (production implementation deferred to State 11)
**Isolated Workdir:** /home/lewis/isolated/femdation-velvet-ballistics/vb-om21
**Input:** proof-obligations.planned.jsonl (52 obligations), proof-review.md (State 6 APPROVED)
**Mapping Status:** planned (all rows; materialized/verified at State 12+)
**Bridge Reference:** proof-to-implementation-input.md

## Trust Boundaries Carried from State 6

| Trust Boundary | Scope | Impact on Bridge |
|---|---|---|
| TB-vb-om21-tla-tooling-gap | 6 TLA+ obligations | No TLC execution evidence; TLA+ artifacts are temporal design evidence, not Rust implementation evidence. Marked as temporal-model-only in bridge. |
| TB-vb-om21-verus-production-binding | 11 Verus obligations | Verus specs are standalone models. Production `exec fn` binding deferred to State 11. Marked as model-boundary in bridge. |
| TB-vb-om21-flux-package-level | 11 Flux obligations | Single-file Flux checks blocked. Only package-level pass. Marked as refinement-syntax-only in bridge. |
| TB-vb-om21-kani-model-abstraction | 11 Kani harnesses | Kani uses `kani_vb_om21_model.rs` (simplified key layout) instead of production `ArrayVec` encoder. Marked as model-bridge in bridge. |
| TB-vb-om21-test-first-bead-scope | All 52 obligations | Production code not yet written. All source refs point to existing production code that will host the new tail scan fallback behavior. |

## Key Production Source Files

| Symbol | File:Line | Description |
|---|---|---|
| `run_event_key` | `crates/vb_storage/src/keys.rs:41` | Encodes `[0x11][run_id_u64_be][seq_u64_be]` key |
| `journal_key` | `crates/vb_storage/src/keys.rs:133` | Public alias for `run_event_key` |
| `sequenced_run_key` | `crates/vb_storage/src/keys.rs:137-150` | Core `[prefix][run_id_u64_be][seq_u64_be]` encoder, 17-byte output |
| `run_prefix_key` | `crates/vb_storage/src/keys.rs:178` | Encodes `[0x11][run_id_u64_be]` 9-byte prefix |
| `events_for_run` | `crates/vb_storage/src/journal/replay.rs:53` | Public replay entry point |
| `events_for_run_bounded` | `crates/vb_storage/src/journal/replay.rs:73` | Bounded replay with snapshot tail start |
| `events_for_run_from` | `crates/vb_storage/src/journal/replay.rs:89` | Core replay loop with prefix-bounded scan (line 106 `starts_with(&run_prefix)`) |
| `validate_replay_sequence` | `crates/vb_storage/src/journal/replay.rs:123` | Sequence gap/WrongRun validation |
| `push_replay_event` | `crates/vb_storage/src/journal/replay.rs:134` | Bounded-replay push with TooManyEvents check |
| `classify_replay_push_len` | `crates/vb_storage/src/journal/replay.rs:30` | O(1) bounded-resource push-limit classification |
| `JournalError::SequenceOverflow` | `crates/vb_storage/src/error/mod.rs:67` | Existing typed overflow error |
| `JournalError::WrongRun` | `crates/vb_storage/src/error/mod.rs:52` | Existing WrongRun typed error |
| `JournalError::SequenceGap` | `crates/vb_storage/src/error/mod.rs:60` | Existing SequenceGap typed error |

## Planned New Production Surface (State 11)

| Planned Type | Location | Description |
|---|---|---|
| `JournalError::TailMismatch` | `crates/vb_storage/src/error/mod.rs` (new variant) | Typed error for stale declared tail metadata |
| `JournalError::MissingJournal` | `crates/vb_storage/src/error/mod.rs` (new variant) | Typed error for absent recovery journal |
| `JournalError::TailOverflow` | `crates/vb_storage/src/error/mod.rs` (new variant) | Typed error for u64::MAX+1 tail |
| `scan_tail_fallback` | `crates/vb_storage/src/journal/replay.rs` (new fn) | Tail scan helper using prefix-bounded iteration |

## Planned Behavior Test Targets

| Test Function | Test File | Description |
|---|---|---|
| `test_tail_scan_prefix_bound` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs` | Prefix-bounded scan never observes other-run keys |
| `test_big_endian_max_seq` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs` | Lexicographic order matches numeric EventSeq order |
| `test_tail_mismatch_rejection` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs` | Stale declared tail → typed TailMismatch |
| `test_missing_journal_recovery` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs` | Recovery-required with no journal → MissingJournal |
| `test_zero_tail_empty_journal` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs` | Empty per-run prefix → tail zero |
| `test_single_event_tail` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs` | Single event seq 0 → tail 1 |
| `test_tail_overflow` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs` | u64::MAX max → typed overflow, no wrap |
| `test_key_parse_no_panic` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs` | Malformed keys → no panic, graceful rejection |
| `test_replay_parity` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs` | Tail fallback preserves WrongRun/SequenceGap validation |
| `test_bounded_scan` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs` | O(1) accumulator, no full-event collection |
| `test_typed_error_distinction` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs` | Match/success, stale/TailMismatch, absent/MissingJournal distinct |

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |

## Bridge Matrix: All 52 Proof Obligations

### 1. Prefix-Bound (PO-vb-om21-prefix-bound-*)

| Proof ID | Claim | BA | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun |
|---|---|---|---|---|---|---|---|---|
| PO-vb-om21-prefix-bound-tla | Tail scan observes only matching run prefix keys | Y | `crates/vb_storage/src/keys.rs::run_event_key:41`, `::run_prefix_key:178`, `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_tail_scan_prefix_bound` | `verification/tla/vb_om21_tail_fallback_prefix_bound.tla` | tla-plus | java -jar tools/tla2tools.jar ... (TLC blocked, trust boundary) | 5 |
| PO-vb-om21-prefix-bound-verus | Tail scan observes only matching run prefix keys | Y | `crates/vb_storage/src/keys.rs::run_event_key:41`, `::run_prefix_key:178`, `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_tail_scan_prefix_bound` | `verification/verus/vb_om21_tail_fallback_prefix_bound.rs` | verus | verus --crate-type=lib ... (model-boundary, State 11 binding required) | 5 |
| PO-vb-om21-prefix-bound-kani | Tail scan observes only matching run prefix keys | Y | `crates/vb_storage/src/keys.rs::run_event_key:41`, `::run_prefix_key:178`, `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_tail_scan_prefix_bound` | `crates/vb_storage/src/kani_vb_om21_prefix_bound.rs` | kani | cargo kani -p vb_storage --harness vb_om21_prefix_bound_harness | 5 |
| PO-vb-om21-prefix-bound-flux | Tail scan observes only matching run prefix keys | Y | `crates/vb_storage/src/keys.rs::run_event_key:41`, `::run_prefix_key:178`, `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_tail_scan_prefix_bound` | `verification/flux/vb_om21_tail_fallback_prefix_bound.rs` | flux-rs | cargo flux -p vb_storage -F flux-proofs (single-file blocked, trust boundary) | 5 |
| PO-vb-om21-prefix-bound-proptest | Tail scan observes only matching run prefix keys | Y | `crates/vb_storage/src/keys.rs::run_event_key:41`, `::run_prefix_key:178`, `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_tail_scan_prefix_bound` | `crates/vb_storage/tests/proptest/vb_om21_prefix_bound_proptest.rs` | proptest | cargo nextest run -p vb_storage vb_om21_prefix_bound_proptest | 5 |

### 2. Big-Endian Max (PO-vb-om21-big-endian-max-*)

| Proof ID | Claim | BA | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun |
|---|---|---|---|---|---|---|---|---|
| PO-vb-om21-big-endian-max-verus | Lexicographic order matches numeric order for seq bytes | Y | `crates/vb_storage/src/keys.rs::sequenced_run_key:137-150`, `::run_event_key:41` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_big_endian_max_seq` | `verification/verus/vb_om21_tail_fallback_big_endian_max.rs` | verus | verus --crate-type=lib ... (model-boundary) | 5 |
| PO-vb-om21-big-endian-max-kani | Lexicographic order matches numeric order for seq bytes | Y | `crates/vb_storage/src/keys.rs::sequenced_run_key:137-150`, `::run_event_key:41` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_big_endian_max_seq` | `crates/vb_storage/src/kani_vb_om21_big_endian_max.rs` | kani | cargo kani -p vb_storage --harness vb_om21_big_endian_max_harness | 5 |
| PO-vb-om21-big-endian-max-flux | Lexicographic order matches numeric order for seq bytes | Y | `crates/vb_storage/src/keys.rs::sequenced_run_key:137-150`, `::run_event_key:41` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_big_endian_max_seq` | `verification/flux/vb_om21_tail_fallback_big_endian_max.rs` | flux-rs | cargo flux -p vb_storage -F flux-proofs (single-file blocked) | 5 |
| PO-vb-om21-big-endian-max-proptest | Lexicographic order matches numeric order for seq bytes | Y | `crates/vb_storage/src/keys.rs::sequenced_run_key:137-150`, `::run_event_key:41` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_big_endian_max_seq` | `crates/vb_storage/tests/proptest/vb_om21_big_endian_max_proptest.rs` | proptest | cargo nextest run -p vb_storage vb_om21_big_endian_max_proptest | 5 |

### 3. Tail Mismatch (PO-vb-om21-tail-mismatch-*)

| Proof ID | Claim | BA | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun |
|---|---|---|---|---|---|---|---|---|
| PO-vb-om21-tail-mismatch-tla | Stale declared tail → typed TailMismatch | Y | `crates/vb_storage/src/error/mod.rs::JournalError` (planned TailMismatch variant), `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_tail_mismatch_rejection` | `verification/tla/vb_om21_tail_fallback_tail_mismatch.tla` | tla-plus | TLC blocked (trust boundary) | 5 |
| PO-vb-om21-tail-mismatch-verus | Stale declared tail → typed TailMismatch | Y | `crates/vb_storage/src/error/mod.rs::JournalError` (planned), `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_tail_mismatch_rejection` | `verification/verus/vb_om21_tail_fallback_tail_mismatch.rs` | verus | verus --crate-type=lib ... (model-boundary) | 5 |
| PO-vb-om21-tail-mismatch-kani | Stale declared tail → typed TailMismatch | Y | `crates/vb_storage/src/error/mod.rs::JournalError` (planned), `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_tail_mismatch_rejection` | `crates/vb_storage/src/kani_vb_om21_tail_mismatch.rs` | kani | cargo kani -p vb_storage --harness vb_om21_tail_mismatch_harness | 5 |
| PO-vb-om21-tail-mismatch-flux | Stale declared tail → typed TailMismatch | Y | `crates/vb_storage/src/error/mod.rs::JournalError` (planned), `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_tail_mismatch_rejection` | `verification/flux/vb_om21_tail_fallback_tail_mismatch.rs` | flux-rs | cargo flux -p vb_storage -F flux-proofs (single-file blocked) | 5 |
| PO-vb-om21-tail-mismatch-proptest | Stale declared tail → typed TailMismatch | Y | `crates/vb_storage/src/error/mod.rs::JournalError` (planned), `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_tail_mismatch_rejection` | `crates/vb_storage/tests/proptest/vb_om21_tail_mismatch_proptest.rs` | proptest | cargo nextest run -p vb_storage vb_om21_tail_mismatch_proptest | 5 |

### 4. Missing Journal (PO-vb-om21-missing-journal-*)

| Proof ID | Claim | BA | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun |
|---|---|---|---|---|---|---|---|---|
| PO-vb-om21-missing-journal-tla | Recovery-required with no journal → MissingJournal | Y | `crates/vb_storage/src/error/mod.rs::JournalError` (planned MissingJournal variant), `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_missing_journal_recovery` | `verification/tla/vb_om21_tail_fallback_missing_journal.tla` | tla-plus | TLC blocked (trust boundary) | 5 |
| PO-vb-om21-missing-journal-verus | Recovery-required with no journal → MissingJournal | Y | `crates/vb_storage/src/error/mod.rs::JournalError` (planned), `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_missing_journal_recovery` | `verification/verus/vb_om21_tail_fallback_missing_journal.rs` | verus | verus --crate-type=lib ... (model-boundary) | 5 |
| PO-vb-om21-missing-journal-kani | Recovery-required with no journal → MissingJournal | Y | `crates/vb_storage/src/error/mod.rs::JournalError` (planned), `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_missing_journal_recovery` | `crates/vb_storage/src/kani_vb_om21_missing_journal.rs` | kani | cargo kani -p vb_storage --harness vb_om21_missing_journal_harness | 5 |
| PO-vb-om21-missing-journal-flux | Recovery-required with no journal → MissingJournal | Y | `crates/vb_storage/src/error/mod.rs::JournalError` (planned), `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_missing_journal_recovery` | `verification/flux/vb_om21_tail_fallback_missing_journal.rs` | flux-rs | cargo flux -p vb_storage -F flux-proofs (single-file blocked) | 5 |
| PO-vb-om21-missing-journal-proptest | Recovery-required with no journal → MissingJournal | Y | `crates/vb_storage/src/error/mod.rs::JournalError` (planned), `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_missing_journal_recovery` | `crates/vb_storage/tests/proptest/vb_om21_missing_journal_proptest.rs` | proptest | cargo nextest run -p vb_storage vb_om21_missing_journal_proptest | 5 |

### 5. Zero Tail Query (PO-vb-om21-zero-tail-query-*)

| Proof ID | Claim | BA | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun |
|---|---|---|---|---|---|---|---|---|
| PO-vb-om21-zero-tail-query-tla | Empty per-run prefix → tail zero | Y | `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-101`, `crates/vb_storage/src/keys.rs::run_prefix_key:178` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_zero_tail_empty_journal` | `verification/tla/vb_om21_tail_fallback_zero_tail_query.tla` | tla-plus | TLC blocked (trust boundary) | 5 |
| PO-vb-om21-zero-tail-query-verus | Empty per-run prefix → tail zero | Y | `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-101`, `crates/vb_storage/src/keys.rs::run_prefix_key:178` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_zero_tail_empty_journal` | `verification/verus/vb_om21_tail_fallback_zero_tail_query.rs` | verus | verus --crate-type=lib ... (model-boundary) | 5 |
| PO-vb-om21-zero-tail-query-kani | Empty per-run prefix → tail zero | Y | `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-101`, `crates/vb_storage/src/keys.rs::run_prefix_key:178` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_zero_tail_empty_journal` | `crates/vb_storage/src/kani_vb_om21_zero_tail_query.rs` | kani | cargo kani -p vb_storage --harness vb_om21_zero_tail_query_harness | 5 |
| PO-vb-om21-zero-tail-query-flux | Empty per-run prefix → tail zero | Y | `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-101`, `crates/vb_storage/src/keys.rs::run_prefix_key:178` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_zero_tail_empty_journal` | `verification/flux/vb_om21_tail_fallback_zero_tail_query.rs` | flux-rs | cargo flux -p vb_storage -F flux-proofs (single-file blocked) | 5 |
| PO-vb-om21-zero-tail-query-proptest | Empty per-run prefix → tail zero | Y | `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-101`, `crates/vb_storage/src/keys.rs::run_prefix_key:178` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_zero_tail_empty_journal` | `crates/vb_storage/tests/proptest/vb_om21_zero_tail_query_proptest.rs` | proptest | cargo nextest run -p vb_storage vb_om21_zero_tail_query_proptest | 5 |

### 6. Single Event Tail (PO-vb-om21-single-event-tail-*)

| Proof ID | Claim | BA | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun |
|---|---|---|---|---|---|---|---|---|
| PO-vb-om21-single-event-tail-verus | Single event seq 0 → tail 1 | Y | `crates/vb_storage/src/keys.rs::run_event_key:41`, `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_single_event_tail` | `verification/verus/vb_om21_tail_fallback_single_event_tail.rs` | verus | verus --crate-type=lib ... (model-boundary) | 5 |
| PO-vb-om21-single-event-tail-kani | Single event seq 0 → tail 1 | Y | `crates/vb_storage/src/keys.rs::run_event_key:41`, `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_single_event_tail` | `crates/vb_storage/src/kani_vb_om21_single_event_tail.rs` | kani | cargo kani -p vb_storage --harness vb_om21_single_event_tail_harness | 5 |
| PO-vb-om21-single-event-tail-flux | Single event seq 0 → tail 1 | Y | `crates/vb_storage/src/keys.rs::run_event_key:41`, `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_single_event_tail` | `verification/flux/vb_om21_tail_fallback_single_event_tail.rs` | flux-rs | cargo flux -p vb_storage -F flux-proofs (single-file blocked) | 5 |
| PO-vb-om21-single-event-tail-proptest | Single event seq 0 → tail 1 | Y | `crates/vb_storage/src/keys.rs::run_event_key:41`, `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_single_event_tail` | `crates/vb_storage/tests/proptest/vb_om21_single_event_tail_proptest.rs` | proptest | cargo nextest run -p vb_storage vb_om21_single_event_tail_proptest | 5 |

### 7. Tail Overflow (PO-vb-om21-tail-overflow-*)

| Proof ID | Claim | BA | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun |
|---|---|---|---|---|---|---|---|---|
| PO-vb-om21-tail-overflow-verus | u64::MAX max → typed overflow, no wrap | Y | `crates/vb_storage/src/error/mod.rs::JournalError::SequenceOverflow:67`, `crates/vb_storage/src/journal/replay.rs::validate_replay_sequence:123-131` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_tail_overflow` | `verification/verus/vb_om21_tail_fallback_tail_overflow.rs` | verus | verus --crate-type=lib ... (model-boundary) | 5 |
| PO-vb-om21-tail-overflow-kani | u64::MAX max → typed overflow, no wrap | Y | `crates/vb_storage/src/error/mod.rs::JournalError::SequenceOverflow:67`, `crates/vb_storage/src/journal/replay.rs::validate_replay_sequence:123-131` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_tail_overflow` | `crates/vb_storage/src/kani_vb_om21_tail_overflow.rs` | kani | cargo kani -p vb_storage --harness vb_om21_tail_overflow_harness | 5 |
| PO-vb-om21-tail-overflow-flux | u64::MAX max → typed overflow, no wrap | Y | `crates/vb_storage/src/error/mod.rs::JournalError::SequenceOverflow:67`, `crates/vb_storage/src/journal/replay.rs::validate_replay_sequence:123-131` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_tail_overflow` | `verification/flux/vb_om21_tail_fallback_tail_overflow.rs` | flux-rs | cargo flux -p vb_storage -F flux-proofs (single-file blocked) | 5 |
| PO-vb-om21-tail-overflow-proptest | u64::MAX max → typed overflow, no wrap | Y | `crates/vb_storage/src/error/mod.rs::JournalError::SequenceOverflow:67`, `crates/vb_storage/src/journal/replay.rs::validate_replay_sequence:123-131` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_tail_overflow` | `crates/vb_storage/tests/proptest/vb_om21_tail_overflow_proptest.rs` | proptest | cargo nextest run -p vb_storage vb_om21_tail_overflow_proptest | 5 |

### 8. Key Parse (PO-vb-om21-key-parse-*)

| Proof ID | Claim | BA | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun |
|---|---|---|---|---|---|---|---|---|
| PO-vb-om21-key-parse-verus | Length/prefix validation before sequence extraction, no panic | Y | `crates/vb_storage/src/keys.rs::sequenced_run_key:137-150`, `::run_event_key:41`, `::run_prefix_key:178` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_key_parse_no_panic` | `verification/verus/vb_om21_tail_fallback_key_parse.rs` | verus | verus --crate-type=lib ... (model-boundary) | 5 |
| PO-vb-om21-key-parse-kani | Length/prefix validation before sequence extraction, no panic | Y | `crates/vb_storage/src/keys.rs::sequenced_run_key:137-150`, `::run_event_key:41`, `::run_prefix_key:178` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_key_parse_no_panic` | `crates/vb_storage/src/kani_vb_om21_key_parse.rs` | kani | cargo kani -p vb_storage --harness vb_om21_key_parse_harness | 5 |
| PO-vb-om21-key-parse-flux | Length/prefix validation before sequence extraction, no panic | Y | `crates/vb_storage/src/keys.rs::sequenced_run_key:137-150`, `::run_event_key:41`, `::run_prefix_key:178` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_key_parse_no_panic` | `verification/flux/vb_om21_tail_fallback_key_parse.rs` | flux-rs | cargo flux -p vb_storage -F flux-proofs (single-file blocked) | 5 |
| PO-vb-om21-key-parse-miri | Length/prefix validation before sequence extraction, no panic | Y | `crates/vb_storage/src/keys.rs::sequenced_run_key:137-150`, `::run_event_key:41`, `::run_prefix_key:178` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_key_parse_no_panic` | `crates/vb_storage/tests/miri/vb_om21_key_parse_miri.rs` | miri | cargo +nightly miri test -p vb_storage vb_om21_key_parse_miri | 5 |
| PO-vb-om21-key-parse-proptest | Length/prefix validation before sequence extraction, no panic | Y | `crates/vb_storage/src/keys.rs::sequenced_run_key:137-150`, `::run_event_key:41`, `::run_prefix_key:178` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_key_parse_no_panic` | `crates/vb_storage/tests/proptest/vb_om21_key_parse_proptest.rs` | proptest | cargo nextest run -p vb_storage vb_om21_key_parse_proptest | 5 |
| PO-vb-om21-key-parse-fuzz | Length/prefix validation before sequence extraction, no panic | Y | `crates/vb_storage/src/keys.rs::sequenced_run_key:137-150`, `::run_event_key:41`, `::run_prefix_key:178` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_key_parse_no_panic` | `fuzz/fuzz_targets/vb_om21_key_parse_key_parser.rs` | cargo-fuzz | cargo +nightly fuzz run vb_om21_key_parse_key_parser --target x86_64-unknown-linux-gnu -- -runs=100000 | 5 |

### 9. Replay Parity (PO-vb-om21-replay-parity-*)

| Proof ID | Claim | BA | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun |
|---|---|---|---|---|---|---|---|---|
| PO-vb-om21-replay-parity-tla | Tail fallback preserves WrongRun/SequenceGap validation | Y | `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120`, `::validate_replay_sequence:123-131`, `crates/vb_storage/src/error/mod.rs::JournalError::WrongRun:52`, `::SequenceGap:60` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_replay_parity` | `verification/tla/vb_om21_tail_fallback_replay_parity.tla` | tla-plus | TLC blocked (trust boundary) | 5 |
| PO-vb-om21-replay-parity-verus | Tail fallback preserves WrongRun/SequenceGap validation | Y | `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120`, `::validate_replay_sequence:123-131`, `crates/vb_storage/src/error/mod.rs::JournalError::WrongRun:52`, `::SequenceGap:60` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_replay_parity` | `verification/verus/vb_om21_tail_fallback_replay_parity.rs` | verus | verus --crate-type=lib ... (model-boundary) | 5 |
| PO-vb-om21-replay-parity-kani | Tail fallback preserves WrongRun/SequenceGap validation | Y | `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120`, `::validate_replay_sequence:123-131`, `crates/vb_storage/src/error/mod.rs::JournalError::WrongRun:52`, `::SequenceGap:60` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_replay_parity` | `crates/vb_storage/src/kani_vb_om21_replay_parity.rs` | kani | cargo kani -p vb_storage --harness vb_om21_replay_parity_harness | 5 |
| PO-vb-om21-replay-parity-flux | Tail fallback preserves WrongRun/SequenceGap validation | Y | `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120`, `::validate_replay_sequence:123-131`, `crates/vb_storage/src/error/mod.rs::JournalError::WrongRun:52`, `::SequenceGap:60` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_replay_parity` | `verification/flux/vb_om21_tail_fallback_replay_parity.rs` | flux-rs | cargo flux -p vb_storage -F flux-proofs (single-file blocked) | 5 |
| PO-vb-om21-replay-parity-proptest | Tail fallback preserves WrongRun/SequenceGap validation | Y | `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120`, `::validate_replay_sequence:123-131`, `crates/vb_storage/src/error/mod.rs::JournalError::WrongRun:52`, `::SequenceGap:60` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_replay_parity` | `crates/vb_storage/tests/proptest/vb_om21_replay_parity_proptest.rs` | proptest | cargo nextest run -p vb_storage vb_om21_replay_parity_proptest | 5 |

### 10. Bounded Scan (PO-vb-om21-bounded-scan-*)

| Proof ID | Claim | BA | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun |
|---|---|---|---|---|---|---|---|---|
| PO-vb-om21-bounded-scan-verus | O(1) accumulator, no full-event collection for tail query | Y | `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120` (prefix check at line 106, Vec::new at line 96), `::classify_replay_push_len:30-49` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_bounded_scan` | `verification/verus/vb_om21_tail_fallback_bounded_scan.rs` | verus | verus --crate-type=lib ... (model-boundary) | 5 |
| PO-vb-om21-bounded-scan-kani | O(1) accumulator, no full-event collection for tail query | Y | `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120`, `::classify_replay_push_len:30-49` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_bounded_scan` | `crates/vb_storage/src/kani_vb_om21_bounded_scan.rs` | kani | cargo kani -p vb_storage --harness vb_om21_bounded_scan_harness | 5 |
| PO-vb-om21-bounded-scan-flux | O(1) accumulator, no full-event collection for tail query | Y | `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120`, `::classify_replay_push_len:30-49` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_bounded_scan` | `verification/flux/vb_om21_tail_fallback_bounded_scan.rs` | flux-rs | cargo flux -p vb_storage -F flux-proofs (single-file blocked) | 5 |
| PO-vb-om21-bounded-scan-proptest | O(1) accumulator, no full-event collection for tail query | Y | `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120`, `::classify_replay_push_len:30-49` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_bounded_scan` | `crates/vb_storage/tests/proptest/vb_om21_bounded_scan_proptest.rs` | proptest | cargo nextest run -p vb_storage vb_om21_bounded_scan_proptest | 5 |

### 11. Typed Errors (PO-vb-om21-typed-errors-*)

| Proof ID | Claim | BA | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun |
|---|---|---|---|---|---|---|---|---|
| PO-vb-om21-typed-errors-tla | Match=success, stale=TailMismatch, absent=MissingJournal, all distinct | Y | `crates/vb_storage/src/error/mod.rs::JournalError` (planned TailMismatch/MissingJournal/TailOverflow variants), `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_typed_error_distinction` | `verification/tla/vb_om21_tail_fallback_typed_errors.tla` | tla-plus | TLC blocked (trust boundary) | 5 |
| PO-vb-om21-typed-errors-verus | Match=success, stale=TailMismatch, absent=MissingJournal, all distinct | Y | `crates/vb_storage/src/error/mod.rs::JournalError` (planned), `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_typed_error_distinction` | `verification/verus/vb_om21_tail_fallback_typed_errors.rs` | verus | verus --crate-type=lib ... (model-boundary) | 5 |
| PO-vb-om21-typed-errors-kani | Match=success, stale=TailMismatch, absent=MissingJournal, all distinct | Y | `crates/vb_storage/src/error/mod.rs::JournalError` (planned), `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_typed_error_distinction` | `crates/vb_storage/src/kani_vb_om21_typed_errors.rs` | kani | cargo kani -p vb_storage --harness vb_om21_typed_errors_harness | 5 |
| PO-vb-om21-typed-errors-flux | Match=success, stale=TailMismatch, absent=MissingJournal, all distinct | Y | `crates/vb_storage/src/error/mod.rs::JournalError` (planned), `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_typed_error_distinction` | `verification/flux/vb_om21_tail_fallback_typed_errors.rs` | flux-rs | cargo flux -p vb_storage -F flux-proofs (single-file blocked) | 5 |
| PO-vb-om21-typed-errors-proptest | Match=success, stale=TailMismatch, absent=MissingJournal, all distinct | Y | `crates/vb_storage/src/error/mod.rs::JournalError` (planned), `crates/vb_storage/src/journal/replay.rs::events_for_run_from:89-120` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs::test_typed_error_distinction` | `crates/vb_storage/tests/proptest/vb_om21_typed_errors_proptest.rs` | proptest | cargo nextest run -p vb_storage vb_om21_typed_errors_proptest | 5 |

## Unresolved Mapping Gaps

1. **TLA+ → Rust state/event mapping (6 obligations):** TLA+ models are temporal design evidence, not Rust implementation evidence. The 6 TLA+ obligations (prefix-bound, tail-mismatch, missing-journal, zero-tail-query, replay-parity, typed-errors) are accepted as trust boundaries per State 6 review. At State 12+, every TLA+ invariant must map to a concrete Rust state transition, not just a file ref. Currently blocked by `tools/tla2tools.jar` absence.

2. **Verus → production exec fn binding (11 obligations):** All 11 Verus obligations have standalone model artifacts that pass `verus --crate-type=lib` verification. The GOD RULE "No Vacuum Verus Proofs" requires `requires`/`ensures` on actual production `exec fn`. This binding is deferred to State 11. Current standalone-pass evidence establishes well-formed mathematical models only.

3. **Flux → single-file refinement verification (11 obligations):** Installed `cargo-flux` does not accept `--lib` for single-file targeting. Package-level `cargo flux -p vb_storage -F flux-proofs` confirms syntax acceptance but does not prove per-obligation refinement. Resolution at State 11 requires single-file checks or approved waiver.

4. **Kani → production encoder bridging (11 obligations):** All 11 Kani harnesses use `kani_vb_om21_model.rs` (simplified key layout with `[u8; 17]` fixed arrays) instead of production `ArrayVec` encoder. The model mirrors the exact byte layout and domain semantics. At State 11, production types must either become Kani-compatible or the model must be proven equivalent via additional harnesses.

5. **Planned error variants (TailMismatch, MissingJournal, TailOverflow):** These error variants do not yet exist in `crates/vb_storage/src/error/mod.rs`. They are planned additions for State 11. The bridge maps them to the existing `JournalError` enum as the planned insertion point.

6. **Test-first deferral (all 52 obligations):** No behavior test is yet implemented. All `behavior_test_refs` point to planned test function names in `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs`. All `refinement_harness_refs` point to existing verifier artifacts from State 5. All `mapping_status` is `planned` per State 7 rules.

## Summary

| Metric | Value |
|---|---|
| Total proof obligations | 52 |
| Behavior-affecting | 52 |
| Unique Rust source symbols | 13 (keys.rs: 5, replay.rs: 5, error/mod.rs: 3) |
| Planned behavior test functions | 11 |
| Existing refinement harnesses | 52 |
| TLA+ obligations (trust boundary) | 6 |
| Verus obligations (trust boundary) | 11 |
| Flux obligations (trust boundary) | 11 |
| Kani obligations (model bridge) | 11 |
| Proptest obligations | 11 |
| Miri obligations | 1 |
| Fuzz obligations | 1 |
| Mapping status | all `planned` (State 7) |
| State 12 closure gate | `materialized` or `verified` required |
