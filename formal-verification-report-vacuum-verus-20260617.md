# Formal Verification Report: Vacuum Verus Findings

**Date:** 2026-06-18T03:56:16Z  
**Bead:** `vb-dzibx`  
**Agent role:** formal-verifier  
**Status:** `FAIL_LOCAL` — recent Verus artifacts verify, but are not production-bound.

## Scope

Truth-serum/proof-reviewer audits were run against the recent Verus deliverables for:

- `vb_proof_kernels`
- `vb_runtime`
- `vb_queue_semantics`
- `vb_storage`
- `vb_boundary_inventory`
- `vb_ipc`
- `vb_yaml`
- `vb_ajc40_flux`

## Closure Decision

No PASS may be recorded for these artifacts until proof-writer/implementation work replaces the vacuum models with production-bound proof obligations and proof-reviewer accepts them.

## Repair Update — 2026-06-18

Proof-writer/verus repair agents were invoked for the affected lanes. The repair strategy was conservative: artifacts that could not be made production-bound without a new proof plan were retired/downgraded rather than laundered as proofs.

Scoped proof-review result:

- Review artifact: `.beads/vb-dzibx/proof-review.md`
- Findings artifact: `.beads/vb-dzibx/proof-findings.jsonl`
- Review status: `APPROVED` for the **retirement/downgrade objective only**

This does **not** approve L4 production proof closure. It approves that the previous vacuum artifacts are no longer represented as production deductive evidence.

Active-context verification run after repair:

- `contracts/proof_obligations.yaml` parsed successfully.
- Retired/syntax-only Verus targets returned `0 verified, 0 errors` where applicable.
- `crates/vb_ipc/src/verification/verus/vb_5iebh.rs` returned `9 verified, 0 errors` as local non-proof sanity evidence.
- Repaired crate builds passed for `vb_yaml`, `vb_boundary_inventory`, `vb_ipc`, `vb_storage`, `vb_runtime`, `vb_queue_semantics`, `vb_proof_kernels`, and `vb_ajc40_flux` positive feature build.
- Scoped trust-marker scan over repaired Verus targets found `0` executable matches for `assume`, `assume_specification`, `external_body`, `external`, or `axiom`.

## Production-Bound Replacement Update — 2026-06-18

After the retirement/downgrade pass, production-bound replacement proof lanes were planned, reviewed, repaired, executed, and reviewed.

Plan review:

- Final plan review: `.beads/vb-dzibx/replacement-proof-plan-review-r4.md`
- Status: `APPROVED`
- Caveat: cargo-fuzz lanes for YAML/IPC are explicitly deferred and block full fuzz/L4 closure.

Replacement proof review:

- Proof review: `.beads/vb-dzibx/replacement-proof-review.md`
- Findings: `.beads/vb-dzibx/replacement-proof-findings.jsonl`
- Status: `APPROVED` for scoped production-bound replacement obligations.

Approved scoped replacement obligations:

- `RPO-YAML-001`..`RPO-YAML-004`
- `RPO-QUEUE-001`..`RPO-QUEUE-003`
- `RPO-IPC-001`..`RPO-IPC-003`
- `RPO-AJC40-001`..`RPO-AJC40-004`

Deferred / not full closure:

- `RPO-YAML-FUZZ-001`
- `RPO-YAML-FUZZ-002`
- `RPO-IPC-FUZZ-001`

Active-context command evidence included:

- YAML Kani/proptest replacement commands: passed.
- Queue Kani/Flux/proptest replacement commands: passed; Flux is helper-smoke only because package output still reports trusted functions.
- IPC Kani/proptest replacement commands: passed.
- AJC40 Flux/proptest/generator-parity/Verus/Kani replacement commands: passed.

This update changes the status from “vacuum proofs retired only” to “scoped production-bound replacements approved,” while preserving the explicit non-closure for deferred fuzz/full-L4 lanes.

## Formal Classification

| Crate / Area | Classification | Reason |
| --- | --- | --- |
| `vb_proof_kernels` | `FAIL_LOCAL` | Parallel proof-kernel types and tautological specs are not bound to production; additional undisclosed Verus/Kani failures were reported by proof-reviewer. |
| `vb_runtime` | `FAIL_LOCAL` | Action-completion specs use local primitive models and wrong production paths; no `extern_spec`, `assume_specification`, or production `requires`/`ensures`. |
| `vb_queue_semantics` | `FAIL_LOCAL` | Queue specs/bridges use mirror types and local models; no production bindings; registry mismatch. |
| `vb_storage` | `FAIL_LOCAL` | `classification_specs.rs` verifies locally but includes phantom constants, deleted cfg-verus bridges, and no production crate wiring. |
| `vb_boundary_inventory` | `FAIL_LOCAL` | Validation spec uses mirror types; several predicates contradict production behavior; no production binding. |
| `vb_ipc` | `FAIL_LOCAL` | IPC spec uses wrong magic/header shape and mirror types; PO-BOUNDED-024 is explicitly tautological; no production binding. |
| `vb_yaml` | `FAIL_LOCAL` | Specs are not wired into the crate, are not registry-backed, and one spec contradicts production mixed-case behavior. |
| `vb_ajc40_flux` | `FAIL_LOCAL` | 73 Verus checks are local-model checks; binding lemmas do not call production exec functions; stale provenance references missing source/generator files. |

## Repair Bead Filed

Created bead:

- `vb-dzibx` — **P0 verify: replace vacuum Verus proofs with production-bound obligations across audited crates**

Acceptance requires:

1. Every retained Verus proof has an auditable production binding (`extern_spec`, approved `assume_specification` with trusted-base ledger entry, or production `requires`/`ensures`).
2. Mirror types are removed or bridged to production types with checked obligations.
3. Known spec/production mismatches from proof-reviewer reports are fixed.
4. `contracts/proof_obligations.yaml` stops marking vacuum artifacts as `deductively_verified`.
5. proof-reviewer returns PASS before formal-verifier records any PASS.

## Evidence Locations

Primary proof-review artifacts include:

- `.beads/proof-review-vb_yaml-20260617/`
- `.beads/proof-review-vb-boundary-inventory/`
- `.beads/proof-review-vb-ipc-5iebh-20260617/`
- `.beads/proof-review-classification-specs-20260617/`
- `.beads/vb-kzz99-review/`
- `.beads/vb-r37is/`
- `.beads/vb-pr6mg/`
- `/home/lewis/.beads/vb-vb3ip/`

## Formal-Verifier Boundary

This report does **not** claim the proofs are repaired. The repair work requires proof-writer / implementation agents. This formal-verifier pass only prevents false closure by recording the current artifacts as failed until production-bound repairs land and are reviewed.
