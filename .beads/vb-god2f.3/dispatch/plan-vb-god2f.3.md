# Plan — vb-god2f.3: replan HVR-PO-STORAGE-001 production-bound Verus closure

| Field | Value |
|---|---|
| Bead | `vb-god2f.3` (P0 IN_PROGRESS, parent `vb-god2f`) |
| Planner invocation | `proof-planner/vb-god2f.3@2026-07-01` |
| State | 4 (planned) — proof-planner output, awaiting proof-plan-reviewer |
| Companion review | `.beads/vb-god2f/dispatch/black-hat.md` |
| Verifier lane | **Verus** (default Rust behavior lane for pure/core invariants) |
| Production-binding | **STRONG preferred; WEAK_MIRROR acceptable with drift-gate header**. No `EXPLICITLY_ALLOWED` / `ALLOWED_EXCEPTIONS` / `OFFLOAD` mechanism is permitted. |
| Re-derivation note | Parent `vb-god2f` claimed this file existed at this path on 2026-06-30. It did not. This is the re-derived version produced via the proof-planner → proof-plan-reviewer cycle per `vb-240tk`. |

## 1. Goal

Repair or replan `HVR-PO-STORAGE-001` so it is a **production-bound
Verus-compatible bridge or approved replacement lane**, with the
explicit black-hat-handoff retirement of the mirror-model file
`crates/vb_storage/verification/verus/recovery_types_spec.rs`.

The plan must preserve the **acceptance criterion from `bd show vb-god2f.3`**
verbatim: *"repair/replan/re-review a production-bound Verus-compatible
bridge or approved replacement lane; no mirror-model/vacuum proof;
formal-verifier runs exact reviewed command before any PASS claim."*

The blocker is also restated in the parent `bd show vb-god2f`
NOTES: *"HVR-PO-STORAGE-001 remains BLOCKED_TOOLING and explicitly
not authorized for formal-verifier PASS by
.beads/vb-god2f/proof-review-r2.md."*

## 2. Obligation in scope

| Obligation ID | Crate | Surface | State at rerun |
|---|---|---|---|
| `HVR-PO-STORAGE-001` | `vb_storage` | Scalar classification / recovery kernel (MRWE5) | `BLOCKED_TOOLING` |

