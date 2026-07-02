# Trusted Base Plan — vb-qxjgx

This document is the trust-marker ledger for the proof plan. Every
trust marker (assumed property, reduced model bound, broadened
`#[trusted]`/`extern_spec`, `kani::cover!` reachability, skipped test)
introduced by a `proof-obligation/v1` row is logged here with its
boundary proof and compensating evidence.

## 1. Assumption Ledger (per `proof-obligation/v1.assumptions`)

All seven obligations have empty `assumptions` arrays. No obligation
assumes an external property; every property is discharged by the
verifier directly. The Kani harnesses use `kani::any()` and
`kani::any_where()` for symbolic input but do NOT use `kani::assume(...)`
to short-circuit any branch.

| Obligation | Assumptions | Notes |
|---|---|---|
| PO-QXJGX-001 | [] | No `assume`. Pure bijection property. |
| PO-QXJGX-002 | [] | No `assume`. One-to-one projection property. |
| PO-QXJGX-003 | [] | No `assume`. Family admit/reject property. |
| PO-QXJGX-004 | [] | No `assume`. Parity-gate acceptance grid. |
| PO-QXJGX-005 | [] | No `assume`. Round-trip property. |
| PO-QXJGX-006 | [] | No `assume`. Counter-increment property. |
| PO-QXJGX-007 | [] | No `assume`. Compile-time constant property + literal-sync. |

## 2. Model-Bound Ledger (per `proof-obligation/v1.model_bounds`)

| Obligation | Bound | Default | Reduction? | Trust basis |
|---|---|---|---|---|
| PO-QXJGX-001 (kani) | `unwind=8, input_size=1024, mem_high=20G, mem_max=24G` | per `references/resource-governance.md` | None | Default kani budget; no reduction. |
| PO-QXJGX-002 (kani) | same | default | None | Default. |
| PO-QXJGX-003 (kani) | same | default | None | Default. |
| PO-QXJGX-004 (kani) | same | default | None | Default. |
| PO-QXJGX-005 (kani) | same | default | None | Default. |
| PO-QXJGX-006 (proptest) | `cases=10000, input_size=1024` | per `references/resource-governance.md` | None | Default proptest budget. |
| PO-QXJGX-007 (proptest) | same | default | None | Default. |

No obligation reduces the default model bound; no trusted-base row is
required for bound reduction (per Gate 8 in
`references/plan-quality-gates.md`).

## 3. Tooling and Trust-Marker Ledger (per `verifier-lane-decision/v1`)

| Lane decision | Tooling | Trust marker | Boundary proof | Compensating evidence |
|---|---|---|---|---|
| VLD-QXJGX-001 | `cargo-kani@0.67.0` (CBMC) | `kani::cover!` reachability for the new id-33 arm | The cover proves the id-33 branch is reachable in the `kani::any()` u16 sweep; the `kani::assert` is the property evidence. | The closed-set bijection property is independently asserted. |
| VLD-QXJGX-002 | same | `kani::cover!` reachability for the StepSucceeded arm | Same pattern: cover is reachability; `kani::assert` is property. | Independent bijection assertion. |
| VLD-QXJGX-003 | same | `kani::cover!` reachability for the id-33 family branch | Same pattern. | The reject path is also asserted. |
| VLD-QXJGX-004 | same | `kani::cover!` reachability for the new family id | Same pattern. | The reject paths are asserted for `MAGIC_SNAPSHOT` and `MAGIC_BLOB`. |
| VLD-QXJGX-005 | same | `kani::cover!` reachability for the legacy envelope-12 + StepSucceeded branch | The cover proves the legacy tolerance is reachable; `kani::assert` is property. | The cross-bind rejection is asserted for `SlotWrittenEvent` under envelope id 33. |
| VLD-QXJGX-006 | same | `kani::cover!` reachability for the cross-bind rejection | Same pattern. | The Ok paths are also asserted. |
| VLD-QXJGX-007 | same | `kani::cover!` reachability for the legacy decode path | Same pattern. | The canonical path is asserted. |
| VLD-QXJGX-008 | `proptest@1.5` | `prop_assume!(false)` anti-invariant for the pre-fix collapse | The anti-invariant asserts the post-fix projection does NOT match the pre-fix collapse; this is the rejection evidence. | The post-fix property is independently asserted. |
| VLD-QXJGX-009 | same | `prop_assume!(false)` anti-invariant for `SlotWritten` in step-closing positions | Same pattern. | The post-fix `StepSucceeded` assertion is independent. |
| VLD-QXJGX-010 | same | `prop_assume!(false)` anti-invariant for id-keyed counter | Same pattern. | The variant-keyed counter is independent. |
| VLD-QXJGX-011 | same | `prop_assume!(false)` anti-invariant for schema version 0/2 admitting | Same pattern. | The Ok(1) path is independent. |
| VLD-QXJGX-012 (flux-rs `blocked_tooling`) | `flux@nightly-2026-02-15` (not in workspace) | n/a | Boundary: `codec/mod.rs:184-186` comments out `pub mod flux_validation`; flux_rs crate is absent from the workspace. | Compensating evidence: PO-QXJGX-007 third proptest parses `flux_validation.rs:14,33` and asserts 33 is in both literal sets. The `tools/go-skill-v9-validate` rejects a Verus obligation without a production-binding plan; the flux obligation is `blocked_tooling` and the literal-sync is enforced by proptest, which has a non-vacuous strategy and an anti-invariant. The `tooling_acquisition_ref` is `BEAD-TOOL-FLUX-RS-INSTALL` (a separate bead to add `flux_rs` to the workspace and re-enable the module). |

