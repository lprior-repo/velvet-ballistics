# Codebase Map — vb-uwxct

- bead_id: vb-uwxct
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-uwxct
- jj workspace: cheap25-vb-uwxct
- exploration_timestamp: 2026-07-01T15:23:00Z
- contract_target: `vb_storage::keys::run_event_key` (and the private `sequenced_run_key` it delegates to) returning `Err(JournalError::SequenceOverflow)` iff `seq.get() == u64::MAX`.

## Scope Audit Summary

Reviewed all `crates/workspace_tests/tests/*.rs` proptests/unit tests that exercise `run_event_key`, `run_event_key` ordering tests, max-sequence selection tests, the related event-level `is_valid()` checks, and the Kani harness `vb_eepg_typed_partitioned_ids`. Out of every test that touches the `(run, seq) -> key` boundary, the **only over-rejecting specimens** are:

1. The `proptest!` block at the tail of `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs:1305-1450` — six proptest functions take `seq_val: u64` (full range) and call `run_event_key(...).expect("...must encode")`. When proptest samples `seq_val == u64::MAX`, the encoder correctly returns `Err(JournalError::SequenceOverflow)`, but the test panics instead of treating that case per the contract.
2. The Kani harness `crates/vb_storage/src/kani_typed_partitioned_ids.rs:43-93` — `assert_key_contracts` matches on `Err(_) => assert!(false)` for `run_event_key`. When `SymbolicKeyInputs.seq_hi == 0xFFFF && seq_lo == 0xFFFF` (i.e. `seq_value == u64::MAX`), the harness fails instead of accepting the documented `SequenceOverflow` rejection.

All other max-sequence / key tests already enforce the contract precisely (`Err(JournalError::SequenceOverflow)` only when `seq == u64::MAX`, success for every other value).

## Production Contract

- `crates/vb_storage/src/keys.rs:480-496` — `fn sequenced_run_key(prefix, run, seq)`:
  - `if seq.get() == u64::MAX { return Err(JournalError::SequenceOverflow); }` — exact-overflow rejection.
  - All other `seq.get() ∈ 0..u64::MAX` MUST succeed (pushes prefix + run + seq big-endian into `ArrayVec<JOURNAL_KEY_BYTES>`).
- `crates/vb_storage/src/keys.rs:81-83` — `pub fn run_event_key(run, seq) -> Result<[u8; 17], JournalError>` delegates to `sequenced_run_key`.
- `crates/vb_storage/src/keys.rs:85-91` — `pub fn run_snapshot_key(run, seq)` shares the same path.

`JournalError::SequenceOverflow` is defined at `crates/vb_storage/src/error/mod.rs:53-54` (referenced via mirrored comments in `verification/verus/production_inner/vb_vzcuf_PS_001_production.rs:144` and `…_002_production.rs:179`).

## Files Mapped

### Primary File (over-rejecting proptests)

`crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs` (1450 lines)

