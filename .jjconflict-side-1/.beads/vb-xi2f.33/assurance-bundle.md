# Assurance Bundle

**bead_id**: `vb-xi2f.33`
**source_checkout**: `/home/lewis/src/vb-workspaces/vb-xi2f.33`
**isolated_workspace**: `/home/lewis/src/vb-workspaces/vb-xi2f.33`
**commit_or_change**: `vb-xi2f.33` / P1: digest covers ask semantics
**packaging_date**: 2026-05-25
**packaging_agent**: `evidence-packaging` (deepseek-v4-pro)

## Requirement Coverage

| Requirement | Contract Clause | Proof/Test Evidence | Review Evidence | Status |
|---|---|---|---|---|
| R1: Ask prompt must be hashed | INV-ASK-001, POST-001 | PO-KANI-001 (FAIL_LOCAL), PO-PROPTEST-001 (PASS: 1000 random pairs), B1: `digest_ask_prompt_sensitivity.rs` (6 unit), `digest_yaml_e2e.rs` (2 e2e) | proof-review.md: APPROVED; test-suite-review.md: APPROVED; formal-verification-report.md: PARTIAL PASS | **COVERED** |
| R2: Ask timeout must be hashed | INV-ASK-002, POST-002 | PO-KANI-002 (FAIL_LOCAL), PO-PROPTEST-002 (PASS: 1000 random pairs), B2: `digest_ask_timeout_sensitivity.rs` (6 unit) | proof-review.md: APPROVED; test-suite-review.md: APPROVED; formal-verification-report.md: PARTIAL PASS | **COVERED** |
| R3: Digest must be deterministic | INV-ASK-003, POST-003 | PO-PROPTEST-003 (PASS: 500 random sources), B3: `digest_ask_determinism.rs` (5 unit), `digest_compilation_pipeline.rs` (integration) | proof-review.md: APPROVED; test-suite-review.md: APPROVED | **COVERED** |
| R4: Empty prompt must produce distinct digest | INV-ASK-004, POST-004 | PO-KANI-003 (FAIL_LOCAL), PO-PROPTEST-003 (PASS), B4: `digest_ask_empty_prompt.rs` (4 unit) | proof-review.md: APPROVED; test-suite-review.md: APPROVED | **COVERED** |
| R5: None vs Some("") timeout distinction | INV-ASK-005, POST-005 | PO-KANI-004 (FAIL_LOCAL), PO-PROPTEST-002 (PASS), B5: `digest_ask_timeout_sensitivity.rs` (2 unit), B11: sentinel test | proof-review.md: APPROVED; test-suite-review.md: APPROVED | **COVERED** |
| R6: Fix must apply to both duplicate sites | INV-ASK-006, POST-006 | PO-KANI-005 (FAIL_LOCAL), B6: `digest_duplicate_parity.rs` (4 unit) | proof-review.md: APPROVED; test-suite-review.md: APPROVED; code review confirms identical arms in both files | **COVERED** |
| R7: No Set/Finish regression | INV-ASK-007, POST-007 | B7: `digest_set_finish_regression.rs` (12 unit), integration + e2e tests; TB-005 (delegated) | proof-review.md: APPROVED; test-suite-review.md: APPROVED; 245 lib tests pass (0 failures) | **COVERED** |
| TC-001: Explicit Ask match arm in digest_step_primitive | — | B8: `digest_ask_explicit_arm.rs` (2 runtime); static grep confirms `Ask { prompt, timeout }` at part_05.rs:158 | proof-review.md: APPROVED; test-suite-review.md: APPROVED | **COVERED** |
| TC-002: Deterministic field ordering for Ask | — | PO-PROPTEST-004 (PASS: 500 random inputs), B9: `digest_ask_determinism.rs` (2 unit) | proof-review.md: APPROVED; confirmed by code review of part_05.rs:158-170 | **COVERED** |
| TC-003: Empty prompt handled correctly | — | B10: `digest_ask_explicit_arm.rs` (3 unit) | test-suite-review.md: APPROVED | **COVERED** |
| TC-004: Timeout sentinel distinction | — | B11: `digest_ask_explicit_arm.rs` (1 unit); static grep CI | test-suite-review.md: APPROVED; TB-003 (verified-by-proptest) | **COVERED** |
| TC-005: No Set/Finish regression in digest_step_primitive | — | B13/B14: `digest_set_finish_regression.rs` (12 unit) | test-suite-review.md: APPROVED | **COVERED** |
| TC-006: Duplicate implementation parity | — | B6: `digest_duplicate_parity.rs` (4 unit) | test-suite-review.md: APPROVED | **COVERED** |
| TC-007: No panic/unwrap/expect in digest_step_primitive | — | PO-KANI-006 (FAIL_LOCAL), B12: `digest_ask_explicit_arm.rs` (10 unit); static grep confirms 0 unwrap/expect/panic/unsafe | proof-review.md: APPROVED; test-suite-review.md: APPROVED | **COVERED** |
| WF-INV-001: Deterministic digest path end-to-end | — | PO-PROPTEST-003/004 (PASS), B16: `digest_structural_fields.rs` (2 unit) | test-suite-review.md: APPROVED | **COVERED** |
| WF-INV-004: All Ask fields consumed by hasher | — | PO-KANI-001/002/004/005 (FAIL_LOCAL), proptest evidence (3000 total random cases) | proof-review.md: APPROVED | **COVERED** |

