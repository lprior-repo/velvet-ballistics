# Proof Strategy — vb-pg2wq duplicate-event test exact-contract repair

STATUS: PLANNED. No verifier, test, fuzz, CI, or proof success is claimed here.

## Bead identity

- **bead_id**: vb-pg2wq
- **title**: Tests: make duplicate-event test assert one exact contract (P1 bug)
- **lane**: test-only Rust-local assertion repair
- **scope**: 6 weak `matches!(.., JournalError::DuplicateEvent { .. })` occurrences in 5 proptest functions across 4 files under `crates/vb_storage/tests/`
- **invariant source**: `crates/vb_storage/src/batch/append_event.rs:61-67` (production contract, NOT modified)
- **canonical pattern reference**: `crates/vb_storage/src/tests.rs:1344-1367` (`fn duplicate_event_returns_exact_run_and_seq`)
- **proptest analog reference**: `crates/vb_storage/src/tests.rs:4888-4892` (`fn journal_writer_queue_flush_rejects_duplicate_event`)

## Scope

This bead is a test-only assertion rewrite. It does NOT modify production code under `crates/vb_storage/src/`, does NOT modify any `Cargo.toml`, and does NOT introduce new proof harnesses. The full set of edits is:

| # | File | Function | Weak lines | Strong lines | Target |
|---|------|----------|-----------|--------------|--------|
| 1 | `crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs` | `ps001_duplicate_rejected` | 77-78 | 77-81 | PO-vb-pg2wq-001 |
| 2 | `crates/vb_storage/tests/proptest_vb_vzcuf_PS_003.rs` | `ps003_dup_fields` | 63-64 | 63-67 | PO-vb-pg2wq-001 |
| 3 | `crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs` | `ps004_no_persist` | 47-48 | 47-51 | PO-vb-pg2wq-002 |
| 4 | `crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs` | `ps004_empty_commit_after_rej` | 93-94 | 93-97 | PO-vb-pg2wq-002 |
| 5 | `crates/vb_storage/tests/proptest_vb_vzcuf_PS_008.rs` | `ps008_dup_before_queue` | 35 | 35-39 | PO-vb-pg2wq-001 |
| 6 | `crates/vb_storage/tests/proptest_vb_vzcuf_PS_009.rs` | `ps009_dup_rejected` | 35-36 | 35-39 | PO-vb-pg2wq-001 |

All 6 occurrences are rewritten from the weak `prop_assert!(matches!(result, Err(JournalError::DuplicateEvent { .. })))` to the field-bound `prop_assert!(matches!(result, Err(JournalError::DuplicateEvent { run: r, seq: s }) if r == RunId::new(run) && s == EventSeq::new(seq)))`. The proptest input strategies are preserved verbatim. All secondary assertions (in `ps004_no_persist` and `ps004_empty_commit_after_rej`) are preserved verbatim.

## Risk classification

The seed-level risk profile is uniform across the 6 function-specific seeds:

- `audit-regression-resistance` (primary)
- `test-quality` (primary)
- `variant-confusion` (primary, distinguishing `DuplicateEvent` from `DuplicateStagedKey` and all other `Err` variants)
- `secondary-invariant-preservation` (for PS_004 seeds only)
- `pattern-discipline` (for the class-no-regression seed)

The risk classes driving lane decisions are:

- `field_sensitivity` (proptest obligation PO-vb-pg2wq-001/002 must pin the `run: RunId` and `seq: EventSeq` tuple, not just the variant)
- `equality` (source-lint obligation PO-vb-pg2wq-003 must hold zero hits on the weak pattern post-landing)

Triggers NOT present (recorded as `not_applicable` per seed in `verifier-lane-decisions.jsonl`):

- **Rust-local invariant / pure-core / arithmetic** (Verus): no production change in scope
- **Bounded state machine / rejection** (Kani): the existing Kani harness at `crates/vb_storage/src/kani_vb_vzcuf_ps004.rs:48-59` already models the contract; no new harness is required. Kani binding is strengthened, not added.
- **Refinement / index / ownership** (Flux-rs): no refinement annotations, no extern_specs, no Flux-backed predicates introduced
- **Concurrency / interleaving / cancellation / shutdown / channel / lock / task_ownership** (Loom): tests are single-threaded; no `Send`/`Sync` boundary in the test surface
- **UB / unsafe / FFI / raw_pointer / aliasing / provenance / layout** (Miri): workspace enforces `forbid(unsafe_code)`; no unsafe surface in test code
- **Parser / codec / hostile_input / persisted_bytes / ipc_decode / fuzzable_canonicalization** (cargo-fuzz): the input surface is proptest-generated typed `(u64, u64)`; no hostile-byte parser/codec. The corresponding PS_009 fuzz target at `fuzz/fuzz_targets/vb_vzcuf_PS_009.rs` is OUT OF SCOPE per codebase-map.md and contract.md.

## Lane policy application

Three `proof-obligation/v1` rows are planned:

| ID | Verifier | Clause | Required lanes | Notes |
|----|----------|--------|----------------|-------|
| `PO-vb-pg2wq-001` | proptest | `O1-exact-tuple-pin-and-variant-discriminant` | cargo-test (proptest) | 4 functions: ps001/ps003/ps008/ps009 |
| `PO-vb-pg2wq-002` | proptest | `O1-exact-tuple-pin-and-variant-discriminant` | cargo-test (proptest) | 2 functions: ps004_no_persist / ps004_empty_commit_after_rej (with secondary invariants) |
| `PO-vb-pg2wq-003` | proptest | `O8-no-forbidden-constructs` | source-lint (cargo fmt --check + scripts/check-test-integrity.sh + rtk rg weak-pattern scan + clippy) | cross-cutting pattern-discipline scan |

