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
| `extern_vb_storage_keys.rs` | `verification/flux/extern_vb_storage_keys.rs` | BINDING-DOC | Documents production constant bindings for vb-w6po5. |
| `vb_w6po5_storage_key_refinements.rs` | `verification/flux/vb_w6po5_storage_key_refinements.rs` | WIRED | Storage key length/order refinements; literals bind to crates/vb_storage/src/constants.rs. |

No Flux artifact in `verification/flux/` is currently WIRED except:
- `vb_w6po5_storage_key_refinements.rs` — literals bind to `crates/vb_storage/src/constants.rs` prefix/length constants.
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
- [x] `vb_w6po5_storage_key_refinements.rs` uses real refinements (`usize[N]`,
      `u8[N]`, `bool[E]`) — zero vacuous `bool[true]` specs.
- [x] `extern_vb_storage_keys.rs` documents production constant bindings.
- [x] Vacuous `vb_w6po5_storage_key_refinements.rs` removed from `crates/vb_storage/src/keys/`.
- [x] Flux check: `flux --crate-type=lib verification/flux/vb_w6po5_storage_key_refinements.rs` → 30 checked, 0 trusted, 0 ignored.