## Proof Evidence

| Obligation | Tool | Command | Artifact | Result | Waiver |
|---|---|---|---|---|---|
| PO-KANI-001 | Kani | `cargo kani -p vb_compile --harness check_ask_prompt_sensitivity --unwind 3` | `crates/vb_compile/src/kani_digest_ask_prompt_sensitivity.rs` | FAIL_LOCAL (blake3 InlineAsm) | Compensated by PO-PROPTEST-001 (PASS, 1000 cases) |
| PO-KANI-002 | Kani | `cargo kani -p vb_compile --harness check_ask_timeout_sensitivity --unwind 3` | `crates/vb_compile/src/kani_digest_ask_timeout_sensitivity.rs` | FAIL_LOCAL (blake3 InlineAsm) | Compensated by PO-PROPTEST-002 (PASS, 1000 cases) |
| PO-KANI-003 | Kani | `cargo kani -p vb_compile --harness check_empty_prompt_distinct --unwind 5` | `crates/vb_compile/src/kani_digest_ask_empty_prompt.rs` | FAIL_LOCAL (blake3 InlineAsm) | Compensated by PO-PROPTEST-003 (PASS, 500 cases) |
| PO-KANI-004 | Kani | `cargo kani -p vb_compile --harness check_timeout_sentinel_distinction --unwind 5` | `crates/vb_compile/src/kani_digest_ask_timeout_sentinel.rs` | FAIL_LOCAL (blake3 InlineAsm) | Compensated by PO-PROPTEST-002 (PASS, 1000 cases) |
| PO-KANI-005 | Kani | `cargo kani -p vb_compile --harness check_ask_field_ordering_deterministic --unwind 3` | `crates/vb_compile/src/kani_digest_ask_field_ordering.rs` | FAIL_LOCAL (blake3 InlineAsm) | Compensated by PO-PROPTEST-004 (PASS, 500 cases) + code review |
| PO-KANI-006 | Kani | `cargo kani -p vb_compile --harness check_digest_step_primitive_no_panic --unwind 3` | `crates/vb_compile/src/kani_digest_step_primitive_no_panic.rs` | FAIL_LOCAL (blake3 InlineAsm) | Compensated by 245 unit tests (PASS) + code review |
| PO-PROPTEST-001 | proptest | `cargo test -p vb_compile --test proptest_digest_ask_prompt_sensitivity` | `crates/vb_compile/tests/proptest_digest_ask_prompt_sensitivity.rs` | **PASS** (1000 random pairs, 0.31s) | None |
| PO-PROPTEST-002 | proptest | `cargo test -p vb_compile --test proptest_digest_ask_timeout_sensitivity` | `crates/vb_compile/tests/proptest_digest_ask_timeout_sensitivity.rs` | **PASS** (1000 random pairs, 0.05s) | None |
| PO-PROPTEST-003 | proptest | `cargo test -p vb_compile --test proptest_digest_determinism` | `crates/vb_compile/tests/proptest_digest_determinism.rs` | **PASS** (500 sources, 0.12s) | None |
| PO-PROPTEST-004 | proptest | `cargo test -p vb_compile --test proptest_digest_ask_ordering` | `crates/vb_compile/tests/proptest_digest_ask_ordering.rs` | **PASS** (500 inputs, 0.11s) | None |
| PO-FUZZ-001 | cargo-fuzz | `cargo check --manifest-path fuzz/Cargo.toml` (compilation); `cargo fuzz run canonical_digest_ask -- -max_len=65536 -runs=100000` (execution) | `fuzz/fuzz_targets/canonical_digest_ask.rs` | COMPILES (PASS); execution **DEFERRED** | Execution deferred — long-running security check; not required for bead closure per bridge review |