| Function / span | Lines | Status | Notes |
|---|---|---|---|
| `run_event_key_ordering_matches_numeric_comparison` | 233-253 | OK | const 0/255/MAX-1 samples |
| `sequence_bytes_decoded_to_correct_u64_values` | 256-284 | OK | asserts `Err(SequenceOverflow)` only for `EventSeq(u64::MAX)` |
| `max_sequence_selection_returns_largest_value` | 287-308 | OK | success-only roundtrip on valid seqs |
| `big_endian_byte_ordering_preserves_numeric_ordering_for_all_u64_pairs` | 311+ | OK | pure byte slice ordering |
| `single_event_at_max_minus_one_replays_correctly` | 663-693 | OK | `Err(_)` catch-all is permissive but the seq is `u64::MAX - 1` — contract-safe |
| `max_sequence_key_is_rejected_without_panic` | 700-723 | OK | asserts `Err(SequenceOverflow)` only for `EventSeq(u64::MAX)`; valid `MAX - 1` roundtrip via `.expect` (correct: `MAX - 1` is non-sentinel) |
| `sequence_overflow_detected_when_checked_add_would_wrap` | 726-739 | OK | pure arithmetic |
| `sequence_below_max_does_not_overflow` | 742-763 | OK | pure arithmetic |
| `max_seq_plus_one_does_not_wrap_to_zero` | 766+ | OK | pure arithmetic |
| `run_event_key_construction_with_various_sequences_does_not_panic` | 783-814 | OK | match-arm `Err(_e) => { /* typed failure acceptable */ }` is permissive, not over-rejecting — encoder only errs on `u64::MAX` and that is exactly what the contract says |
| `run_event_key_has_correct_byte_length_for_all_boundary_sequences` | 817-845 | OK | success for 0/1/MAX-1; `Err(SequenceOverflow)` only for `u64::MAX` |
| `build_run_prefix_has_correct_format` | 848-857 | OK | unrelated to seq |
| `prefix_extraction_from_full_key_matches_manual_prefix` | 860-875 | OK | unrelated to seq |
| `prefix_check_correctly_rejects_wrong_prefix` | 878-893 | OK | unrelated to seq |
| `sequence_bytes_at_offset_9_to_17_are_correct_for_all_boundary_values` | 896-923 | OK | handpicked valid values |
| `sequence_overflow_must_be_distinct_from_sequence_gap` | 1195-1215 | OK | variant identity check |
| `big_endian_bytes_preserve_ordering` (proptest) | 1309-1320 | OK | pure byte slice ordering, no encoder |
| **`run_event_key_lexicographic_ordering`** (proptest) | 1326-1351 | **OVER-REJECTING** | `s1: u64, s2: u64` with `prop_assume!(r1 != 0 && r2 != 0)`; `run_event_key(...).expect("key…must encode")` panics on `s1 == u64::MAX` or `s2 == u64::MAX` |
| **`sequence_bytes_roundtrip_through_key_encoding`** (proptest) | 1355-1369 | **OVER-REJECTING** | `seq_val: u64` with `prop_assume!(run_val != 0)`; `run_event_key(...).expect("key must encode for any valid run/seq")` panics on `seq_val == u64::MAX` |
| **`run_event_key_always_17_bytes`** (proptest) | 1373-1386 | **OVER-REJECTING** | `seq_val: u64` with `prop_assume!(run_val != 0)`; `.expect("key must encode for valid inputs")` panics on `seq_val == u64::MAX` |
| **`run_event_key_always_has_correct_prefix`** (proptest) | 1390-1401 | **OVER-REJECTING** | `seq_val: u64` with `prop_assume!(run_val != 0)`; `.expect("key must encode for valid inputs")` panics on `seq_val == u64::MAX` |
| **`different_runs_have_different_event_key_prefixes`** (proptest) | 1405-1423 | **OVER-REJECTING** | `s1: u64, s2: u64` with `prop_assume!(r1 != 0 && r2 != 0 && r1 != r2)`; both `.expect("key…must encode")` panic when `s1 == u64::MAX` or `s2 == u64::MAX` |
| **`same_run_different_seq_keys_differ_in_seq_bytes`** (proptest) | 1427-1449 | **OVER-REJECTING** | `s1: u64, s2: u64` with `prop_assume!(run_val != 0 && s1 != s2)`; both `.expect` panic when `s1 == u64::MAX` or `s2 == u64::MAX` |

### Kani harness (over-rejecting)

`crates/vb_storage/src/kani_typed_partitioned_ids.rs` (129 lines)

