# Flux/Kani/Verus Verification Hygiene Notes (vb-f1qe7, vb-8sa4i, vb-frskm, vb-xy2aw, vb-8voze, vb-g6xgs, vb-w7nn9, vb-y3deg)

**STATUS:** This document records the verifier-hygiene findings from the
2026-06-30 audit and the resolution applied per bead. Each section states
the rule, the violation pattern, and the remediation taken in this pass.
No production code was changed; this bead is documentation-and-gate-only.

## 1. Flux: remove broad `trusted` and tautological refinement wrappers (vb-f1qe7)

**Rule:** A Flux refinement must NOT be marked `#[trusted]` unless the
artifact is bound to production code via `#[path = ".../crates/..."]` or
the `production_inner/` mirror. Tautological `ensures true` or
`requires true` wrappers must be removed because they verify nothing.

**Pattern:** Some Flux specs added `#[trusted]` to skip verification or used
`#[flux::trusted]` as a stand-in for proper binding. Others wrapped a
trivial `ensures(true)` around production code to claim a pass.

**Resolution:** See `verification/flux/WIRING_STATUS.md` (vb-e5kxn). Every
Flux artifact is marked SCOPED-ONLY until bound. Tautological wrappers
remain in their files but are explicitly labeled as such; they MUST NOT
appear in any "PASS" claim without a real refinement target.

## 2. Kani: bind harnesses to production implementations (vb-8sa4i)

**Rule:** A Kani harness MUST exercise the production implementation,
either via `#[path = ".../crates/..."]` or by inlining production logic.
A harness that constructs a hand-written shadow struct and asserts
properties about it does NOT prove the production struct satisfies those
properties.

**Pattern:** Some harnesses defined a local copy of `RunFrame` or
`ActionTicket`, populated it with `kani::any()`, and asserted `bounded ==
true`. The harness passed but proved only the shadow struct's invariants,
not the production struct's.

**Resolution:** See `verification/kani/PRODUCTION_BINDING_NOTES.md`.
Production-binding is a hard precondition for any Kani claim. Shadow-model
harnesses are downgraded to "structural exercise" and cannot be cited as
production proof.

## 3. Kani: replace `cover!`-only and `assert!(true)` harnesses (vb-frskm)

**Rule:** A Kani harness must include at least one non-trivial assertion
that the production code is exercised against. A harness whose only
operation is `kani::cover!(true)` or `assert!(true)` proves nothing.

**Pattern:** Several harnesses reached into production code, exercised
a single path, and called `kani::cover!(true)` at the end. These
"passes" recorded coverage of one branch but no property assertion.

**Resolution:** Such harnesses are flagged in
`verification/kani/PRODUCTION_BINDING_NOTES.md` and must be rewritten
before they can be cited as proof of any obligation.

## 4. Kani: replace hardcoded `WorkflowFrame` and `Recovery` shapes (vb-xy2aw)

**Rule:** Hardcoding `WorkflowFrame { ... }` with constant fields defeats
the bounded model checker. A harness that always builds the same frame
proves only that one frame is safe, not the family.

**Pattern:** A handful of harnesses built `WorkflowFrame { slot_count: 16,
symbol_count: 4, ... }` and asserted safety. They do not exercise the
slot-layout, symbol-count, or input-shape boundaries.

**Resolution:** `kani::any()` generators are required. Hardcoded-shape
harnesses are downgraded and listed for rewrite.

## 5. Verus: remove tautological `choose` proof predicates (vb-8voze)

**Rule:** `choose` predicates that resolve to `true` without constraining
the witness are tautologies. A `proof fn` whose only step is
`choose(|x: T| true)` proves no useful property.

**Pattern:** A few Verus specs used `choose` to satisfy `exists` without
tying the witness to any concrete invariant.

**Resolution:** The `verification/verus/PRODUCTION_BINDING_NOTES.md`
file flags these and lists the audit-trail. Future Verus artifacts must
not add new tautological `choose` predicates; the
`scripts/check-verus-production-binding.sh` gate is the source of truth.

## 6. Verus: make `moon ci` depend on Verus binding and drift gates (vb-g6xgs)

**Rule:** `moon ci` MUST run `bash scripts/check-verus-production-binding.sh`
and `bash scripts/check-production-inner-drift.sh` and FAIL on any
production-binding violation or drift. Today those gates exist but are
not wired into `moon ci`.

**Pattern:** Auditors verified that the binding-gate scripts existed and
ran, but `moon ci` did not invoke them, so a fresh CI run could pass
without checking production binding.

**Resolution:** See `MOON_CI_VERIFIER_GATES.md` in this directory. The
follow-up implementation lands the wiring in `.moon/tasks/all.yml` so
`moon ci` blocks on binding/drift failures.

## 7. Verus: repair or retire untracked `choose_proofs.vr` (vb-w7nn9)

**Rule:** Every `.vr` and `.rs` proof artifact in `verification/verus/`
MUST be reachable from a registered obligation in
`contracts/proof_obligations.yaml`. Untracked artifacts cannot be
audited and cannot be cited.

**Pattern:** `choose_proofs.vr` (and similar files) lived in
`verification/verus/` but had no corresponding row in
`contracts/proof_obligations.yaml`. They were exercised by ad-hoc runs
and reported passes that could not be tied to any obligation.

**Resolution:** See `VERUS_UNTRACKED_ARTIFACTS.md`. The untracked file
is flagged for retirement or registration; this bead documents the
decision but does not delete historical artifacts (deletion would
destroy evidence).

## 8. Verus: include `.vr` files in production-binding audit (vb-y3deg)

**Rule:** The Verus production-binding audit
(`scripts/check-verus-production-binding.sh`) MUST inspect `.vr` files
in addition to `.rs` files, because Verus proof files use `.vr` as the
standard extension.

**Pattern:** The current binding script only matched `*.rs`. Verus
artifacts written in `.vr` form (e.g. `choose_proofs.vr`) were invisible
to the audit and could escape binding violations.

**Resolution:** See `VERUS_BINDING_AUDIT_VR_COVERAGE.md`. The follow-up
bead patches `scripts/check-verus-production-binding.sh` to walk both
`.rs` and `.vr` files.

## Acceptance Criteria (this bead group)

- [x] Each of the 8 hygiene findings is recorded with rule, pattern, and
      resolution.
- [x] No production code is changed by this bead group.
- [x] Every follow-up points at a concrete file in the workspace or a
      concrete script under `scripts/`.
- [x] Companion `WIRING_STATUS.md`, `PRODUCTION_BINDING_NOTES.md`,
      `MOON_CI_VERIFIER_GATES.md`, `VERUS_UNTRACKED_ARTIFACTS.md`, and
      `VERUS_BINDING_AUDIT_VR_COVERAGE.md` are created alongside this
      document to keep the audit trail retrievable.