## Test Evidence

| Test/Gate | Command | Artifact | Result |
|---|---|---|---|
| Unit tests (regression) | `cargo test -p vb_compile --lib` | 245 lib tests | **245/245 PASS** |
| Full test suite | `cargo test -p vb_compile` | All digest tests (10 files) + lib tests | **371/371 PASS** (20 suites, 3.01s) |
| Digest prompt sensitivity | (integrated in above) | `digest_ask_prompt_sensitivity.rs` (6 unit) | PASS |
| Digest timeout sensitivity | (integrated in above) | `digest_ask_timeout_sensitivity.rs` (6 unit) | PASS |
| Digest determinism | (integrated in above) | `digest_ask_determinism.rs` (5 unit) | PASS |
| Digest empty prompt | (integrated in above) | `digest_ask_empty_prompt.rs` (4 unit) | PASS |
| Digest explicit arm | (integrated in above) | `digest_ask_explicit_arm.rs` (10 unit) | PASS |
| Set/Finish regression | (integrated in above) | `digest_set_finish_regression.rs` (12 unit) | PASS |
| Structural fields | (integrated in above) | `digest_structural_fields.rs` (6 unit) | PASS |
| Compilation pipeline | (integrated in above) | `digest_compilation_pipeline.rs` (integration) | PASS |
| YAML E2E | (integrated in above) | `digest_yaml_e2e.rs` (e2e) | PASS |
| Duplicate parity | (integrated in above) | `digest_duplicate_parity.rs` (4 unit) | PASS |
| Proptest: prompt sensitivity | `cargo test -p vb_compile --test proptest_digest_ask_prompt_sensitivity` | 1000 random prompt pairs | **PASS** (0.31s) |
| Proptest: timeout sensitivity | `cargo test -p vb_compile --test proptest_digest_ask_timeout_sensitivity` | 1000 random timeout pairs | **PASS** (0.05s) |
| Proptest: determinism | `cargo test -p vb_compile --test proptest_digest_determinism` | 500 random sources | **PASS** (0.12s) |
| Proptest: field ordering | `cargo test -p vb_compile --test proptest_digest_ask_ordering` | 500 random inputs | **PASS** (0.11s) |
| Moon CI | `moon ci` | 27 tasks | **27/27 PASS** (7 cached, 3m59s, 0 failures) |
| Crate compilation | `cargo check -p vb_compile --tests` | vb_compile crate | PASS |
| Crate compilation (vb_yaml) | `cargo check -p vb_yaml` | vb_yaml crate (visibility changes) | PASS |
| Production code panic scan | `grep -c -E '\b(unwrap\|expect\|panic\|todo\|unimplemented)\b' part_05.rs` | part_05.rs lines 140-175 | **0 matches** |
| Production code unsafe scan | `grep -c 'unsafe' part_05.rs` | part_05.rs | **0 matches** |
| No ignored tests | `grep -r '#\[ignore\]'` | All digest test files | **0 matches** |
| Ask arm confirm (active path) | `grep 'Ask { prompt, timeout }' part_05.rs` | part_05.rs:158 | **CONFIRMED** |
| Ask arm confirm (legacy path) | `grep 'Ask { prompt, timeout }' compile/mod.rs` | compile/mod.rs:257 | **CONFIRMED** |

## Review Evidence

