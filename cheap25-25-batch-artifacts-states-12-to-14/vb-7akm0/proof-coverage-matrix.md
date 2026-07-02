# Proof Coverage Matrix: vb-7akm0

## Proof Seed → Obligation Mapping

| Proof Seed ID | Requirement ID | Contract Clause | Verifier | Obligation IDs |
|--------------|---------------|-----------------|----------|----------------|
| PS-vb-7akm0-001 | R-vb-7akm0-001 | LS-VESTIGIAL.1 | moon-lint-src | PO-LINT-001 |
| PS-vb-7akm0-002 | R-vb-7akm0-002 | LS-VESTIGIAL.2 | moon-lint-src, cargo-check, cargo-test | PO-LINT-001, PO-COMPILE-001, PO-TEST-001 |
| PS-vb-7akm0-003 | R-vb-7akm0-003 | LS-VESTIGIAL.3 | moon-lint-src, cargo-check, cargo-test | PO-LINT-001, PO-COMPILE-001, PO-TEST-001 |
| PS-vb-7akm0-004 | R-vb-7akm0-004 | LS-VESTIGIAL.4 | moon-lint-src, cargo-check, cargo-test | PO-LINT-001, PO-COMPILE-001, PO-TEST-001 |
| PS-vb-7akm0-005..011 | R-vb-7akm0-005..011 | LS-INTERNAL.1..7 | moon-lint-src, cargo-check, cargo-test | PO-LINT-001, PO-COMPILE-001, PO-TEST-001 |
| PS-vb-7akm0-012..014 | R-vb-7akm0-012..014 | LS-TAINT.1..3 | moon-lint-src, cargo-check, cargo-test | PO-LINT-001, PO-COMPILE-001, PO-TEST-001 |
| PS-vb-7akm0-015..018 | R-vb-7akm0-015..018 | LS-SCHEMA.1..4 | moon-lint-src, cargo-check, cargo-test | PO-LINT-001, PO-COMPILE-001, PO-TEST-001 |
| PS-vb-7akm0-019 | R-vb-7akm0-019 | LS-DIAG.1 | moon-lint-src, grep-externality | PO-LINT-001, PO-EXTERN-001 |
| PS-vb-7akm0-020 | R-vb-7akm0-020 | LS-DIAG.2 | moon-lint-src, cargo-test | PO-LINT-001, PO-TEST-001 |
| PS-vb-7akm0-021 | R-vb-7akm0-021 | LS-DIAG.3 | moon-lint-src, cargo-test, grep-externality | PO-LINT-001, PO-TEST-001, PO-EXTERN-001 |
| PS-vb-7akm0-022 | R-vb-7akm0-022 | LS-REEXPORT.1 | moon-lint-src, grep-externality | PO-LINT-001, PO-EXTERN-001 |
| PS-vb-7akm0-023 | R-vb-7akm0-023 | LS-ORPHAN.1 | moon-lint-src, decision-ack | PO-LINT-001, PO-DECISION-001 |
| PS-vb-7akm0-024 | R-vb-7akm0-024 | LS-ORPHAN.2 | moon-lint-src, decision-ack, check-verus-production-binding, check-production-inner-drift, grep | PO-LINT-001, PO-DECISION-001, PO-EXTERN-001, PO-DECISION-GREP-001 |
| PS-vb-7akm0-025 | R-vb-7akm0-025 | LS-LIFECYCLE.1 | moon-lint-src, cargo-test, grep-externality | PO-LINT-001, PO-TEST-001, PO-EXTERN-001 |
| PS-vb-7akm0-026 | R-vb-7akm0-026 | LS-INVARIANT.1 | moon-lint-src | PO-LINT-001 |
| PS-vb-7akm0-027 | R-vb-7akm0-027 | LS-INVARIANT.2 | cargo-test | PO-TEST-001 |
| PS-vb-7akm0-028 | R-vb-7akm0-028 | LS-VERIFY.1 | moon-lint-src | PO-LINT-001 |
| PS-vb-7akm0-029 | R-vb-7akm0-029 | LS-VERIFY.2 | cargo-test | PO-TEST-001 |
| PS-vb-7akm0-030 | R-vb-7akm0-030 | LS-VERIFY.3 | grep-externality, check-verus-production-binding | PO-EXTERN-001 |

