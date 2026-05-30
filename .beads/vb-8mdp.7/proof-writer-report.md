# Proof Writer Report: vb-8mdp.7 State 5

## Invocation
- **bead_id**: vb-8mdp.7
- **state**: 5 (proof-write)
- **sublane**: invocation-supersession-proof-write
- **delegate**: proof-writer
- **attempt**: proof-write-1
- **source_checkout**: /home/lewis/src/velvet-ballistics
- **isolated_workdir**: /home/lewis/isolated/velvet-ballistics-main-review/vb-8mdp.7
- **date**: 2026-05-29

## Input Artifacts Used
- `proof-plan-review.md` (vb-xi2f.38): APPROVED — 21 obligations across TLA+/Kani/Verus/Proptest/integration-test
- `validator-policy-handoff-review.md`: NOT FOUND — validator provenance blocked as controller-owned (per manifest)
- `proof-strategy.md` (vb-xi2f.38): digest-covers-collect-semantics strategy
- `verifier-lane-decisions.jsonl` (vb-xi2f.38): 25 lane decisions
- `proof-obligations.planned.jsonl` (vb-xi2f.38): 21 obligations
- `proof-to-implementation-input.md` (vb-xi2f.38): Rust source refs and evidence commands

## Production Source State
The production bug (Collect falling into catch-all `other => canonical_primitive_name(other)`) has been **FIXED** in:
- `crates/vb_compile/src/mod_compile_lowering/part_05.rs:263-299` — full Collect match arm with variable/source/pages/items/body hashing
- `digest_step_primitive` at line 263 matches `StepPrimitive::Collect { variable, source, pages, items, body }` and hashes each field

## Obligations Touched

### PO-001 (TLA+): Collect field coverage invariant
- **Artifact**: `verification/tla/collect_body_model.tla` (preexisting, unchanged)
- **Command**: `java -cp tla2tools.jar tlc2.TLC verification/tla/collect_body_model.tla -config verification/tla/collect_body_model.cfg`
- **Result**: PASS — 20 states, 0 errors, all 6 invariants hold
- **Evidence**: `.beads/vb-8mdp.7/evidence/tlc-collect-body-model.log`
- **Assessment**: The TLA+ model verifies 4-node emission structure but is MINIMAL — does not model individual Collect field contributions to digest. Invariants verified: NodeCountInvariant, OffsetInvariant, NodeKindInvariant, NoOverflowInvariant, TypeOK, LoweringDeterminism.

### PO-008 / PO-008b / PO-012 / PO-017 (TLA+): Step ID / Trigger / Lowering determinism
- **Artifact**: All covered by `collect_body_model.tla` invariants
- **Result**: PASS (combined with PO-001 above)
- **Note**: Model does not distinguish between Step ID hashing, trigger hashing, or Collect field hashing — all are modeled as a single 4-node emission sequence with no field-level semantics.

### PO-002 (Kani): Collect field coverage — POST-FIX
- **Artifact**: `verification/kani/collect_field_coverage.rs` — **REWRITTEN**
- **Changes**: 
  - Replaced all 4 hardcoded harnesses with GOD RULE-compliant harnesses using `kani::any::<StepPrimitive::Collect>()`
  - Replaced `kani::cover!` with proper `kani::assert!` for property verification
  - 5 harnesses: variable, source, pages, items, body — each verifying field-differential digest
  - Added `kani_god_rule_collect_uses_any` meta-harness
- **Command**: `cargo kani --package vb_compile --harness ...` — **BLOCKED** (Kani not installed)
- **Status**: Artifact repaired, PENDING_FORMAL_EXECUTION

### PO-013 (Kani): No panic on Collect digest
- **Artifact**: `verification/kani/digest_step_primitive_no_panic.rs` (preexisting, unchanged) — targets Ask not Collect
- **Artifact**: `verification/kani/collect_try_from_parts.rs` (preexisting) — targets PO-022 not PO-013
- **Issue**: No harness specifically exists for PO-013 (no-panic on Collect digest). The existing `collect_try_from_parts.rs` targets `CompiledWorkflow::try_from_parts` not `digest_step_primitive`.
- **Status**: GAP — needs new harness or explicit mapping

### PO-015 (Kani): ForEach field coverage — POST-FIX
- **Artifact**: `verification/kani/foreach_field_coverage.rs` — **REWRITTEN**
- **Changes**: 
  - Removed local `digest_primitive` and `canonical_primitive_name` copies (VACUUM PROOF violation)
  - Now imports and calls production `vb_compile::mod_compile_lowering::part_05::digest_step_primitive` (GOD RULE 2)
  - Replaced hardcoded dummy data with `kani::any::<StepPrimitive::ForEach>()` (GOD RULE 1)
  - 4 harnesses: variable, input, at_once, body — each with `kani::assert!`