| Review | Artifact | Status | Findings |
|---|---|---|---|
| Contract Definition | `contract.md` | Defined (State 3) | 7 invariants, 7 TC clauses, 7 POST conditions |
| Proof Plan Review | `.beads/vb-xi2f.33/proof-plan-review.md` | **STATUS: APPROVED** | 4 advisory findings, 0 blockers |
| Proof Review (Round 2) | `.beads/vb-xi2f.33/proof-review.md` | **STATUS: APPROVED** | 1 MEDIUM (provenance), 4 LOW (bookkeeping/docs), 2 INFO (tooling). All CRITICAL/HIGH resolved. |
| Proof-to-Rust Bridge Review (RETRY) | `.beads/vb-xi2f.33/proof-to-rust-review.md` | **STATUS: APPROVED** | 4 CRITICAL/HIGH/MEDIUM resolved. 3 MEDIUM + 4 LOW remain (non-blocking). |
| Test Suite Review (RETRY) | `.beads/vb-xi2f.33/test-suite-review.md` | **STATUS: APPROVED** | CRITICAL TSR-001 resolved. 2 MEDIUM→LOW (tag mutation gap, no golden digest). 2 LOW new findings. |
| Formal Verification Report | `reports/formal-verification-report.md` | **Result: PARTIAL PASS** | 4/11 PASS, 6/11 FAIL_LOCAL (blake3 InlineAsm), 1/11 deferred. No blockers for bead delivery. |
| Black-Hat Review | **MISSING_EVIDENCE** — no `black-hat-review.md` found in `.beads/vb-xi2f.33/` | **UNVERIFIED** | User reports "Black-hat APPROVED WITH CONDITIONS" but artifact is absent from the bead directory. |
| Moon CI Gate | `.beads/moon-ci-status.txt: EXIT_CODE: ` (empty/incomplete); verification-ledger.jsonl:52 (moon ci: 27 tasks, PASS) | PASS (per ledger) | `.beads/moon-ci-status.txt` has empty EXIT_CODE but verification-ledger line 52 records successful moon ci. |

## Waivers And Deferred Work

| Item | Reason | Owner | Expiry/Follow-up | Compensating Evidence |
|---|---|---|---|---|
| Kani blake3 InlineAsm barrier (6 harnesses) | Known Kani tooling limitation: `TerminatorKind::InlineAsm is not currently supported` in blake3's `__cpuid_count`. Not a code defect. | Kani maintainers (kani#2) | Until Kani supports inline asm | 4/4 proptest PASS (3000 total random cases); 245 unit tests PASS; code review confirms fix is applied |
| Fuzz execution (PO-FUZZ-001) | Compilation confirmed; long-running execution deferred per bridge review | Follow-up bead | Security check, not blocking for bead closure | Compilation PASS; harness is structurally correct |
| Dead-code parity test replicas (not actual `compile/mod.rs`) | `compile/mod.rs` is unmounted (no `mod compile;` in `lib.rs`) — cannot import. Local replicas verify algorithm parity. | Future bead (if legacy path is re-mounted) | Follow-up on legacy path decision | 4 parity tests PASS; both files have identical Ask arms (confirmed by code review) |
| `kani-list.json` not updated (0 entries) | 6 new Kani harnesses not registered for CI coverage tracking | Bookkeeping bead | CI coverage gap | Harnesses are in the crate tree (`#[cfg(kani)] pub mod` in lib.rs) |
| Golden digest test not written | Deterrence against accidental algorithm changes | Follow-up bead | `digest_ask_determinism.rs` | All behavior invariants covered by relative comparison tests |
| Cross-primitive tag test (b"ask" removal survives all tests) | Defense-in-depth: `b"ask"` tag provides cross-primitive disambiguation | Follow-up bead | Test improvement | Core contract invariants fully covered by existing tests |
| Agent invocation ledger incomplete | Missing proof-planner, proof-writer, and proof-reviewer round 1 entries | Bookkeeping | Provenance traceability gap | Does not affect proof soundness |
| No `machine-gate-report.md` | Artifact not generated for this bead | **MISSING_EVIDENCE** | Gate coverage gap | Moon CI evidence in verification-ledger.jsonl:52 (27 tasks, 0 failures) |
| No `regression-diff.md` | Artifact not generated for this bead | **MISSING_EVIDENCE** | Regression tracking gap | 245 existing unit tests pass (0 regression); Ask arm is additive (no existing code modified) |

## Truth Serum Audit

- **Status**: Not yet run. `truth-serum-report.md` and `final-evidence-decision.md` must be created after audit.
- **Audit scope**: All artifacts referenced in this bundle.

## Implementation Fix Verification

Both duplicate sites contain the identical `Ask { prompt, timeout }` arm:

**Active path** (`crates/vb_compile/src/mod_compile_lowering/part_05.rs:158-170`):
```rust
vb_yaml::ast::StepPrimitive::Ask { prompt, timeout } => {
    hasher.update(b"ask");
    hasher.update(prompt.as_bytes());
    match timeout {
        Some(t) => {
            hasher.update(b"timeout");
            hasher.update(t.as_bytes());
        }
        None => {
            hasher.update(b"no_timeout");
        }
    }
}
```

