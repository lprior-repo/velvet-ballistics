# Verifier Lane Matrix: vb-7akm0

## Lane Status Overview

| Lane | Verifier | Applicability | Script/Wrapper | Tool Available | Script Exists | Required Decision | Blockers |
|------|----------|---------------|----------------|----------------|---------------|-------------------|----------|
| 1 | `moon-lint-src` | required | `moon run :lint-src` (`.moon/tasks/all.yml:46-62`) | `cargo clippy` (workspace) | Yes | LD-vb-7akm0-001 | None — gate exists, command is single-shot |
| 2 | `cargo-check` | required | `cargo check --workspace --all-features` | `cargo` (workspace) | Yes (built-in) | LD-vb-7akm0-002 | None |
| 3 | `cargo-test` | required | `cargo test --workspace` | `cargo test` (workspace) | Yes (built-in) | LD-vb-7akm0-003 | None |
| 4 | `grep-externality` | required | `grep -R 'vb_validate::diag::diag_codes::CODE_' .` (and 4 similar per item) | `grep` | Yes (built-in) | LD-vb-7akm0-004 | Must run BEFORE ApplyTreatment (externality analysis depends on pre-change symbol surface) |
| 5 | `check-verus-production-binding` | required | `bash scripts/check-verus-production-binding.sh` | `bash` + scripts | Yes | LD-vb-7akm0-005 | None — gate exists |
| 6 | `check-production-inner-drift` | required | `bash scripts/check-production-inner-drift.sh` | `bash` + scripts | Yes | LD-vb-7akm0-006 | None — gate exists |
| 7 | `decision-ack` | required | `cat .beads/vb-7akm0/decision-ack.md` (artifact existence check) | filesystem | N/A | LD-vb-7akm0-007 | Implementation owner must create decision-ack.md before ApplyTreatment |
| 8 | `grep` (incident pre-condition) | required | `grep -R 'IncidentReport' verification/verus/production_inner/` | `grep` | Yes (built-in) | LD-vb-7akm0-008 | Must run BEFORE ApplyTreatment for category G.2 |
| 9 | `verus` | not_applicable | `bash scripts/verify-verus.sh` | `verus` | Yes | LD-vb-7akm0-009 | Vis-refactor only; no spec/proof fn changes; binding gate (Lane 5) covers health |
| 10 | `kani` | not_applicable | `bash scripts/kani-list.sh <package>` | `cargo-kani` | Yes | LD-vb-7akm0-010 | No new harnesses; canonical gates consume vb_validate::gates::* (not the touched duplicates) |
| 11 | `flux-rs` | not_applicable | `bash scripts/flux-check-package.sh <package>` | `cargo-flux` | Yes | LD-vb-7akm0-011 | No refinement types in touched files |
| 12 | `loom` | not_applicable | `cargo test --cfg loom -p <package>` | `loom` (dev-dep) | N/A | LD-vb-7akm0-012 | No concurrent actors introduced |
| 13 | `proptest` | not_applicable | `cargo test --package <pkg> proptests` | `cargo test` | N/A | LD-vb-7akm0-013 | No new property tests; existing cargo-test (Lane 3) covers |
| 14 | `cargo-fuzz` | not_applicable | `cargo fuzz list` | `cargo-fuzz` | Yes | LD-vb-7akm0-014 | No fuzz targets in scope; touched files are not parser/compiler code |
| 15 | `miri` | not_applicable | `cargo miri test` | `cargo-miri` | N/A | LD-vb-7akm0-015 | No unsafe code in scope (Holzman Rust) |
| 16 | `tla-plus` | not_applicable | N/A (globally removed) | N/A | N/A | LD-vb-7akm0-016 | TLA+ globally removed; no temporal behavior |

## Lane × Proof Seed Coverage

| Proof Seed | moon-lint-src | cargo-check | cargo-test | grep-externality | check-verus-prod-bind | check-prod-inner-drift | decision-ack |
|-----------|---------------|-------------|------------|------------------|----------------------|------------------------|-------------|
| PS-vb-7akm0-001..004 (vestigial A) | ✅ | ✅ | ✅ (A.2-4) | — | — | — | — |
| PS-vb-7akm0-005..014 (B, C) | ✅ | ✅ | ✅ | — | — | — | — |
| PS-vb-7akm0-015..018 (D schema) | ✅ | ✅ | ✅ | — | — | — | — |
| PS-vb-7akm0-019 (E.1 diag_codes) | ✅ | — | — | ✅ | — | — | — |
| PS-vb-7akm0-020 (E.2 diag_convert) | ✅ | — | ✅ | — | — | — | — |
| PS-vb-7akm0-021 (E.3 diag_render) | ✅ | — | ✅ | ✅ | — | — | — |
| PS-vb-7akm0-022 (F diagnostic.rs) | ✅ | — | — | ✅ | — | — | — |
| PS-vb-7akm0-023 (G.1 commands_diff) | ✅ | — | — | — | — | — | ✅ |
| PS-vb-7akm0-024 (G.2 commands_incident) | ✅ | — | — | — | ✅ | ✅ | ✅ (grep) |
| PS-vb-7akm0-025 (G lifecycle.rs) | ✅ | — | ✅ | ✅ | — | — | — |
| PS-vb-7akm0-026 (LS-INVARIANT.1) | ✅ | — | — | — | — | — | — |
| PS-vb-7akm0-027 (LS-INVARIANT.2) | — | — | ✅ | — | — | — | — |
| PS-vb-7akm0-028 (LS-VERIFY.1) | ✅ | — | — | — | — | — | — |
| PS-vb-7akm0-029 (LS-VERIFY.2) | — | — | ✅ | — | — | — | — |
| PS-vb-7akm0-030 (LS-VERIFY.3) | — | — | — | — | ✅ | — | — |