The verifier-lane-decisions.jsonl contains 56 rows: 8 seeds × 7 default-profile verifiers (verus, kani, flux-rs, loom, miri, cargo-fuzz, proptest). 7 rows are `applicability: required` (one proptest row per function-specific seed plus the class-no-regression seed) and 49 rows are `applicability: not_applicable` with concrete evidence references (contract.md O6-no-production-change SHA-256, codebase-map.md lines 318-324 Kani binding SHA-256, codebase-map.md lines 102-106 fuzz out-of-scope SHA-256, codebase-map.md lines 301-306 concurrency surface SHA-256, codebase-map.md lines 263-278 production API surface SHA-256, AGENTS.md engineering-rules forbid(unsafe_code)).

## Cargo test verifier commands (planned evidence)

The proptest obligations are exercised via the workspace's pinned nightly toolchain:

```
cargo test -p vb_storage --test proptest_vb_vzcuf_PS_001 ps001_duplicate_rejected --no-fail-fast
cargo test -p vb_storage --test proptest_vb_vzcuf_PS_003 ps003_dup_fields --no-fail-fast
cargo test -p vb_storage --test proptest_vb_vzcuf_PS_004 ps004_no_persist --no-fail-fast
cargo test -p vb_storage --test proptest_vb_vzcuf_PS_004 ps004_empty_commit_after_rej --no-fail-fast
cargo test -p vb_storage --test proptest_vb_vzcuf_PS_008 ps008_dup_before_queue --no-fail-fast
cargo test -p vb_storage --test proptest_vb_vzcuf_PS_009 ps009_dup_rejected --no-fail-fast
```

The first four are bound to PO-vb-pg2wq-001; the next two (and the PS_004 set) are bound to PO-vb-pg2wq-002. Each command should pass under the field-bound assertion. A failure indicates either a production regression (production code returns wrong tuple) or a test-setup bug — both are surfaced cleanly by the strengthened assertion.

## Source-lint verifier commands (planned evidence)

The source-lint obligation PO-vb-pg2wq-003 is exercised by:

```
rustup run nightly-2026-04-28 cargo fmt --all --check
bash scripts/check-test-integrity.sh
rtk rg -n -- 'JournalError::DuplicateEvent \{ \.\. \}' \
    crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs \
    crates/vb_storage/tests/proptest_vb_vzcuf_PS_003.rs \
    crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs \
    crates/vb_storage/tests/proptest_vb_vzcuf_PS_008.rs \
    crates/vb_storage/tests/proptest_vb_vzcuf_PS_009.rs
```

Plus the workspace `moon :lint-src` task as the canonical closure gate.

## Adjacent (out-of-scope) follow-up candidates

These weak `matches!(.., JournalError::DuplicateEvent { .. })` patterns exist in the codebase but are NOT modified by this bead (per contract.md §Adjacent Out-of-Scope Follow-Up Candidates and codebase-map.md §Adjacent NOT in scope):

| File | Function | Lines |
|------|----------|-------|
| `crates/vb_storage/src/batch/t_append_event.rs` | `batch_append_event_rejects_duplicate_event` | 20-43 |
| `crates/vb_storage/src/batch/t_byte_accounting_part2.rs` | `rejected_duplicate_event_not_staged_in_batch` | 84-106 |
| `crates/vb_storage/src/batch/t_byte_accounting_part3.rs` | `duplicate_detection_fires_before_count_check` | 5-20 |
| `crates/vb_storage/src/batch/t_byte_accounting_part3.rs` | `duplicate_and_queue_full_conflict_duplicate_wins` | 55-70 |
| `crates/vb_storage/src/batch/t_byte_accounting_part4.rs` | `cross_batch_duplicate_is_rejected_with_duplicate_event` | 5-20 |
| `crates/vb_storage/src/batch/t_byte_accounting_part4.rs` | `duplicate_event_aborts_batch` | 22-36 |
| `crates/vb_storage/src/batch/t_byte_accounting_part4.rs` | `e2e_aborted_batch_commit_returns_typed_batch_aborted_error` | 76-104 |
| `crates/vb_storage/src/batch/t_byte_accounting_part4.rs` | `append_strict_batch_atomicity_rolls_back_on_duplicate` | 106-129 |
| `crates/vb_storage/src/tests.rs` | `duplicate_event_append_is_rejected` | 837-851 |
| `crates/workspace_tests/tests/journal_side_index_contracts.rs` | `two_in_flight_same_run_seq` | 495-531 |

These are flagged for follow-up beads but not modified here.

## Waiver posture

No behavior-affecting waiver is made. `waiver-candidates.jsonl` is empty by design: every planned obligation is bounded to the test surface, and the production contract is preserved verbatim. The `E_BEHAVIOR_WAIVER` failure mode is avoided by the test-only scope of the bead.

## Release gate

The bead's planned evidence is the workspace `moon ci` canonical gate (per AGENTS.md "moon ci is canonical"), with `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_001/003/004/008/009` and `bash scripts/check-test-integrity.sh` as the targeted test gates, and `moon :lint-src` as the source-lint gate.