| Function | Lines | Status | Notes |
|---|---|---|---|
| `vb_eepg_typed_partitioned_ids` | 111-115 | **OVER-REJECTING** | calls `assert_key_contracts(inputs)` with arbitrary `SymbolicKeyInputs`; when `seq_hi == 0xFFFF && seq_lo == 0xFFFF` ⇒ `seq_value == u64::MAX` ⇒ `run_event_key` returns `Err(_)`, and `Err(_) => assert!(false)` triggers a vacuous-failure Kani counterexample |
| `assert_key_contracts` | 43-93 | **OVER-REJECTING body** | lines 63-70 `match keys::run_event_key(run, seq) { Ok(key) => {...}, Err(_) => assert!(false) }` — must accept `Err(JournalError::SequenceOverflow)` for `seq_value == u64::MAX` |
| `vb_eepg_record_kind_contracts` | 117-121 | OK | unrelated to seq |
| `vb_eepg_unknown_record_kind_error_contract` | 123-128 | OK | unrelated to seq |

### Sibling tests already correctly tightened (no scope)

These already enforce the contract; the implementation agent must NOT touch them:

- `crates/vb_storage/src/keys/tests.rs`
  - `run_event_key_rejects_event_seq_max_sentinel` (lines 497-505) — `matches!(result, Err(JournalError::SequenceOverflow))` only for `EventSeq(u64::MAX)`. The doc-comment at lines 491-496 already states the contract explicitly.
  - `run_event_key_with_zero_seq` (lines 484-489) — success path for `EventSeq(0)`.
  - All boundary tests at lines 469-526 are already contract-exact.
- `crates/vb_storage/src/codec/tests.rs`
  - `encode_decode_with_max_sequence` (lines 1165-1190) — uses `EventSeq(u64::MAX - 1)` (valid); correct.
  - `encode_decode_header_with_max_sequence_roundtrip` (lines 2636-2649) — uses `RecordKind::Snapshot` (different contract surface, no seq rejection).
- `crates/vb_runtime/src/journal/tests/chunk_004.rs`
  - `journal_event_is_valid_rejects_max_sequence` (lines 964-973) — tests `JournalEvent::is_valid()` on `EventSeq::MAX`; correct (boundary checks the event invariant, not the encoder).
- `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs`
  - `run_event_ordering` proptest (lines 123-146) — already uses `s1 in 0u64..u64::MAX, s2 in 0u64..u64::MAX`; correct contract-exact.
  - `max_sequence_ordering` unit (lines 152-173) — asserts `Err(SequenceOverflow)` only for `EventSeq(u64::MAX)`; correct.
- `crates/vb_storage/src/proptests.rs`
  - `run_event_key_ordering_is_monotonic` (lines 27-40) — uses `0u64..1000u64`; correct.

## Risk Tags

- `parser/codec` — binary key encoding contract (17-byte fixed layout).
- `public_api` — `run_event_key`, `run_snapshot_key`, `sequenced_run_key` are all on the vb_storage public surface.
- `persistence` — keys flow into Fjall LSM tree; over-rejecting tests can block the safe boundary and obscure the typed-error contract.
- `temporal` — sequence ordering and overflow semantics drive tail reconstruction / replay correctness (REQ-vb-om21-08).

## Dependency / Contract Cluster

