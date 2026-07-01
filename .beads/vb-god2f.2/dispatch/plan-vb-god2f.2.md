# Plan — vb-god2f.2: bound vb-god2f Kani timeout lanes

| Field | Value |
|---|---|
| Bead | `vb-god2f.2` (P0 IN_PROGRESS, parent `vb-god2f`) |
| Planner invocation | `proof-planner/vb-god2f.2@2026-07-01` |
| State | 4 (planned) — proof-planner output, awaiting proof-plan-reviewer |
| Companion review | `.beads/vb-god2f/dispatch/black-hat.md` |
| Verifier lane | **Kani** (default Rust behavior lane per `verification-lane-policy.md`) |
| Production-binding | **N/A — Kani obligations do not carry `production_binding`; production-binding is a Verus-only mechanism.** Kani harnesses must instead satisfy GOD-RULE 1 (no hardcoded shapes) and bind to production source via `#[path]` if they require the production struct definition. |
| Re-derivation note | Parent `vb-god2f` claimed this file existed at this path on 2026-06-30. It did not. This plan is the re-derived version produced via the proof-planner → proof-plan-reviewer cycle per `vb-240tk`. |

## 1. Goal

Re-close HVR-PO-BI-001 and HVR-PO-CORE-004 — both Kani obligations that
left the `formal-verifier` rerun on **20260619T115530Z** as
`non-closing state-space/tool timeout` — by repairing the Kani harness
**without weakening the property** and **without hardcoding input shapes**.

The plan must preserve the **acceptance criterion from `bd show vb-god2f.2`**
verbatim: *"split/bound/review harnesses without weakening claims or
hardcoding shapes; proof-reviewer approves; formal-verifier reruns exact
or re-approved obligations to PASS or accepted non-closure."*

## 2. Obligations in scope (from `bd show vb-god2f.2`)

| Obligation ID | Crate | Surface | Failure mode on rerun |
|---|---|---|---|
| `HVR-PO-BI-001` | `vb_boundary_inventory` | Boundary validation invariant | State-space / tool timeout |
| `HVR-PO-CORE-004` | `vb_core` | Core engine invariant | State-space / tool timeout |

Raw logs referenced by the bead body:
`.evidence/vb-god2f/formal-runs/20260619T115530Z/logs/HVR-PO-BI-001.log`
and `.evidence/vb-god2f/formal-runs/20260619T115530Z/logs/HVR-PO-CORE-004.log`.
The full original failure surface is in
`.beads/vb-god2f/formal-verification-report.md`.

## 3. Verifier lane decision

| Lane | Required? | Rationale |
|---|---|---|
| Verus | **not_applicable** | Kani-rerun repair; obligation semantics are bounded-state panic/overflow/index risk — not pure/core invariant territory. No `proof-obligation/v1` row uses `verifier: verus`. Non-applicability evidence: this plan plus the parent `vb-god2f.2` description (which constrains the lane to Kani). |
| **Kani** | **required** | Default Rust behavior lane for panic/overflow/index risk per `verification-lane-policy.md`. Both obligations originated as Kani. The rerun failure was timeout, not wrong-lane. |
| Flux | not_applicable (initial) | Refinements may be a follow-up if Kani repair needs an illegal-state refinement; out of scope for this plan. |
| proptest | not_applicable | Obligations are *bounded-state proofs of existing properties*, not pressure tests of behaviour space. A proptest companion is acceptable evidence but is not the closure lane. |
| cargo-fuzz | not_applicable | Both obligations are pure Rust invariants; not parser/codec/IPC fuzzing territory. |

## 4. Kani harness repair strategy (no claim weakening)

Three repair strategies are admissible. The proof-writer MUST pick at
least one and justify the choice in `proof-plan-findings.jsonl`.
**Strategies that hardcode shapes, fix a constant, or remove an
assertion are prohibited by GOD-RULE 1.**

### Strategy A — Bound (preferred for `HVR-PO-CORE-004`)

1. Identify the unbounded symbolic input (`kani::any::<T>()` for the
   type that drove state-space explosion).
2. Introduce `kani::assume(...)` predicates that bind the input to
   the documented `ResourceContract::DEFAULT` envelope:
   - `len <= ResourceContract::max_yaml_bytes` for byte buffers,
   - `depth <= ResourceContract::max_expr_depth` for expression
     trees,
   - `arity <= ResourceContract::max_step_arity` for step tuples.
3. The bound MUST come from the contract, not from a magic number.
4. Re-run with the same `--output-format=regular --jobs N` baseline.
5. Expect `SUCCESS` (verifier reached a fixed point inside the bound).

### Strategy B — Split (preferred for `HVR-PO-BI-001`)

1. Decompose the property into a chain of lemmas, each proved under
   a smaller harness that exercises one transition at a time.
2. The top-level harness becomes a thin orchestrator that asserts
   the lemma chain implies the original property.
3. Lemmas live next to the harness (`#[cfg(kani)] mod lemmas;`) and
   each is invoked from the top-level harness with concrete witness
   types that exercise the boundary path.

### Strategy C — Scope-down (last resort)

1. Tighten the harness precondition so it targets the **invariant**
   rather than the full pre-state.