## Contract Clause Coverage

| Clause | Summary | Covered By | Status |
|--------|---------|-----------|--------|
| LS-VESTIGIAL.1..4 | 4 vestigial allow-removals | PO-LINT-001, PO-COMPILE-001, PO-TEST-001 | covered |
| LS-INTERNAL.1..7 | 7 gate-internal pub fn → fn narrowings | PO-LINT-001, PO-COMPILE-001, PO-TEST-001 | covered |
| LS-TAINT.1..3 | 3 taint/type/secret-leak pub fn → fn narrowings | PO-LINT-001, PO-COMPILE-001, PO-TEST-001 | covered |
| LS-SCHEMA.1..4 | 4 schema-support pub → pub(crate) narrowings | PO-LINT-001, PO-COMPILE-001, PO-TEST-001 | covered |
| LS-DIAG.1 | diag_codes.rs decision (option a or b) | PO-LINT-001, PO-EXTERN-001 | covered |
| LS-DIAG.2 | diag_convert.rs pub(super) allow-removal | PO-LINT-001, PO-TEST-001 | covered |
| LS-DIAG.3 | diag_render.rs externally-reachable allow-removal | PO-LINT-001, PO-TEST-001, PO-EXTERN-001 | covered |
| LS-REEXPORT.1 | diagnostic.rs re-export allow-removal | PO-LINT-001, PO-EXTERN-001 | covered |
| LS-ORPHAN.1 | commands_diff.rs decision-required | PO-LINT-001, PO-DECISION-001 | covered (decision-ack pre-condition) |
| LS-ORPHAN.2 | commands_incident.rs decision-required + Verus prod-binding | PO-LINT-001, PO-DECISION-001, PO-DECISION-GREP-001, PO-EXTERN-001 | covered |
| LS-LIFECYCLE.1 | lifecycle.rs externally-reachable allow-removal | PO-LINT-001, PO-TEST-001, PO-EXTERN-001 | covered |
| LS-INVARIANT.1 | every remaining pub item reachable | PO-LINT-001 | covered |
| LS-INVARIANT.2 | behavior-preserving | PO-TEST-001 | covered |
| LS-VERIFY.1 | moon run :lint-src exits 0 | PO-LINT-001 | covered |
| LS-VERIFY.2 | cargo test --workspace exits 0 | PO-TEST-001 | covered |
| LS-VERIFY.3 | check-verus-production-binding exits 0 | PO-EXTERN-001 | covered |

## Hazard → Obligation Coverage

| Hazard (from delivery-scope.jsonl risk_tags) | Obligations |
|----------------------------------------------|-------------|
| H1, H9 (lint_suppression_audit — vestigial, diag, lifecycle) | PO-LINT-001 |
| H2 (test_visibility — B/C/D sibling-module direct paths) | PO-COMPILE-001, PO-TEST-001 |
| H3 (test_visibility — D schema support) | PO-COMPILE-001, PO-TEST-001 |
| H4 (public_api — E.1 diag_codes decision) | PO-EXTERN-001 |
| H5 (public_api — E.3/F diag_render + diagnostic.rs reexport) | PO-EXTERN-001 |
| H6 (dormant_artifact + decision_required — G.1/G.2 commands_diff/commands_incident) | PO-DECISION-001 |
| H7 (production_binding_verification — G.2 IncidentReport Verus binding) | PO-DECISION-GREP-001, PO-EXTERN-001 |
| H8 (public_api — lifecycle.rs create_run_header) | PO-EXTERN-001 |
| H9, H10 (lint_suppression_audit — bead-wide invariant/verify) | PO-LINT-001, PO-TEST-001 |

## Obligation Summary

| Obligation ID | Verifier | Target | Mode | Owner State | Behavior Affecting |
|--------------|----------|--------|------|-------------|---------------------|
| PO-LINT-001 | moon-lint-src | `moon run :lint-src` (workspace) | verify-standard | 5 | false |
| PO-COMPILE-001 | cargo-check | `cargo check --workspace --all-features` | verify-standard | 5 | false |
| PO-TEST-001 | cargo-test | `cargo test --workspace` (incl. --lib and --tests) | verify-standard | 5 | false |
| PO-EXTERN-001 | grep-externality + check-verus-production-binding + check-production-inner-drift | `.evidence/grep-externality/<run_id>/<item>.txt` + raw exit codes | verify-formal-closure | 5 | false |
| PO-DECISION-001 | decision-ack | `.beads/vb-7akm0/decision-ack.md` existence + content hash | pre-condition | 4b→5 | false |
| PO-DECISION-GREP-001 | grep | `grep -R 'IncidentReport' verification/verus/production_inner/` raw output | pre-condition | 4b→5 | false |