## Lane × Requirement Coverage

| Requirement | moon-lint-src | cargo-check | cargo-test | grep-externality | check-verus-prod-bind | check-prod-inner-drift | decision-ack |
|------------|---------------|-------------|------------|------------------|----------------------|------------------------|-------------|
| R-vb-7akm0-001..004 (A vestigial) | ✅ | ✅ | ✅ (2-4) | — | — | — | — |
| R-vb-7akm0-005..014 (B, C) | ✅ | ✅ | ✅ | — | — | — | — |
| R-vb-7akm0-015..018 (D schema) | ✅ | ✅ | ✅ | — | — | — | — |
| R-vb-7akm0-019 (E.1 diag_codes) | ✅ | — | — | ✅ | — | — | — |
| R-vb-7akm0-020..021 (E.2, E.3) | ✅ | — | ✅ | ✅ (E.3) | — | — | — |
| R-vb-7akm0-022 (F) | ✅ | — | — | ✅ | — | — | — |
| R-vb-7akm0-023 (G.1) | ✅ | — | — | — | — | — | ✅ |
| R-vb-7akm0-024 (G.2) | ✅ | — | — | — | ✅ | ✅ | ✅ (grep) |
| R-vb-7akm0-025 (G lifecycle) | ✅ | — | ✅ | ✅ | — | — | — |
| R-vb-7akm0-026..027 (INVARIANTs) | ✅ (1) | — | ✅ (2) | — | — | — | — |
| R-vb-7akm0-028..030 (VERIFYs) | ✅ (28) | — | ✅ (29) | ✅ (30 via check-verus) | ✅ | — | — |

## Lane Decision Summary

| Verifier | Required | Not Applicable | Seeds Covered |
|----------|----------|----------------|---------------|
| `moon-lint-src` | 1 | 0 | 25 PS rows (all attribute-removal + LS-VERIFY.1) |
| `cargo-check` | 1 | 0 | 18 PS rows (A, B, C, D compile-check subset) |
| `cargo-test` | 1 | 0 | 25 PS rows (all that touch test code) |
| `grep-externality` | 1 | 0 | 5 PS rows (E.1, E.3, F, lifecycle, LS-VERIFY.3) |
| `check-verus-production-binding` | 1 | 0 | 2 PS rows (G.2 pre-condition, LS-VERIFY.3) |
| `check-production-inner-drift` | 1 | 0 | 1 PS row (G.2 mirror independence) |
| `decision-ack` | 1 | 0 | 2 PS rows (G.1, G.2 pre-condition) |
| `verus` | 0 | 1 | none (binding covered by Lane 5) |
| `kani` | 0 | 1 | none (canonical gates unaffected) |
| `flux-rs` | 0 | 1 | none (no refinement types in scope) |
| `loom` | 0 | 1 | none (no concurrent actors) |
| `proptest` | 0 | 1 | none (cargo-test covers) |
| `cargo-fuzz` | 0 | 1 | none (no fuzz targets in scope) |
| `miri` | 0 | 1 | none (no unsafe code) |
| `tla-plus` | 0 | 1 (globally removed) | none (no temporal behavior) |
| **Total** | **7 required decisions** | **8 not_applicable** | **30 seeds** |

## Production-Binding Gate (Lane 5)

Lane 5 (`check-verus-production-binding`) is the existing God-Rule-2 gate. It MUST exit 0 after category G changes because:

- The bead does NOT modify `verification/verus/extern_*.rs` or `verification/verus/production_inner/*.rs`.
- The bead does NOT introduce new `#[path = ...]` bindings or break existing ones.
- `commands_incident::IncidentReport` is local to `crates/vb_cli/src/commands_incident.rs`; the Verus proofs bind via `production::Kind::IncidentReport` enum variant (delivery-scope.jsonl row 32).
- Even if category G.2 narrows `IncidentReport` to `pub(crate)`, the production_inner mirror (`production_inner/vb_ahfl_bounds_production_inner.rs`) is unchanged and re-compiles unchanged.

If `check-verus-production-binding.sh` exits non-zero after the changes, **the implementation owner MUST roll back category G** and consult the proof-reviewer before proceeding.

## Decision-Ack Pre-Condition (Lane 7)

Lane 7 (`decision-ack`) is a structural pre-condition for category G. The implementation owner MUST create `.beads/vb-7akm0/decision-ack.md` with one of:
- `Decision: RetireOrphanTest`
- `Decision: RegisterOrphanTest`

before invoking `ApplyTreatment` for `commands_diff.rs` and `commands_incident.rs`. Default recommendation is RetireOrphanTest (per codebase-map.md §"Open Questions" recommendation 1 and contract.md §2.7 LS-ORPHAN.1 default). If neither decision is recorded, `ApplyTreatment` MUST hard-fail with exit code 64 (EX_USAGE).

## Forbidden: Externally Reachable Item Allow-Removal

The bead's spec explicitly forbids removing `#[allow(unreachable_pub)]` from items that are externally reachable (decisions G). Lane 4 (`grep-externality`) is the structural guard: before any category E or F or G.lifecycle change, the implementation owner MUST run grep to confirm each `pub` item remaining IS externally reachable, and capture the evidence in `.evidence/grep-externality/<run_id>/<item>.txt`. If any grep returns 0 hits for an item the bead intends to keep `pub`, that item MUST be narrowed to `pub(crate)` (and the allow deleted).