# Contract — vb-uwxct

- **bead_id**: vb-uwxct
- **title**: Tests: make max-sequence and key tests reject only exact overflow cases (P1 bug)
- **kind**: TEST-ONLY REPAIR
- **isolated_workdir**: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-uwxct

## 1. Statement

The production encoder `vb_storage::keys::run_event_key` (and its private
helper `sequenced_run_key`) returns `Err(JournalError::SequenceOverflow)`
**iff** `seq.get() == u64::MAX`, and `Ok([u8; 17])` otherwise. Seven
test/harness specimens currently treat a full-range `u64` input as if the
encoder could only succeed; on the sentinel input they panic or assert
`false`. This bead tightens those specimens so that the sentinel rejection
is observed as the contractually-correct response, and properties under test
continue to hold on the encodable range `0..u64::MAX`.

## 2. Scope

### In scope

| Symbol | Path | Action |
|---|---|---|
| proptest `run_event_key_lexicographic_ordering` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs:1326-1351` | tighten |
| proptest `sequence_bytes_roundtrip_through_key_encoding` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs:1355-1369` | tighten |
| proptest `run_event_key_always_17_bytes` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs:1373-1386` | tighten |
| proptest `run_event_key_always_has_correct_prefix` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs:1390-1401` | tighten |
| proptest `different_runs_have_different_event_key_prefixes` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs:1405-1423` | tighten |
| proptest `same_run_different_seq_keys_differ_in_seq_bytes` | `crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs:1427-1449` | tighten |
| Kani harness `assert_key_contracts` (called by `vb_eepg_typed_partitioned_ids`) | `crates/vb_storage/src/kani_typed_partitioned_ids.rs:43-115` | tighten |

### Reference only (do not modify)

- `crates/vb_storage/src/keys.rs` (production)
- `crates/vb_storage/src/keys/tests.rs` (unit tests, already correct)
- `crates/vb_storage/src/codec/tests.rs` (`encode_decode_with_max_sequence` uses `u64::MAX - 1`, correct)
- `crates/vb_storage/src/proptests.rs` (uses `0u64..1000u64`)
- `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs:123-146` (canonical-positive reference)
- `crates/vb_runtime/src/journal/tests/chunk_004.rs:964-973` (event validity, separate invariant)
- `verification/verus/extern_vb_storage_keys.rs` (spec mirror, out of scope)
- `verification/verus/production_inner/vb_vzcuf_PS_001_production.rs`, `…_PS_002_production.rs`

## 3. Preconditions

- The worker's isolated workspace is `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-uwxct`,
  confirmed by `pwd -P` resolving to that path.
- The production source at `crates/vb_storage/src/keys.rs:480-496` is
  already contract-correct (returns `Err(JournalError::SequenceOverflow)` for
  `seq == u64::MAX`, `Ok` otherwise). No edits to this file are required.
- Each specimen currently has the over-rejecting shape listed in
  `.beads/vb-uwxct/codebase-map.md`.

## 4. Postconditions

### 4.1 Targeted Cargo Test Postcondition

```bash
cargo test -p workspace_tests --test restate_journal_tail_scan_fallback_tests -- --nocapture
```

- Zero panics attributable to `run_event_key(...).expect(...)` over
  full-range `u64` inputs.
- All six tightened proptests pass on their property under test for every
  sampled `seq ∈ 0..u64::MAX`.
- The unit tests in `crates/vb_storage/src/keys/tests.rs`
  (`run_event_key_rejects_event_seq_max_sentinel`) remain green (the
  reference-positive tests confirm the contract).

### 4.2 Kani Harness Postcondition (only if the harness is byte-edited)

```bash
bash scripts/kani-list.sh vb_storage
```

followed by the package-level Kani probe that includes the harness group.
- `vb_eepg_typed_partitioned_ids` returns PASS.
- For inputs where `seq_value == u64::MAX`, the harness accepts the
  `Err(JournalError::SequenceOverflow)` rejection (no vacuous counterexample).

### 4.3 Source-Lint Postcondition

- Zero new `unwrap()`, `expect()`, `panic!`, `todo!`, `unimplemented!`, `dbg!`,
  `assert!(false)`, or `[T]::last()` / unchecked indexing in the six
  proptests and the Kani harness.
- The Kani harness body retains `Err(_) => assert!(false)` only if the
  sentinel rejection is **separately classified** as
  `Err(JournalError::SequenceOverflow)` ⇒ `assert!(seq_value == u64::MAX)`,
  or if `kani::assume(seq_value != u64::MAX)` is added at the top.

## 5. Invariants

For each of the seven repaired specimens, the post-repair invariant is:

```
∀ inputs sampled by the specimen engine:
  let result = run_event_key(run, seq) in
    ( result.is_ok()    ⟹ specimen asserts the documented property on the Ok shape )
    ∧
    ( result == Err(JournalError::SequenceOverflow)
       ⟹ specimen either skips (via assume / constraint) or treats it as
          contract-conformant — never as a specimen failure )
    ∧
    ( result == Err(other_variant) ⟹ specimen fails closed; "other" is
       unreachable on the current encoder so this arm must remain reachable
       only by future encoder breakage )