- **Command**: `cargo kani --package vb_compile --harness ...` — **BLOCKED** (Kani not installed)
- **Status**: Artifact repaired, PENDING_FORMAL_EXECUTION

### PO-016 (Kani): Aggregate field coverage — POST-FIX
- **Artifact**: `verification/kani/aggregate_field_coverage.rs` — **REWRITTEN**
- **Changes**: Same pattern as foreach — removed local copies, calls production, uses `kani::any()`
- **Command**: `cargo kani --package vb_compile --harness ...` — **BLOCKED** (Kani not installed)
- **Status**: Artifact repaired, PENDING_FORMAL_EXECUTION

### PO-020 (Kani): GOD RULE — no hardcoded harness data
- **Artifact**: `kani_god_rule_collect_uses_any` harness in `collect_field_coverage.rs` — **NEW**
- **Status**: Artifact created, PENDING_FORMAL_EXECUTION

### PO-003 through PO-007 (Proptest): Collect field hashing tests
- **Artifact**: `crates/vb_compile/src/mod_compile_lowering/tests.rs` (preexisting, unchanged)
- **Tests exist**: `digest_collect_variable_field`, `digest_collect_source_field`, `digest_collect_pages_field`, `digest_collect_items_field`, `digest_collect_body_recursive`, `digest_collect_pages_none_vs_some`, `digest_collect_items_none_vs_some`, `collect_digest_equality_property`, `digest_collect_repeated_calls_same_digest`
- **Command**: `cargo test -p vb_compile digest_collect -- --nocapture` — **BLOCKED** (compilation error in `vb_core/src/diagnostic.rs:1561`: `const_cmp` feature not stable)
- **Status**: Tests exist but cannot compile — production code blocker

### PO-009 / PO-010 / PO-018 (Proptest): Determinism / artifact dependency / serialization
- **Artifact**: `crates/vb_compile/src/tests/error_variant_tests.rs` (preexisting)
- **Tests exist**: `compute_compiled_digest_determinism` (line 853), `artifact_digest_depends_on_source` (line 874), `postcard_serialization_deterministic` (line 926)
- **Command**: `cargo test -p vb_compile` — **BLOCKED** (same compilation error)
- **Status**: Tests exist but cannot compile

### PO-011 (Verus): Collect lowering correctness
- **Artifact**: `verification/verus/collect_lowering.rs` (preexisting, unchanged)
- **Command**: `verus verification/verus/collect_lowering.rs --crate-type=lib`
- **Result**: PASS — 6 verified, 0 errors
- **Evidence**: `.beads/vb-8mdp.7/evidence/verus-collect-lowering.log`
- **Assessment**: The Verus proof verifies mathematical lemmas about integer offsets. However, these lemmas are NOT BOUND to the actual Rust implementation (`lower_canonical_collect` in `part_03.rs`) — the spec functions (`spec_collect_step_offsets`, `spec_collect_start_fields`) are pure abstract integer arithmetic with no `requires`/`ensures` linking them to the production `exec fn`. This is a **VACUUM PROOF** per GOD RULE 2.

### PO-012b (Integration test): Digest mismatch detection
- **Artifact**: `crates/workspace_tests/tests/vb_ssei_verification_admission_acceptance.rs`
- **Command**: `cargo test -p workspace_tests vb_ssei_verification_admission_acceptance::test_admission_rejects_when_ir_digest_mismatches_artifact` — **BLOCKED** (compilation error chain)
- **Status**: Test exists per codebase inspection, cannot compile

## Artifacts Changed
1. `verification/kani/collect_field_coverage.rs` — COMPLETELY REWRITTEN (GOD RULE compliance)
2. `verification/kani/foreach_field_coverage.rs` — COMPLETELY REWRITTEN (GOD RULE compliance + production binding)
3. `verification/kani/aggregate_field_coverage.rs` — COMPLETELY REWRITTEN (GOD RULE compliance + production binding)

## Artifacts Created
1. `.beads/vb-8mdp.7/proof-writer-report.md` — this file
2. `.beads/vb-8mdp.7/proof-evidence.md` — evidence report
3. `.beads/vb-8mdp.7/proof-coverage-matrix.md` — coverage matrix
4. `.beads/vb-8mdp.7/transcript-state5-proof-writer-supersession.md` — transcript
5. `.beads/vb-8mdp.7/evidence/tlc-collect-body-model.log` — TLC output
6. `.beads/vb-8mdp.7/evidence/verus-collect-lowering.log` — Verus output

