# Waiver Candidates — vb-t0iw9

schema_version: waiver-candidates/v1
state: 4
bead_id: vb-t0iw9
companion_file: waiver-candidates.jsonl (intentionally empty; see below)

## Why this file is empty

Every planning obligation emitted by this plan is `behavior_affecting: true`
(`proof-obligations.planned.jsonl`, all five rows). The
`proof-planner/SKILL.md` EARS contract invariant:

> Never emit behavior-affecting waiver-candidate.

is enforced by the validator at `scripts/src/lib.rs:check_waivers` which
emits `E_BEHAVIOR_WAIVER` for any `waiver-candidate/v1` row with
`behavior_affecting: true`. Therefore no waiver row can be safely emitted
for any obligation in this plan.

The validator also rejects any waiver that uses weak vocabulary
(`"too hard"`, `"not needed"`, `"we'll add this later"`, etc.) and
requires an ISO-8601 `expiry`. The present bead has no time-bounded trust
boundary that satisfies the waiver-planning-guide rules:

- `tooling`/`proptest`/`cargo-fuzz` verifiers are available; no
  tool-availability gap.
- `model_bounds` (cases, runs_per_probe, max_len, rss_limit_mb) are
  workspace-acceptable; no resource-budget gap.
- The production-binding surface for Verus/Kani/Flux is
  `limitation_kind: surface_absent` (no production Rust); the
  `verifier-lane-decisions.jsonl` already records this as
  `applicability: not_applicable` with concrete evidence refs, which is
  the correct disposition per the SKILL.md EARS contract.
- No third-party dependency lacks an upstream spec; the only third-party
  surface (`bd v1.0.5`) is pinned via OB-001's binary capture and is
  not a waivable surface.

The 11 `E_LANE_DECISION_MISSING` major findings emitted by the
`validate-plan` self-audit for the absent default-profile verifiers
(verus/kani/flux-rs over `parse_canonicalization`/`hostile_input`/
`rejection`/`illegal_state`/`bounded_transition`) are NOT waiver
candidates; they are `not_applicable` lane decisions that the
`proof-plan-reviewer` owns at State 4b per the SKILL.md
"Handoff 1 -- To proof-plan-reviewer" clause.

## What would re-open this file

A waiver row would only be valid here if any of the following became
true:

1. A future obligation targets a third-party crate without an upstream
   spec, AND a sibling obligation verifies the same claim via a
   different verifier. (Today: no third-party surface in scope.)
2. A test cannot be modeled under cargo-fuzz within the workspace's
   resource budget, AND a paired native sanitizer run covers the claim.
   (Today: max_total_time=120 is well within budget; no carve-out
   requested by the bead prompt.)
3. A symmetry/model reduction is needed (TLC, Miri flag) and liveness
   is not claimed. (Today: no TLC, no Miri, no symmetry reduction.)
4. A trusted abstraction (`#[verifier::external_body]`, `#[trusted]`,
   `opaque`, `extern_spec`) is introduced by a Verus/Kani/Flux
   obligation, AND the boundary is justified upstream. (Today: no such
   obligation in the plan.)

If any of the above becomes true, the writer (State 5) MUST add a
`waiver-candidate/v1` row with `behavior_affecting: false`,
non-empty `boundary_proof`, non-empty `compensating_evidence`, an
ISO-8601 `expiry`, and `review_status: proposed`.

## Validation summary

- `validate-plan` non-strict: PASS (0 blockers, 11 majors — all
  reviewer-owned disposition).
- `validate-plan --strict`: 11 majors would become blockers; the
  present plan does not target strict mode. If the
  `proof-plan-reviewer` chooses strict, the reviewer owns the
  disposition for the 11 E_LANE_DECISION_MISSING majors.