```

For each of the six proptests, the **property under test** must be unchanged
in the `Ok` case:

| Specimen | Property under test (verbatim, in Ok case) |
|---|---|
| `run_event_key_lexicographic_ordering` | `key1 < key2 ⇔ r1 < r2 ∨ (r1 == r2 ∧ s1 < s2)` |
| `sequence_bytes_roundtrip_through_key_encoding` | `u64::from_be_bytes(key[9..17]) == seq_val` |
| `run_event_key_always_17_bytes` | `key.len() == JOURNAL_KEY_BYTES` |
| `run_event_key_always_has_correct_prefix` | `key[0] == PREFIX_RUN_EVENT` |
| `different_runs_have_different_event_key_prefixes` | `&key1[..9] != &key2[..9]` |
| `same_run_different_seq_keys_differ_in_seq_bytes` | `&key1[..9] == &key2[..9]` ∧ `&key1[9..17] != &key2[9..17]` ∧ `key1 != key2` |

## 6. Failure-Mode Outcomes (within specimens)

| Failure | Specimen response | Status |
|---|---|---|
| `run_event_key(run, EventSeq::new(s))` returns `Err(SequenceOverflow)` with `s == u64::MAX` | Specimen skips via `prop_assume!` / `in 0u64..u64::MAX` OR explicit match arm classifies it as contract-conformant | OK |
| `run_event_key(...)` returns `Err(SequenceOverflow)` with `s ∈ 0..u64::MAX` | Specimen should fail closed with `prop_assert!(false, "encoder over-rejected")` | BAD (negative regression — encoder over-rejected) |
| `run_event_key(...)` returns `Ok(...)` with `s ∈ 0..u64::MAX` | Specimen property holds | OK |
| `run_event_key(...)` returns `Ok(...)` with `s == u64::MAX` | Specimen must fail closed (this is encoder regression) | BAD (positive regression — encoder over-accepted) |
| `run_event_key(...)` returns `Err(other_variant)` (currently unreachable) | Specimen fails closed via defensive `prop_assert!(false, "unexpected variant")` or harness `assert!(false)` | OK (defensive) |

## 7. Acceptance Signals

The bead is acceptable when:

1. `.beads/vb-uwxct/contract.md` is on disk.
2. `.beads/vb-uwxct/proof-seeds.jsonl` is valid JSONL, parsed with `jq -c .`
   without error.
3. `.beads/vb-uwxct/traceability-matrix.jsonl` is valid JSONL with one row
   per (specimen, contract_clause) tie.
4. `pwd -P` resolves to the isolated workdir.
5. (Closed by downstream agents) `cargo test -p workspace_tests --test restate_journal_tail_scan_fallback_tests -- --nocapture` exits 0.
6. (Closed by downstream agents) `bash scripts/kani-list.sh vb_storage` plus harness probe complete.

## 8. Non-Goals

- Modify production `vb_storage` code.
- Modify the `JournalError` enum or any `JournalError` variant.
- Modify the Verus spec mirror (`verification/verus/extern_vb_storage_keys.rs`).
- Touch the unit tests at `crates/vb_storage/src/keys/tests.rs:469-526`.
- Touch the proptest at `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs`.
- Touch event-validity tests at
  `crates/vb_runtime/src/journal/tests/chunk_004.rs`.
- Touch proptests at `crates/vb_storage/src/proptests.rs`.
- Touch `kani_record_kind.rs`.
- Add new dependencies.

## 9. Reciprocal Contracts (downstream consumers)

| Downstream | Reciprocal contract |
|---|---|
| `test-planner` | Plan one regression test per over-rejecting proptest that exercises both `seq == u64::MAX - 1` (success) and `seq == u64::MAX` (typed Err). Mirror the contract clauses C0..C6 below. |
| `proof-plan-reviewer` | Verify the planned Kani harness obligation targets production code paths (`KanIPath` directly bound to `sequenced_run_key`) and does not just relax the existing model. |
| `proof-writer` | Optional: extend `verification/verus/extern_vb_storage_keys.rs` spec mirror if the Kani tightening surfaces a new `cover!` claim. Do not duplicate. |
| `holzman-rust (implementation)` | Tighten the seven specimens using one of the four repair forms per `type-contracts.md` §6. |
| `black-hat-reviewer` | Walk through one full-case vector per specimen verifying the property under test still holds on `seq ∈ 0..u64::MAX`. |
| `truth-serum / evidence-packaging` | Gate the targeted evidence package on `cargo test -p workspace_tests --test restate_journal_tail_scan_fallback_tests -- --nocapture` plus `bash scripts/kani-list.sh vb_storage` after the harness tightening. |

## 10. Contract Clauses (referenced by `proof-seeds.jsonl`)

| ID | Clause |
|---|---|
| C0 | production encoder contract — `Err(JournalError::SequenceOverflow)` iff `seq.get() == u64::MAX`; Ok otherwise with 17-byte fixed layout |
| C1 | `run_event_key_lexicographic_ordering` accepts the sentinel rejection while preserving ordering property on `Ok` case |
| C2 | `sequence_bytes_roundtrip_through_key_encoding` accepts the sentinel rejection while preserving big-endian roundtrip on `Ok` case |
| C3 | `run_event_key_always_17_bytes` accepts the sentinel rejection while preserving 17-byte length on `Ok` case |
| C4 | `run_event_key_always_has_correct_prefix` accepts the sentinel rejection while preserving `key[0] == 0x11` on `Ok` case |
| C5 | `different_runs_have_different_event_key_prefixes` accepts the sentinel rejection while preserving per-run prefix distinction on `Ok` case |
| C6 | `same_run_different_seq_keys_differ_in_seq_bytes` accepts the sentinel rejection while preserving same-run/different-seq distinction on `Ok` case |
| C7 | Kani harness `assert_key_contracts` accepts the typed `SequenceOverflow` rejection when `seq_value == u64::MAX` (no vacuous counterexample) |

These IDs are referenced by the `contract_clause` field of every row in
`proof-seeds.jsonl` and the `contract_clause` field of every row in
`traceability-matrix.jsonl`.