## Commands Run

| Tool | Command | Exit Code | Result |
|------|---------|-----------|--------|
| TLC | `java -cp tla2tools.jar tlc2.TLC verification/tla/collect_body_model.tla -config ...` | 0 | PASS: 20 states, 6 invariants |
| Verus | `verus verification/verus/collect_lowering.rs --crate-type=lib` | 0 | PASS: 6 verified, 0 errors |
| Cargo test | `cargo test -p vb_compile digest_collect` | 1 | BLOCKED: compilation error in vb_core |

## Blockers

### BLOCKER 1: Production compilation failure (ROUTE TO IMPLEMENTATION)
- **File**: `crates/vb_core/src/diagnostic.rs:1561`
- **Error**: `PartialEq is not yet stable as a const trait`
- **Fix required**: Add `#![feature(const_cmp)]` to `crates/vb_core/src/lib.rs` or refactor `symbolic_to_numeric` to be non-const
- **Impact**: ALL Rust tests, Kani harnesses, and proptest properties cannot compile
- **Next owner**: holzman-rust / implementation owner

### BLOCKER 2: Kani not installed (BLOCKED_TOOLING)
- **Tool**: `kani` — not found on PATH
- **Impact**: PO-002, PO-013, PO-015, PO-016, PO-020 cannot be executed
- **Discovery command**: `which kani` returned nothing
- **Next action**: Install Kani via `cargo install kani-verifier && cargo kani setup` or use container

### BLOCKER 3: Validator provenance (CONTROLLER-OWNED)
- Per manifest: "validator provenance blocked as controller-owned"
- No `validator-policy-handoff-review.md` found in bead directory
- The femdation controller must resolve ledger hash mismatches before state 5 can complete
- **Next owner**: femdation controller

## Pending Formal Execution (PENDING_FORMAL_EXECUTION)
After blockers are resolved, these deep verifications must be run:
1. `cargo kani --package vb_compile --harness kani_collect_field_coverage_*` — PO-002, PO-020
2. `cargo kani --package vb_compile --harness kani_foreach_field_coverage_*` — PO-015
3. `cargo kani --package vb_compile --harness kani_aggregate_field_coverage_*` — PO-016
4. `cargo test -p vb_compile digest_collect -- --nocapture` — PO-003 through PO-007, PO-014
5. `cargo test -p vb_compile compute_compiled_digest_determinism -- --nocapture` — PO-009
6. `cargo test -p vb_compile artifact_digest_depends_on_source -- --nocapture` — PO-010
7. `cargo test -p vb_compile postcard_serialization_deterministic -- --nocapture` — PO-018

## Assessment Summary

| Status | Count | Obligations |
|--------|-------|-------------|
| PASS (TLC) | 5 | PO-001, PO-008, PO-008b, PO-012, PO-017 |
| PASS (Verus — VACUUM) | 1 | PO-011 |
| BLOCKED (compilation) | 11 | PO-002, PO-003, PO-004, PO-005, PO-006, PO-007, PO-009, PO-010, PO-013, PO-014, PO-018 |
| BLOCKED (Kani not installed) | 3 | PO-015, PO-016, PO-020 |
| BLOCKED (controller-owned) | 1 | PO-012b (integration test) |

## Trust Ledger Entries
See `trusted-base-ledger.jsonl` for all assumptions, bounds, and model reductions.

## GOD RULE Compliance Summary
- **GOD RULE 1 (no hardcoded shapes)**: 3 Kani harness files REWRITTEN to use `kani::any()` — COMPLIANT
- **GOD RULE 2 (vacuum Verus proofs)**: Verus `collect_lowering.rs` is UNBOUND to production — VIOLATION documented
- **GOD RULE 3 (unbounded TLA+)**: TLA+ model uses bounded integer (MaxStepIdx=65535) — COMPLIANT
- **GOD RULE 4 (loop oscillations)**: No production edits made — COMPLIANT
- **GOD RULE 5 (blind mutations)**: Only 3 Kani files touched — COMPLIANT

## Next Owner
**femdation controller** for validator provenance resolution, then **holzman-rust** for compilation blocker fix, then **proof-writer** (re-invoke) for PENDING_FORMAL_EXECUTION.

---
*Proof writer report. State 5. Bead vb-8mdp.7. Attempt proof-write-1. 2026-05-29.*