**Total obligations:** 6 (within 4-6 budget).

## Lane Decision Summary

| Verifier | Required | Not Applicable | Seeds |
|----------|----------|----------------|-------|
| `moon-lint-src` | 1 | 0 | 25 PS rows |
| `cargo-check` | 1 | 0 | 18 PS rows (A, B, C, D) |
| `cargo-test` | 1 | 0 | 25 PS rows (all that touch test code) |
| `grep-externality` | 1 | 0 | 5 PS rows (E.1, E.3, F, lifecycle, LS-VERIFY.3) |
| `check-verus-production-binding` | 1 | 0 | 2 PS rows (G.2, LS-VERIFY.3) |
| `check-production-inner-drift` | 1 | 0 | 1 PS row (G.2) |
| `decision-ack` | 1 | 0 | 2 PS rows (G.1, G.2) |
| `verus` | 0 | 1 | 0 (binding gate covers) |
| `kani` | 0 | 1 | 0 (canonical gates unaffected) |
| `flux-rs` | 0 | 1 | 0 (no refinement types in scope) |
| `loom` | 0 | 1 | 0 (no concurrent actors) |
| `proptest` | 0 | 1 | 0 (cargo-test covers) |
| `cargo-fuzz` | 0 | 1 | 0 (no fuzz targets in scope) |
| `miri` | 0 | 1 | 0 (no unsafe code) |
| `tla-plus` | 0 | 1 (globally removed) | 0 |
| **Total** | **7 required** | **8 not_applicable** | **30 seeds** |

## Cross-Verification: All 30 Seeds Touched

| Seed Group | Count | PO-LINT-001 | PO-COMPILE-001 | PO-TEST-001 | PO-EXTERN-001 | PO-DECISION-001 | PO-DECISION-GREP-001 |
|------------|-------|-------------|----------------|-------------|---------------|-----------------|----------------------|
| PS-001..004 (vestigial A) | 4 | ✅ all | ✅ all | ✅ 002-004 | — | — | — |
| PS-005..011 (gate B) | 7 | ✅ all | ✅ all | ✅ all | — | — | — |
| PS-012..014 (taint C) | 3 | ✅ all | ✅ all | ✅ all | — | — | — |
| PS-015..018 (schema D) | 4 | ✅ all | ✅ all | ✅ all | — | — | — |
| PS-019 (diag_codes E.1) | 1 | ✅ | — | — | ✅ | — | — |
| PS-020 (diag_convert E.2) | 1 | ✅ | — | ✅ | — | — | — |
| PS-021 (diag_render E.3) | 1 | ✅ | — | ✅ | ✅ | — | — |
| PS-022 (diagnostic.rs F) | 1 | ✅ | — | — | ✅ | — | — |
| PS-023 (commands_diff G.1) | 1 | ✅ | — | — | — | ✅ | — |
| PS-024 (commands_incident G.2) | 1 | ✅ | — | — | ✅ (check-verus + drift) | ✅ | ✅ |
| PS-025 (lifecycle G.touch) | 1 | ✅ | — | ✅ | ✅ | — | — |
| PS-026 (LS-INVARIANT.1) | 1 | ✅ | — | — | — | — | — |
| PS-027 (LS-INVARIANT.2) | 1 | — | — | ✅ | — | — | — |
| PS-028 (LS-VERIFY.1) | 1 | ✅ | — | — | — | — | — |
| PS-029 (LS-VERIFY.2) | 1 | — | — | ✅ | — | — | — |
| PS-030 (LS-VERIFY.3) | 1 | — | — | — | ✅ (check-verus) | — | — |
| **Total** | **30** | **25** | **18** | **18** | **6** | **2** | **1** |

Every proof seed has at least one obligation touch-point. No silent omissions.