**Legacy/parity path** (`crates/vb_compile/src/compile/mod.rs:257-269`): byte-identical copy of the same arm. Confirmed by proof-review.md:246-261.

Visibility re-exports fixed in `lib.rs:76`: `canonical_digest` and `digest_step_primitive` now re-exported from `pub use lwr::{...}`.

## GOD RULES Compliance

| GOD RULE | Status | Evidence |
|---|---|---|
| RULE 1: No hardcoded Kani shapes | PASS | All Kani harnesses use `kani::any()` for input generation within bounds; proptest strategies generate random inputs |
| RULE 2: No vacuum Verus proofs | PASS | All Kani harnesses call actual production Rust functions (`crate::lwr::canonical_digest` / `digest_step_primitive`) |
| RULE 3: TLA+ bounded correctly | N/A | No TLA+ specs for this bead (pure deterministic hash function) |
| RULE 4: Fix implementation, not harness | PASS | The implementation fix (Ask arm in digest_step_primitive) was applied to production code. Harnesses were not weakened to pass. |
| RULE 5: Scoped to blast radius | PASS | Only Ask primitive digest coverage and duplicate-site parity addressed. No broader digest changes. |

## Unresolved Blocker Assessment

| Blocker | Severity | Status |
|---|---|---|
| Missing `black-hat-review.md` | HIGH | **Blocking** — artifact required by the evidence-packaging skill's mandatory verification gate. User states "APPROVED WITH CONDITIONS" but no artifact exists. Compensating: all upstream reviews (proof-review, test-suite-review, bridge-review, formal-verification) are APPROVED. |
| Missing `machine-gate-report.md` | MEDIUM | **Non-blocking** — moon ci evidence exists in verification-ledger.jsonl (27 tasks, PASS). |
| Missing `regression-diff.md` | MEDIUM | **Non-blocking** — regression test evidence exists (245 lib tests + 126 digest tests, all PASS). |

## Artifact Inventory

| Artifact | Path | Status |
|---|---|---|
| delivery-scope.jsonl | `.beads/vb-xi2f.33/delivery-scope.jsonl` | ✓ 11 lines, valid JSONL |
| contract.md | `.beads/vb-xi2f.33/contract.md` | ✓ 142 lines |
| traceability-matrix.jsonl | `.beads/vb-xi2f.33/traceability-matrix.jsonl` | ✓ 18 rows, valid JSONL |
| proof-review.md | `.beads/vb-xi2f.33/proof-review.md` | ✓ APPROVED (314 lines) |
| proof-plan-review.md | `.beads/vb-xi2f.33/proof-plan-review.md` | ✓ APPROVED (146 lines) |
| proof-to-rust-review.md | `.beads/vb-xi2f.33/proof-to-rust-review.md` | ✓ APPROVED (287 lines) |
| test-suite-review.md | `.beads/vb-xi2f.33/test-suite-review.md` | ✓ APPROVED (202 lines) |
| formal-verification-report.md | `reports/formal-verification-report.md` | ✓ PARTIAL PASS (111 lines) |
| verification-ledger.jsonl | `verification-ledger.jsonl` | ✓ 63 lines (15 for vb-xi2f.33), valid JSONL |
| proof-evidence.md | `evidence/proof-evidence.md` | ✓ 146 lines, raw command evidence |
| proof-obligations.planned.jsonl | `.beads/vb-xi2f.33/proof-obligations.planned.jsonl` | ✓ 11 obligations, valid JSONL |
| trusted-base-ledger.jsonl | `evidence/trusted-base-ledger.jsonl` | ✓ 7 entries, valid JSONL |
| agent-invocation-ledger.jsonl | `.beads/vb-xi2f.33/agent-invocation-ledger.jsonl` | ✓ 4 entries, valid JSONL |
| waiver-candidates.jsonl | `.beads/vb-xi2f.33/waiver-candidates.jsonl` | ✓ 1 entry (WC-NONE-001) |
| proof-seeds.jsonl | `.beads/vb-xi2f.33/proof-seeds.jsonl` | ✓ 10 seeds |
| black-hat-review.md | `.beads/vb-xi2f.33/black-hat-review.md` | ✗ **MISSING** |
| machine-gate-report.md | `.beads/vb-xi2f.33/machine-gate-report.md` | ✗ **MISSING** |
| regression-diff.md | `.beads/vb-xi2f.33/regression-diff.md` | ✗ **MISSING** |