2. Forbidden: silently deleting a property assertion.
3. Forbidden: replacing `assert!(...)` with `kani::cover(...)`.
4. Required: keep all pre-existing `assert_*!` calls; the new
   harness is a *subset* of the original, with the remaining cases
   relegated to a downstream follow-up bead (file as a child of
   `vb-god2f.2` if any are out of scope).

### Repair gate

After Strategy A/B/C is applied, the proof-writer MUST:

- Confirm `cargo kani --harness <name>` exits 0 with raw success log
  captured under `.evidence/vb-god2f/formal-runs/<new-ts>/logs/`.
- Confirm `bash scripts/check-panic-surface.sh` still passes (no new
  panics introduced).
- Confirm `bash scripts/forbidden-scan.sh` does not regress
  (no `unwrap`/`expect` introduced to satisfy a Kani postcondition).
- Re-confirm `cargo fmt --check`, `cargo clippy --all-targets
  -- -D warnings`, `cargo build` all pass on the patched harness.

## 5. Production-binding strategy

Kani obligations do not declare `production_binding` (that field is
reserved for Verus per `proof-plan-reviewer` rejection criteria).
However, Kani harnesses that symbolise production types MUST
either:

1. Import the production type via `#[path =
   "../crates/vb_<crate>/src/<module>.rs"]` **and** cite the
   `path::symbol` form in the obligation row, **or**
2. Use `kani::Arbitrary` derived on a small harness-local stub that
   mirrors the production type's invariants (with a header comment
   naming the production source path it shadows).

GOD-RULE 1 forbids a hand-rolled enum/struct that is not tied to the
production type at all. Hand-rolled stubs are permitted only when
they have a `kani::Arbitrary` impl + a header that names the
production path.

## 6. GOD-RULE compliance markers

| Rule | Plan posture |
|---|---|
| **GOD-RULE 1** — No hardcoded Kani shapes | Strategy A/B/C above all require `kani::any()` or `kani::Arbitrary` on production-bounded inputs. Cover/scope-down is not allowed to substitute a constant. |
| **GOD-RULE 2** — No vacuum Verus proofs | Not applicable — this is a Kani plan. If a Verus lane is later proposed in a child bead, it MUST carry `production_binding` per `proof-plan-reviewer`. |
| **GOD-RULE 3** — No unbounded TLA+ math | Not applicable — no TLA+. |
| **GOD-RULE 4** — No loop oscillations | If a Kani counterexample surfaces a real bug in production, the proof-writer MUST patch production, not the harness. `HVR-PO-BI-001` and `HVR-PO-CORE-004` have non-closing timeouts, not counterexamples — but the rule is restated for the proof-writer. |
| **GOD-RULE 5** — No blind verification mutations | The repair MUST stay inside the two harness files for `HVR-PO-BI-001` and `HVR-PO-CORE-004`. No fleet-wide `cargo kani` rerun. Use `bash scripts/kani-list.sh <package>` to enumerate, never `cargo kani list --format json`. |

## 7. Acceptance criteria for `vb-god2f.2`

1. Each of `HVR-PO-BI-001` and `HVR-PO-CORE-004` has either:
   a. A `SUCCESS` Kani log captured at
      `.evidence/vb-god2f/formal-runs/<new-ts>/logs/<PO>.log`
      (raw `cargo kani --harness <name>` output, exit 0), **or**
   b. An `accepted non-closure` waiver row in
      `proof-obligations.planned.jsonl` whose
      `non_applicability_evidence_refs` cites the specific
      strategy-A/B/C reason + the proof-reviewer's acceptance row
      from `verifier-lane-review.jsonl`.
2. No property assertion in either harness was deleted or replaced
   with `cover!`. proof-reviewer must grep for that.
3. No new `unwrap`/`expect`/panic path was introduced (proof-reviewer
   must re-run `scripts/forbidden-scan.sh`).
4. Kani harness files still carry a header naming the production
   source they bind to, with a `kani::Arbitrary` impl (or
   `#[path = "..."]` for the production type).
5. proof-plan-reviewer record at
   `.beads/vb-god2f/dispatch/black-hat.md` shows
   `STATUS: APPROVED` for this plan (no `blocker` findings).

## 8. Cross-references

- Parent bead NOTES: `.beads/vb-god2f/...` (parent beads directory
  in the Dolt tracker; no on-disk directory yet).
- Parent acceptance criterion #3 (closure evidence on disk per
  master §60 / AGENTS.md): this plan + the black-hat review + the
  raw rerun logs are the three artifacts that make that criterion
  satisfiable.
- Original rerun failure: `.evidence/vb-god2f/formal-runs/20260619T115530Z/logs/`
- Sibling blockers (not addressed here): `vb-ujujr`
  (`vb_core` Kani compile break — must close before
  `HVR-PO-CORE-004` can rerun) and `vb-q6xm8` (vb_storage vacuum
  Verus spec retirement — affects `vb-god2f.3`, not this plan).

## 9. Out of scope (per `vb-240tk`)

- No proof artifacts (no Kani harness code) are written in this
  plan file. The proof-writer writes them downstream.
- No `vb-god2f.2` bead-record edit (status stays `IN_PROGRESS`).
- No source patches (`crates/*`) by this planner.
- No `vb_storage/verification/verus/recovery_types_spec.rs`
  retirement (that belongs to `vb-god2f.3`).