| Symbol | Path | Owner crate |
|---|---|---|
| `sequenced_run_key` (private) | `crates/vb_storage/src/keys.rs:480-496` | `vb_storage` |
| `pub fn run_event_key` | `crates/vb_storage/src/keys.rs:81-83` | `vb_storage` |
| `pub fn run_snapshot_key` | `crates/vb_storage/src/keys.rs:86-91` | `vb_storage` |
| `JournalError::SequenceOverflow` variant | `crates/vb_storage/src/error/mod.rs:53-54` | `vb_storage` |
| Symbolic harness `SymbolicKeyInputs` | `crates/vb_storage/src/kani_typed_partitioned_ids.rs:15-24` | `vb_storage` |
| `proptest!` block on `run_event_key` (over-rejecting 6 fns) | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs:1305-1450` | `workspace_tests` |

## Open Questions

- Does the implementation agent prefer (a) tightening proptest input ranges to `0u64..u64::MAX` (canonical pattern used by `fjall_keyspace_manifest_tests.rs:129,131`), or (b) re-binding the assertion to a `match` arm that distinguishes the exact `Err(SequenceOverflow)` case? The Kani harness must use option (b) because `kani::any()` cannot constrain a `u32`-derived packed bitfield across the full u64 range cheaply.
- Are the proptest failures benign (proptest will skip via `prop_assume!` after the tightening and produce no panics) or do they actively regress today? Without running tests, this is UNKNOWN; downstream test-planner should run `cargo test -p workspace_tests --test restate_journal_tail_scan_fallback_tests` first to capture the raw failure.

## Recommended Downstream Owners

- **rust-contract** — confirm that `(run, seq) -> key` contract is precisely "Err only on seq==MAX" and document the asymmetry between `fn sequenced_run_key` (production) and the proptest assertion shape.
- **proof-plan-reviewer / proof-writer** — possibly extend `verification/verus/extern_vb_storage_keys.rs` to mirror the relaxed Kani harness posture (accept `Err(SpecKeyEncodeError::SequenceOverflow)` on `seq_value == u64::MAX`).
- **test-planner** — plan one regression test per over-rejecting proptest: e.g. `run_event_key_lexicographic_ordering_handles_max_sentinel`, asserting that when `s1 == u64::MAX`, the property is satisfied because the encoder correctly rejects with `SequenceOverflow`.
- **holzman-rust (implementation)** — tighten the six proptests in the target file (replace `s1: u64` with `s1 in 0u64..u64::MAX`, etc.) and rewrite the Kani harness to either constrain `seq_raw` such that `seq_value != u64::MAX` or `match` the typed error explicitly.
- **black-hat-reviewer** — check that the six proptest relaxations do not silently accept `s1 == u64::MAX` as success (must skip, not success).
- **truth-serum / evidence-packaging** — gate the targeted evidence package on `cargo test -p workspace_tests --test restate_journal_tail_scan_fallback_tests -- --nocapture` plus `bash scripts/kani-list.sh vb_storage` for the harness lane.

## Verifier Mode Recommendations

- Default Rust lane: `cargo test -p workspace_tests --test restate_journal_tail_scan_fallback_tests -- --nocapture`
- Storage lib unit lane: `cargo test -p vb_storage --lib keys::tests::`
- Kani lane (probe): the harness now compiles but produces a trivial counterexample without a contract-binding patch. Use `bash scripts/kani-list.sh vb_storage` to inventory; the harness-only probe `bash scripts/kani-list.sh vb_storage kani-diagnostic-codes` if that feature gate is wired.
- Spec/proof: `verification/verus/extern_vb_storage_keys.rs:200,280,420` define `SpecKeyEncodeError::SequenceOverflow`; the harness tightening must keep the error literal consistent.

## Unknown / Out-of-Scope

- `crates/vb_runtime/src/journal/tests/chunk_004.rs:965-973` (`journal_event_is_valid_rejects_max_sequence`) — event validity is a separate invariant; out of scope per the bead's "key encoders" wording, but listed here so the implementation agent does not touch it.
- `crates/vb_storage/src/kani_record_kind.rs` harnesses — out of scope (record-kind contracts, not max-sequence key).
- `verification/verus/extern_vb_storage_keys.rs` spec mirror — out of immediate scope; the test-only fix should not require Verus edits, but if the Kani harness produces a new `cover!` claim it will.

## Anti-Hallucination Shield

- All file paths above were read with `Read` or listed with `rtk ls` from the isolated workspace.
- Every line range was confirmed via `Read` during exploration; quoted content is verbatim from the cited file.
- All test names listed appear under exactly one `#[test]` / `#[proptest]` attribute in the cited file.
- The contract source (`keys.rs:485-487`) was read in full; the `Err(_) => assert!(false)` pattern in the Kani harness was read at lines 56-87.
- No verifiers, fuzz harnesses, or production code paths are claimed-touched that were not verified.
