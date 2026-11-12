# Proof-to-Rust Map: vb-vzcuf State 7

bridge_skill: proof-to-implementation
bridge_invocation_id: vb-vzcuf-state7-proof-to-implementation-attempt1
proof_review_invocation_id: vb-vzcuf-state6-proof-reviewer-attempt2
proof_review_status: REJECTED (GOD RULE 2, self-approved TBPs, tautological proofs, missing production code)
mapping_status: planned

## GOD RULE 2 Gap (Deferred to State 11)

**Status: KNOWN GAP, deferred to State 11**

The 9 Verus obligations have standalone spec/proof functions in `verification/verus/` with "PRODUCTION BINDING:" comments but zero `requires`/`ensures` annotations on production `exec fn` in `crates/vb_storage/`. GOD RULE 2 requires mathematical binding via `requires`/`ensures` on the actual production `exec fn` verified by Verus.

Resolution: State 11 adds `staged_bytes`, `byte_limit`, `AccumulatedBytesExceeded`, and `requires`/`ensures` annotations. Compensating evidence: proptest exercises production `JournalWriteBatch` API; Kani harnesses call production `encode_record`.

## Proof-to-Rust Matrix

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |
|---|---|---|---|---|---|---|---|---|
| POB-vb-vzcuf-001 | PS-001 C3 Admission: accept exact fits, reject over-limit | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event | crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs | verification/verus/vb-vzcuf-PS-001.rs (GOD RULE 2 GAP) | verus | verus --crate-type=lib verification/verus/vb-vzcuf-PS-001.rs | 7 |
| POB-vb-vzcuf-002 | PS-001 C3 Admission bounded check | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event | crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs | verification/kani/vb-vzcuf-PS-001.rs | kani | cargo kani -p vb_storage --features kani-vb-vzcuf --harness check_admission_boundary | 7 |
| POB-vb-vzcuf-003 | PS-001 C3 Admission refinement | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event | crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs | verification/flux/vb-vzcuf-PS-001.rs | flux-rs | bash scripts/flux-check-package.sh vb_storage | 7 |
| POB-vb-vzcuf-004 | PS-001 C3 Admission property test | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event | crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs | crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs | proptest | cargo test -p vb_storage --test proptest_vb_vzcuf_PS_001 | 7 |
| POB-vb-vzcuf-005 | PS-002 C7 Overflow safety | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event | crates/vb_storage/tests/proptest_vb_vzcuf_PS_002.rs | verification/verus/vb-vzcuf-PS-002.rs (GOD RULE 2 GAP) | verus | verus --crate-type=lib verification/verus/vb-vzcuf-PS-002.rs | 7 |
| POB-vb-vzcuf-006 | PS-002 C7 Overflow bounded check | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event | crates/vb_storage/tests/proptest_vb_vzcuf_PS_002.rs | verification/kani/vb-vzcuf-PS-002.rs | kani | cargo kani -p vb_storage --features kani-vb-vzcuf --harness check_overflow_safety | 7 |
| POB-vb-vzcuf-007 | PS-002 C7 Overflow refinement | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event | crates/vb_storage/tests/proptest_vb_vzcuf_PS_002.rs | verification/flux/vb-vzcuf-PS-002.rs | flux-rs | bash scripts/flux-check-package.sh vb_storage | 7 |
| POB-vb-vzcuf-008 | PS-002 C7 Overflow property test | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event | crates/vb_storage/tests/proptest_vb_vzcuf_PS_002.rs | crates/vb_storage/tests/proptest_vb_vzcuf_PS_002.rs | proptest | cargo test -p vb_storage --test proptest_vb_vzcuf_PS_002 | 7 |
| POB-vb-vzcuf-009 | PS-003 C4/C6 Error distinctness | true | crates/vb_storage/src/error/mod.rs::JournalError::QueueFull | crates/vb_storage/tests/proptest_vb_vzcuf_PS_003.rs | verification/verus/vb-vzcuf-PS-003.rs (GOD RULE 2 GAP + LEATHAL tautology) | verus | verus --crate-type=lib verification/verus/vb-vzcuf-PS-003.rs | 7 |
| POB-vb-vzcuf-010 | PS-003 C4/C6 Error distinctness bounded | true | crates/vb_storage/src/error/mod.rs::JournalError | crates/vb_storage/tests/proptest_vb_vzcuf_PS_003.rs | verification/kani/vb-vzcuf-PS-003.rs | kani | cargo kani -p vb_storage --features kani-vb-vzcuf --harness check_error_distinctness | 7 |
| POB-vb-vzcuf-011 | PS-003 C4/C6 Error distinctness refinement | true | crates/vb_storage/src/error/mod.rs::JournalError | crates/vb_storage/tests/proptest_vb_vzcuf_PS_003.rs | verification/flux/vb-vzcuf-PS-003.rs | flux-rs | bash scripts/flux-check-package.sh vb_storage | 7 |
| POB-vb-vzcuf-012 | PS-003 C4/C6 Error distinctness property | true | crates/vb_storage/src/error/mod.rs::JournalError | crates/vb_storage/tests/proptest_vb_vzcuf_PS_003.rs | crates/vb_storage/tests/proptest_vb_vzcuf_PS_003.rs | proptest | cargo test -p vb_storage --test proptest_vb_vzcuf_PS_003 | 7 |
| POB-vb-vzcuf-013 | PS-004 C5 No partial mutation | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event | crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs | verification/verus/vb-vzcuf-PS-004.rs (GOD RULE 2 GAP + HIGH weak lemmas) | verus | verus --crate-type=lib verification/verus/vb-vzcuf-PS-004.rs | 7 |
| POB-vb-vzcuf-014 | PS-004 C5 No-mutation bounded check | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event | crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs | verification/kani/vb-vzcuf-PS-004.rs | kani | cargo kani -p vb_storage --features kani-vb-vzcuf --harness check_no_mutation_on_rejection | 7 |
| POB-vb-vzcuf-015 | PS-004 C5 State preservation refinement | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event | crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs | verification/flux/vb-vzcuf-PS-004.rs | flux-rs | bash scripts/flux-check-package.sh vb_storage | 7 |
| POB-vb-vzcuf-016 | PS-004 C5 No-mutation property test | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event | crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs | crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs | proptest | cargo test -p vb_storage --test proptest_vb_vzcuf_PS_004 | 7 |
| POB-vb-vzcuf-017 | PS-005 C2 Codec accounting | true | crates/vb_storage/src/codec/mod.rs::encode_record | crates/vb_storage/tests/proptest_vb_vzcuf_PS_005.rs | verification/verus/vb-vzcuf-PS-005.rs (GOD RULE 2 GAP) | verus | verus --crate-type=lib verification/verus/vb-vzcuf-PS-005.rs | 7 |
| POB-vb-vzcuf-018 | PS-005 C2 Codec bounded check | true | crates/vb_storage/src/codec/mod.rs::encode_record | crates/vb_storage/tests/proptest_vb_vzcuf_PS_005.rs | verification/kani/vb-vzcuf-PS-005.rs | kani | cargo kani -p vb_storage --features kani-vb-vzcuf --harness check_encode_record_length | 7 |
| POB-vb-vzcuf-019 | PS-005 C2 Codec refinement | true | crates/vb_storage/src/codec/mod.rs::encode_record | crates/vb_storage/tests/proptest_vb_vzcuf_PS_005.rs | verification/flux/vb-vzcuf-PS-005.rs | flux-rs | bash scripts/flux-check-package.sh vb_storage | 7 |
| POB-vb-vzcuf-020 | PS-005 C2 Codec property test | true | crates/vb_storage/src/codec/mod.rs::encode_record | crates/vb_storage/tests/proptest_vb_vzcuf_PS_005.rs | crates/vb_storage/tests/proptest_vb_vzcuf_PS_005.rs | proptest | cargo test -p vb_storage --test proptest_vb_vzcuf_PS_005 | 7 |
| POB-vb-vzcuf-021 | PS-006 C1 Limit presence | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::new | crates/vb_storage/tests/proptest_vb_vzcuf_PS_006.rs | verification/verus/vb-vzcuf-PS-006.rs (GOD RULE 2 GAP) | verus | verus --crate-type=lib verification/verus/vb-vzcuf-PS-006.rs | 7 |
| POB-vb-vzcuf-022 | PS-006 C1 Limit bounded check | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::new | crates/vb_storage/tests/proptest_vb_vzcuf_PS_006.rs | verification/kani/vb-vzcuf-PS-006.rs | kani | cargo kani -p vb_storage --features kani-vb-vzcuf --harness check_byte_limit_nonzero | 7 |
| POB-vb-vzcuf-023 | PS-006 C1 Limit refinement | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::new | crates/vb_storage/tests/proptest_vb_vzcuf_PS_006.rs | verification/flux/vb-vzcuf-PS-006.rs | flux-rs | bash scripts/flux-check-package.sh vb_storage | 7 |
| POB-vb-vzcuf-024 | PS-006 C1 Limit property test | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::new | crates/vb_storage/tests/proptest_vb_vzcuf_PS_006.rs | crates/vb_storage/tests/proptest_vb_vzcuf_PS_006.rs | proptest | cargo test -p vb_storage --test proptest_vb_vzcuf_PS_006 | 7 |
| POB-vb-vzcuf-025 | PS-007 C8 Core/storage bridge | true | crates/vb_core/src/workflow/mod.rs::ResourceContract::max_journal_batch_bytes | crates/vb_storage/tests/proptest_vb_vzcuf_PS_007.rs | verification/verus/vb-vzcuf-PS-007.rs (GOD RULE 2 GAP) | verus | verus --crate-type=lib verification/verus/vb-vzcuf-PS-007.rs | 7 |
| POB-vb-vzcuf-026 | PS-007 C8 Bridge bounded check | true | crates/vb_core/src/budget.rs::BudgetError::JournalBatchBytesExceeded | crates/vb_storage/tests/proptest_vb_vzcuf_PS_007.rs | verification/kani/vb-vzcuf-PS-007.rs | kani | cargo kani -p vb_storage --features kani-vb-vzcuf --harness check_budget_bridge | 7 |
| POB-vb-vzcuf-027 | PS-007 C8 Bridge refinement | true | crates/vb_core/src/workflow/mod.rs::ResourceContract::max_journal_batch_bytes | crates/vb_storage/tests/proptest_vb_vzcuf_PS_007.rs | verification/flux/vb-vzcuf-PS-007.rs | flux-rs | bash scripts/flux-check-package.sh vb_storage | 7 |
| POB-vb-vzcuf-028 | PS-007 C8 Bridge property test | true | crates/vb_core/src/budget.rs::WholeWorkflowBudget::max_journal_batch_bytes | crates/vb_storage/tests/proptest_vb_vzcuf_PS_007.rs | crates/vb_storage/tests/proptest_vb_vzcuf_PS_007.rs | proptest | cargo test -p vb_storage --test proptest_vb_vzcuf_PS_007 | 7 |
| POB-vb-vzcuf-029 | PS-008 C6 Guard precedence | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event | crates/vb_storage/tests/proptest_vb_vzcuf_PS_008.rs | verification/verus/vb-vzcuf-PS-008.rs (GOD RULE 2 GAP + LETHAL tautology) | verus | verus --crate-type=lib verification/verus/vb-vzcuf-PS-008.rs | 7 |
| POB-vb-vzcuf-030 | PS-008 C6 Guard bounded check | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event | crates/vb_storage/tests/proptest_vb_vzcuf_PS_008.rs | verification/kani/vb-vzcuf-PS-008.rs | kani | cargo kani -p vb_storage --features kani-vb-vzcuf --harness check_guard_precedence | 7 |
| POB-vb-vzcuf-031 | PS-008 C6 Guard refinement | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event | crates/vb_storage/tests/proptest_vb_vzcuf_PS_008.rs | verification/flux/vb-vzcuf-PS-008.rs | flux-rs | bash scripts/flux-check-package.sh vb_storage | 7 |
| POB-vb-vzcuf-032 | PS-008 C6 Guard property test | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event | crates/vb_storage/tests/proptest_vb_vzcuf_PS_008.rs | crates/vb_storage/tests/proptest_vb_vzcuf_PS_008.rs | proptest | cargo test -p vb_storage --test proptest_vb_vzcuf_PS_008 | 7 |
| POB-vb-vzcuf-033 | PS-009 C2 Duplicate accounting | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event | crates/vb_storage/tests/proptest_vb_vzcuf_PS_009.rs | verification/verus/vb-vzcuf-PS-009.rs (GOD RULE 2 GAP) | verus | verus --crate-type=lib verification/verus/vb-vzcuf-PS-009.rs | 7 |
| POB-vb-vzcuf-034 | PS-009 C2 Duplicate bounded check | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event | crates/vb_storage/tests/proptest_vb_vzcuf_PS_009.rs | verification/kani/vb-vzcuf-PS-009.rs | kani | cargo kani -p vb_storage --features kani-vb-vzcuf --harness check_duplicate_accounting | 7 |
| POB-vb-vzcuf-035 | PS-009 C2 Duplicate refinement | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event | crates/vb_storage/tests/proptest_vb_vzcuf_PS_009.rs | verification/flux/vb-vzcuf-PS-009.rs | flux-rs | bash scripts/flux-check-package.sh vb_storage | 7 |
| POB-vb-vzcuf-036 | PS-009 C2 Duplicate property test | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event | crates/vb_storage/tests/proptest_vb_vzcuf_PS_009.rs | crates/vb_storage/tests/proptest_vb_vzcuf_PS_009.rs | proptest | cargo test -p vb_storage --test proptest_vb_vzcuf_PS_009 | 7 |
| POB-vb-vzcuf-037 | PS-001 C3 Fuzz admission | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event | crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs | fuzz/fuzz_targets/vb_vzcuf_PS_001.rs | cargo-fuzz | cargo fuzz run vb_vzcuf_PS_001 -- -max_total_time=60 | 7 |
| POB-vb-vzcuf-038 | PS-002 C7 Fuzz overflow | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event | crates/vb_storage/tests/proptest_vb_vzcuf_PS_002.rs | fuzz/fuzz_targets/vb_vzcuf_PS_002.rs | cargo-fuzz | cargo fuzz run vb_vzcuf_PS_002 -- -max_total_time=60 | 7 |
| POB-vb-vzcuf-039 | PS-003 C4/C6 Fuzz error distinctness | true | crates/vb_storage/src/error/mod.rs::JournalError | crates/vb_storage/tests/proptest_vb_vzcuf_PS_003.rs | fuzz/fuzz_targets/vb_vzcuf_PS_003.rs | cargo-fuzz | cargo fuzz run vb_vzcuf_PS_003 -- -max_total_time=60 | 7 |
| POB-vb-vzcuf-040 | PS-004 C5 Fuzz no-mutation | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event | crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs | fuzz/fuzz_targets/vb_vzcuf_PS_004.rs | cargo-fuzz | cargo fuzz run vb_vzcuf_PS_004 -- -max_total_time=60 | 7 |
| POB-vb-vzcuf-041 | PS-005 C2 Fuzz codec | true | crates/vb_storage/src/codec/mod.rs::encode_record | crates/vb_storage/tests/proptest_vb_vzcuf_PS_005.rs | fuzz/fuzz_targets/vb_vzcuf_PS_005.rs | cargo-fuzz | cargo fuzz run vb_vzcuf_PS_005 -- -max_total_time=60 | 7 |
| POB-vb-vzcuf-042 | PS-006 C1 Fuzz constructor | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::new | crates/vb_storage/tests/proptest_vb_vzcuf_PS_006.rs | fuzz/fuzz_targets/vb_vzcuf_PS_006.rs | cargo-fuzz | cargo fuzz run vb_vzcuf_PS_006 -- -max_total_time=60 | 7 |
| POB-vb-vzcuf-043 | PS-007 C8 Fuzz bridge | true | crates/vb_core/src/budget.rs::BudgetError::JournalBatchBytesExceeded | crates/vb_storage/tests/proptest_vb_vzcuf_PS_007.rs | fuzz/fuzz_targets/vb_vzcuf_PS_007.rs | cargo-fuzz | cargo fuzz run vb_vzcuf_PS_007 -- -max_total_time=60 | 7 |
| POB-vb-vzcuf-044 | PS-008 C6 Fuzz guard | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event | crates/vb_storage/tests/proptest_vb_vzcuf_PS_008.rs | fuzz/fuzz_targets/vb_vzcuf_PS_008.rs | cargo-fuzz | cargo fuzz run vb_vzcuf_PS_008 -- -max_total_time=60 | 7 |
| POB-vb-vzcuf-045 | PS-009 C2 Fuzz duplicate | true | crates/vb_storage/src/batch.rs::JournalWriteBatch::append_event | crates/vb_storage/tests/proptest_vb_vzcuf_PS_009.rs | fuzz/fuzz_targets/vb_vzcuf_PS_009.rs | cargo-fuzz | cargo fuzz run vb_vzcuf_PS_009 -- -max_total_time=60 | 7 |

## C9 Observability Gap

Contract clause C9 has no dedicated proof obligation. The planned `staged_bytes: u64` field on `JournalWriteBatch` will support an accessor but no obligation verifies its existence/correctness. Must be resolved before State 12.

## Exact Handoff Inputs for proof-reviewer

1. proof-to-rust-map.md
2. rust-refinement-obligations.jsonl (45 RRO rows)
3. agent-invocation-ledger.jsonl (seq 10)
4. proof-review.md (State 6 REJECTED context)
5. proof-findings.jsonl
6. contract.md
7. proof-obligations.planned.jsonl