## 4. Skipped-Test Ledger

This plan has NO `#[cfg_attr(miri, ignore)]` or `#[ignore]` skipped tests.
The Kani harnesses are feature-gated (compile under `cfg(kani)`), and the
proptest sources are dev-dependency gated. Neither is a runtime skip.

## 5. Compensating Evidence Cross-Reference

| Compensation target | Compensating obligation(s) | Why it covers |
|---|---|---|
| flux-rs literal-sync (POST-011) | PO-QXJGX-007 third proptest | Reads the literal source and asserts 33 is in the known set and journal-family refinement sets. |
| Default-profile proptest gap for REQ-001/002 (POST-001/002) | (none — accepted Major finding) | The kani `kani::any()` over u16 covers the full id space; a proptest would shrink the same property without finding new counterexamples. |
| Default-profile proptest gap for REQ-003/005/006/007 (POST-003/004/005/007) | (none — accepted Major finding) | Same reasoning; the kani grid is exhaustive. |
| Default-profile kani gap for REQ-008/009 (POST-008/009) | (none — accepted Major finding) | The durability matrix and recovery counters are compile-time constants; kani provides no additional evidence beyond enumerating the constant. |
| Static-scan obligations (PS-019, PS-021) | Out of scope (holzman-rust + black-hat-reviewer) | Source-lint CI gate, not a formal-verifier execution boundary. |
| Manual-QA obligations (OPEN-Q-A1/A2/A3) | Out of scope (hands-on-qa) | Manual-QA evidence files. |

## 6. Open Tooling Acquisition

`BEAD-TOOL-FLUX-RS-INSTALL` — a future bead (out of scope for vb-qxjgx) to:

1. Add `flux_rs = { version = "...", optional = true }` to
   `crates/vb_storage/Cargo.toml`.
2. Uncomment `pub mod flux_validation` at `codec/mod.rs:184-186`.
3. Pin the nightly toolchain to `nightly-2026-02-15` per
   `references/resource-governance.md`.
4. Run `bash scripts/flux-check-package.sh vb_storage` to confirm the
   flux obligation `PO-QXJGX-011-FLUX` passes.

Until this bead closes, the flux-rs lane is `blocked_tooling` and the
literal-sync is enforced by proptest.

## 7. Cross-Reference

- `proof-strategy.md` — the high-level plan and risk classification.
- `proof-coverage-matrix.md` — the requirement-clause ↔ obligation table.
- `verifier-lane-decisions.jsonl` — the 12 lane decision rows.
- `proof-obligations.planned.jsonl` — the 7 obligation rows.
- `waiver-candidates.jsonl` — zero rows; this bead has no waivers.
