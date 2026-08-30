# Flux Artifact Wiring Status (vb-e5kxn)

**STATUS:** Most Flux artifacts in `verification/flux/` are **NOT**
production-wired. They are scoped-only refinements that exercise hand-written
shadow models against the same external surface as production, but they do
not bind to production code via `#[path = ".../crates/..."]` and cannot be
claimed as evidence of production safety without that binding.

This file records the wiring decision for every Flux artifact so future
agents and CI do not interpret scoped-only files as production-bound
evidence.

## Wiring Categories

- **WIRED**: artifact binds to production via `#[path = ".../crates/..."]`
  or `#[path = ".../production_inner/..."]` + drift-gate header.
- **SCOPED-ONLY**: artifact is a refinement of a shadow model, NOT bound to
  production. May be retained for audit or research, but CANNOT be cited
  as production evidence.
- **RETIRE**: artifact is duplicated by another bound artifact or has no
  remaining obligation; slated for removal in a follow-up bead.

## Inventory

| Artifact | Path | Status | Reason |
|---|---|---|---|
| `choose_refinements.flux` | `verification/flux/choose_refinements.flux` | SCOPED-ONLY | Single-file Flux spec; not `#[path]`-bound to `crates/vb_core/src/expression/...`. |
| `step_budget.rs` | `verification/flux/step_budget.rs` | SCOPED-ONLY | Refinement of a hand-written shadow `StepBudget`; not `#[path]`-bound to `crates/vb_core/src/budget/step.rs`. |
| `flux_sequence.rs` | `verification/flux/flux_sequence.rs` | SCOPED-ONLY | Standalone demo — sequence bounds, contiguity, step ordering, replay bounds. Moved from `crates/vb_storage/src/types/flux_sequence.rs` (vb-hvxpe). |
| `flux_replay.rs` | `verification/flux/flux_replay.rs` | SCOPED-ONLY | Standalone demo — replay contiguity, step ordering, tail bounds, attempt filtering. Moved from `crates/vb_storage/src/recovery/flux_replay.rs` (vb-hvxpe). |
| `vb-vzcuf-PS-001.rs` | `verification/flux/vb-vzcuf-PS-001.rs` | SCOPED-ONLY | PS-001 obligation; shadow model only. |
| `vb-vzcuf-PS-002.rs` | `verification/flux/vb-vzcuf-PS-002.rs` | SCOPED-ONLY | PS-002 obligation; shadow model only. |
| `vb-vzcuf-PS-003.rs` | `verification/flux/vb-vzcuf-PS-003.rs` | SCOPED-ONLY | PS-003 obligation; shadow model only. |
| `vb-vzcuf-PS-004.rs` | `verification/flux/vb-vzcuf-PS-004.rs` | SCOPED-ONLY | PS-004 obligation; shadow model only. |
| `vb-vzcuf-PS-005.rs` | `verification/flux/vb-vzcuf-PS-005.rs` | SCOPED-ONLY | PS-005 obligation; shadow model only. |
| `vb-vzcuf-PS-006.rs` | `verification/flux/vb-vzcuf-PS-006.rs` | SCOPED-ONLY | PS-006 obligation; shadow model only. |
| `vb-vzcuf-PS-007.rs` | `verification/flux/vb-vzcuf-PS-007.rs` | SCOPED-ONLY | PS-007 obligation; shadow model only. |
| `vb-vzcuf-PS-008.rs` | `verification/flux/vb-vzcuf-PS-008.rs` | SCOPED-ONLY | PS-008 obligation; shadow model only. |
| `vb-vzcuf-PS-009.rs` | `verification/flux/vb-vzcuf-PS-009.rs` | SCOPED-ONLY | PS-009 obligation; shadow model only. |
| `vb_rpch_flux_r8.rs` | `verification/flux/vb_rpch_flux_r8.rs` | SCOPED-ONLY | Recovery/hydration shadow model. |
| `vb_rpch_flux_r9.rs` | `verification/flux/vb_rpch_flux_r9.rs` | SCOPED-ONLY | Recovery/hydration shadow model. |
| `vb_xi2f_compile_source.rs` | `verification/flux/vb_xi2f_compile_source.rs` | SCOPED-ONLY | Compile-source shadow model. |
| `vb_xi2f_try_from_parts.rs` | `verification/flux/vb_xi2f_try_from_parts.rs` | SCOPED-ONLY | `try_from_parts` shadow model. |
| `vb_compile/` | `verification/flux/vb_compile/` | SCOPED-ONLY | Directory of shadow-model refinements. |

No Flux artifact in `verification/flux/` is currently WIRED. The
production-storage Flux path is disabled at the package-feature level
(see `crates/vb_storage/src/flux.rs` if present, otherwise confirm via
`bash scripts/check-flux-production-binding.sh`).

## Required Follow-Up (out of scope for this bead)

To turn any SCOPED-ONLY artifact into WIRED:

1. Copy the production source tree into `verification/flux/production_inner/`
   (or pin a `#[path = ".../crates/..."]` to the actual production path).
2. Add a drift-gate header comment with the production revision SHA.
3. Run `bash scripts/check-flux-production-binding.sh` to confirm the
   binding is detected by the gate.
4. Run `cargo flux -p <package> --message-format human` and capture the
   raw log; attach to the bead.
5. Only then may the artifact be cited as production-bound evidence.

Until each step above is taken, every Flux artifact in
`verification/flux/` is SCOPED-ONLY and must be labeled accordingly in any
report or evidence bundle.

## Acceptance Criteria (this bead)

- [x] Every Flux artifact inventoried above with `WIRED`, `SCOPED-ONLY`,
       or `RETIRE` status.
- [x] No `WIRED` claim made without a `#[path]` binding or drift-gate
       header in the artifact itself.
- [x] This file is the canonical source for "is Flux artifact X bound to
       production?" lookups until the binding follow-up lands.
- [x] `flux_sequence.rs` and `flux_replay.rs` downgraded from production
       source to non-closure SCOPED-ONLY artifacts in `verification/flux/`.