The replan target is a **production-bound Verus bridge for the MRWE5
scalar kernel** (per `bd show vb-god2f.3` NOTES: *"Replan scope:
production-bound Verus bridge for MRWE5 scalar kernel; no production
source changes; forbids mirror model and assume/axiom/external_body;
suggests extern_spec! projection for enum derives"*).

Out-of-scope clauses that were already retired to
`HVR-PO-STORAGE-003/005` obligations (per `bd show vb-god2f.3`
NOTES) and are **not** reopened by this plan:

- `REPLAY-*` (replay replay-set membership)
- `UNSUPPORTED-UNION` (untagged union classification)
- `DIGEST-*` (envelope digest stability)

## 3. Verifier lane decision

| Lane | Required? | Rationale |
|---|---|---|
| **Verus** | **required** | `HVR-PO-STORAGE-001` is a pure Rust invariant (scalar classification / recovery kernel). Default Rust-local pure/core invariant lane per `verification-lane-policy.md`. |
| Kani | not_applicable (parallel) | Could cross-validate the same property under bounded state, but the original obligation is Verus and re-routing to Kani would change the obligation semantics. Available as a follow-up in a child bead if Verus closure is itself blocked on tooling. |
| Flux | not_applicable | Refinements are a follow-up if the Verus spec surfaces a length/index relationship that needs an illegal-state refinement. |
| proptest | not_applicable | Pressure test, not proof-of-invariant. |
| cargo-fuzz | not_applicable | Not parser/codec territory. |

## 4. Production-binding strategy (MANDATORY)

Per the `proof-plan-reviewer` Production Binding Plan Validation
section, **every Verus `proof-obligation/v1` row MUST declare a
`production_binding` field** with one of `STRONG`, `WEAK_MIRROR`,
`WEAK_EXTERN`. There is no `EXPLICITLY_ALLOWED`. There is no
allowlist. There is no offload rationale.

For `HVR-PO-STORAGE-001`, the priority order is:

### Preferred: STRONG (direct #[path] to production)

```rust
#[path = "../../crates/vb_storage/src/classification/scalar.rs"]
mod production;

verus! {
    spec fn scalar_classification_matches(s: Scalar) -> bool { ... }
    proof fn classify_preserves_invariant(s: Scalar)
        requires production::classify_pre(s), // assume_specification bridge
        ensures  scalar_classification_matches(s) { ... }
}
```

Required fields on the obligation row:

```yaml
production_binding:
  mechanism: STRONG
  production_path: crates/vb_storage/src/classification/scalar.rs
  production_lines: "1-95"  # exact lines, no zero-width ranges
  assume_specification_targets:
    - production::classify_pre
    - production::classify_post
  exec_wrapper_required: true
  drift_detection: build-time
```

### Acceptable: WEAK_MIRROR (production_inner/ + drift-gate)

If direct `#[path]` to production is blocked by tooling (the
historical blocker for `HVR-PO-STORAGE-001`), the plan MUST:

1. Place a verbatim mirror at
   `verification/verus/production_inner/scalar_classification_production.rs`
   with a drift-gate header that names the production path.
2. Cite `scripts/check-production-inner-drift.sh` as the drift gate
   (`drift_threshold: zero`, `drift_gate_script: scripts/check-production-inner-drift.sh`).
3. The `mirror_path` MUST exist on disk before this plan is approved.

### Forbidden

- `WEAK_EXTERN` to another `extern_*.rs` that does not itself bind to
  production or mirror (chain-of-mirrors is still vacuum).
- `ALLOWED_EXCEPTIONS` / `OFFLOAD` / `EXPLICITLY_ALLOWED` entries in
  `scripts/check-verus-production-binding.sh`. Parent black-hat
  handoff cited the recovery_types_spec.rs mirror; that is **not**
  a precedent for adding more entries — it is a **retirement**
  obligation (see §5).

## 5. Mirror-model retirement (BLACK-HAT HANDOFF — REQUIRED)

Per parent `vb-god2f` NOTES black-hat handoff note (1):

> *"vb-god2f.3 execution MUST retire
> crates/vb_storage/verification/verus/recovery_types_spec.rs
> mirror-model file before close (delete or annotate +
> ALLOWED_EXCEPTIONS)."*

This plan executes the retirement as follows. **Either**:

1. **Delete** `crates/vb_storage/verification/verus/recovery_types_spec.rs`
   entirely, **or**
2. **Annotate** the file's top-of-file comment block with a
   `vacuum` marker that names this plan and this bead, and add an
   entry to `scripts/ignored-fallible-results.allow` (or the
   matching allowlist for that surface) with: `PO = HVR-PO-STORAGE-001`,
   `owner = holzman-rust`, `expiry = 2026-09-30`, `follow_up = <bead-id-of-follow-up>`,
   `reason = "mirror-model retired under vb-god2f.3 re-derivation"`.

The retirement MUST be committed before this plan's formal-verifier
PASS is recorded. The commit hash MUST be cited in the
formal-verifier ledger row.

## 6. GOD-RULE compliance markers

| Rule | Plan posture |
|---|---|
| **GOD-RULE 1** — No hardcoded Kani shapes | N/A — this is a Verus plan. |
| **GOD-RULE 2** — No vacuum Verus proofs | `HVR-PO-STORAGE-001` MUST declare `production_binding` (STRONG preferred; WEAK_MIRROR acceptable with drift-gate). The mirror-model file is being retired, not revived. No `assume`/`axiom`/`admit`/`external_body` permitted in executable proof code. |
| **GOD-RULE 3** — No unbounded TLA+ math | N/A — no TLA+ in scope. |
| **GOD-RULE 4** — No loop oscillations | If a Verus counterexample surfaces a real bug in production, the proof-writer MUST patch production, not the spec. |
| **GOD-RULE 5** — No blind verification mutations | Stay inside `crates/vb_storage/src/classification/` blast radius. Do not fleet-wide `cargo verus`. Use `bash scripts/verify-verus.sh` for registry-driven obligations. |

## 7. Bridge planning (proof-to-implementation handoff)

The plan MUST emit a `proof-to-implementation-input.md` artifact
that names:

- The exact `crates/vb_storage/src/classification/scalar.rs` lines
  being bound.
- The `extern_spec!` projections used (per `bd show vb-god2f.3`
  NOTES suggestion for enum derives).
- The executable witness harness (`#[kani::*]` or unit test) that
  exercises the bridge and is the source of `expected_evidence`.
- The exact `cargo verus` invocation (workdir, flags, expected
  exit) that formal-verifier will run **without modification** to
  record PASS.

`proof-to-implementation-input.md` is downstream of this plan; this
plan commits to its presence as an acceptance criterion (see §8).

## 8. Acceptance criteria for `vb-god2f.3`

1. `HVR-PO-STORAGE-001` carries a `production_binding` field on its
   `proof-obligation/v1` row that conforms to one of the three
   mechanisms (STRONG / WEAK_MIRROR / WEAK_EXTERN) per
   `proof-plan-reviewer` validation rules.
2. If STRONG is chosen: the obligation's `production_path` resolves
   to a real file with non-zero `production_lines`. If WEAK_MIRROR
   is chosen: `mirror_path` and `drift_gate_script` both exist on
   disk and `drift_threshold: zero`.
3. `crates/vb_storage/verification/verus/recovery_types_spec.rs`
   is either deleted or annotated with the `vacuum` marker per §5
   *before* formal-verifier records PASS.
4. `proof-to-implementation-input.md` exists and names concrete
   source lines + exact `cargo verus` invocation.
5. `proof-plan-reviewer` record at
   `.beads/vb-god2f/dispatch/black-hat.md` shows
   `STATUS: APPROVED` for this plan (no `blocker` findings).
6. `bash scripts/check-verus-production-binding.sh` exits 0 on the
   patched spec, with raw output captured as evidence.
7. If `bash scripts/check-verus-production-binding.sh` cannot run
   in this isolated workspace (toolchain gating), the proof-writer
   MUST cite the last-green run's raw log path AND commit a
   re-runnable wrapper script for re-validation when the gate is
   next green.

## 9. Cross-references

- Parent black-hat handoff: parent `vb-god2f` NOTES (paragraph
  beginning *"2026-06-30 PLANNING COMPLETE"*).
- Source blocker: `.beads/vb-god2f/proof-review-r2.md` (HVR-PO-STORAGE-001
  `BLOCKED_TOOLING`).
- Sibling dependency: `vb-q6xm8` (`vb_storage` vacuum Verus spec
  retirement — same surface, may unblock the tooling gate for this
  obligation).
- Existing replan scope: `bd show vb-god2f.3` NOTES (domain-model,
  type-contracts, workflow-model, error-taxonomy, boundary-map,
  hazard-analysis, contract, proof-seeds.jsonl (13 seeds),
  traceability-matrix.jsonl (37 rows)).

## 10. Out of scope (per `vb-240tk`)

- No Verus spec source code is written in this plan file.
- No `vb-god2f.3` bead-record edit (status stays `IN_PROGRESS`).
- The mirror-model file retirement (`recovery_types_spec.rs`)
  **is** a source edit, but it is required by the parent black-hat
  handoff and is committed as part of `vb-god2f.3` execution, not
  this plan. This plan only *commits* to the retirement as an
  acceptance gate.
- No fuzz/proptest/Kani harnesses for `HVR-PO-STORAGE-001`
  (belongs to a child bead if a Kani cross-check is desired).