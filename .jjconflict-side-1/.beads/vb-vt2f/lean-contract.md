# vb-vt2f Theorem Kernel Projection

## Boundary

- TLA+-owned temporal model: active and required for lifecycle and strict-admission clauses; both models have PASS evidence in `.beads/vb-vt2f/proof-evidence.md`.
- Verus-owned Rust core: candidate waiver only (`WAIVER-VERUS-VT2F-002`), dependent on accepted TLA PASS, owner-authorized Kani projection-kernel PASS, BDD/catalog/CI evidence, and explicit projection-equivalence risk acceptance.
- Kani-owned proof kernels: owner-authorized projection kernels in `crates/vb_runtime/src/kani_vt2f_runtime_facade.rs` and `crates/vb_runtime/src/kani_vt2f_shard_lower_semantics.rs`; these are not Lean theorem kernels.
- Theorem-owned kernel: none for this bead.
- Runtime shell: public `Runtime` facade behavior is exercised by BDD scenarios and bounded by TLA/Kani/review obligations; concrete runtime-shell equivalence is not discharged by Lean.
- External systems excluded from theorem proof: storage engine, wall clock, OS scheduling, async/runtime shell, filesystem, and public facade wiring.

## Theorem-Owned Clauses

- None.

## Non-Applicability Rationale

Lean/Aeneas/Hax remain non-applicable because vb-vt2f has no tiny theorem-sized algebraic kernel beyond the already scoped TLA+ temporal models and Kani projection kernels. The owner-authorized Kani projections are executable bounded model-checking targets, not theorem-assistant extraction targets.

Projection equivalence between the Kani kernels and concrete runtime/shard/admission/ask code is not hidden here: it is a trusted manual review/waiver obligation (`PROJ-EQ-VT2F-001`) with expiry and non-reuse caveats. Lean does not prove that equivalence in this bead.

## Reopen Trigger

If later states introduce a new abstract transition kernel for run lifecycle, trace-drain algebra, admission-policy lattice, ticket-to-run refinement, or Kani-projection equivalence that cannot be expressed adequately in Verus or Kani, add a theorem-kernel plan before implementation lands.

## Waiver

- `WAIVER-LEAN-VT2F-001`: owner_state=3; reason=`no tiny theorem kernel in vb-vt2f scope`; limitation=`does not waive TLA+, Kani projection-kernel, projection-equivalence review, BDD, catalog, CI, or Verus waiver-review obligations`; expiry=`before any new algebraic/refinement kernel or proof-assistant extraction target is introduced`; compensating_evidence=`TLA lifecycle PASS, TLA strict-admission PASS, owner-authorized Kani projection-kernel PASS, PROJ-EQ-VT2F-001 review/waiver, BDD nextest, catalog regression, static public-surface audit, and CI/deferred-global evidence`.

## Traceability Repair

- Lean/Aeneas/Hax remains non-applicable.
- Direct PRE/POST/INV/ERR traceability is provided by BDD, review-artifact, TLA+, Kani projection-kernel, projection-equivalence, Verus-waiver-review, and CI obligations rather than theorem-kernel obligations.
