# Proof Plan Review Input: vb-scxh

## Review Request

Review the repaired State 4 proof plan for `vb-scxh`. This is a recovery/evidence-integrity bead. Reject the plan if it asks State 4 to write production code, tests, proof code, harnesses, models, specs, dependencies, or CI config.

## Skill and Scope Basis

- `/home/lewis/.claude/skills/proof-planner/SKILL.md` lines 15-24 require planning-only proof strategy, traceability, explicit waiver rows, and the JSONL schema used here.
- `/home/lewis/.claude/skills/proof-planner/SKILL.md` lines 31-35 prohibit proof-planner from writing proof code, tests, production code, harnesses, models, specs, dependencies, or CI config.
- `/home/lewis/.claude/skills/proof-planner/SKILL.md` lines 41-54 require scoped discovery and blocked-discovery recording.
- `/home/lewis/.claude/skills/proof-planner/SKILL.md` lines 56-70 prohibit invented pass results and require exact paths, commands, assumptions, model bounds, and skipped-lane waiver rows.

## Discovery Summary For Reviewer

- Workdir check: `pwd -P` returned `/home/lewis/src/vb-scxh`.
- Required repair inputs were non-empty: `STATE.md`, `contract.md`, `domain-model-review.md`, `tla-spec.md`, `lean-contract.md`, `verification-layers.md`, `proof-obligations.jsonl`, `traceability-matrix.jsonl`, `contract-repair-report.md`, `proof-findings.jsonl`, `proof-review.md`, and `contract-verification-review.md`.
- Scoped risk-trigger and verifier-trigger searches over `.beads/vb-scxh` ran and found the repaired evidence-integrity, TLA, waiver, BD, safety-anchor, CI, mutation, scope, and Truth Serum markers.
- No State 4 discovery command was blocked.

## Required Review Questions

- Does `proof-obligations.planned.jsonl` include all 33 IDs from current `proof-obligations.jsonl`?
- Does every planned row include the required schema fields from the proof-planner skill?
- Are `.beads/vb-scxh/tla/ScxhRecovery.tla` and `.beads/vb-scxh/tla/ScxhRecovery.cfg` the only authoritative TLA target/config paths unless State 5 explicitly moves/repairs and reruns exact moved commands?
- Does `TLA-SCXH-002` require non-tautological laundering coverage by modeling a subagent-only required evidence candidate or attempted acceptance path?
- Does the plan keep safety bundle/bookmark rows as `status:"planned"` while preserving downstream `failure_classification:"BLOCK_LOCAL"` until raw verification passes?
- Does the plan require exact 12 false closures and raw BD reopened/linked statuses rather than accepting truncated output or prose?
- Does the plan require green CI raw evidence markers: `moon ci`, PASS, 19 completed tasks, 8276/8276 tests passed, runtime marker, and artifact path or fresh rerun?
- Does the plan preserve mutation `FAIL_UNVIABLE` / `DEFERRED` as non-pass?
- Does the plan preserve generated parity deferral to `vb-gvmt` / `vb-qi37.10` and prevent its use as `vb-scxh` closure proof?
- Are skipped Verus, Lean/Aeneas/Hax, Kani, Flux, Loom/Shuttle, Miri, proptest/fuzz, performance, API, and release-provenance lanes represented as machine-readable waiver rows?
- Does the final Truth Serum decision stay blocked unless all raw evidence lanes pass or have approved waivers?

## Planned Required Rows

- `PATH-SCXH-001`: workspace and write-scope guard.
- `ART-SCXH-001`: State and referenced artifact presence.
- `BD-SCXH-001`: exact 12 false-closure raw BD audit.
- `BD-SCXH-002`: raw-BD-only source audit for state/link claims.
- `SAFETY-SCXH-001`: safety bundle/bookmark verification, planned with downstream `failure_classification:"BLOCK_LOCAL"` if raw verification fails.
- `TLA-SCXH-001`: no close/unblock before approved evidence, exact BD recovery, and no blockers.
- `TLA-SCXH-002`: non-tautological subagent-laundering rejection.
- `TLA-SCXH-003`: mutation `FAIL_UNVIABLE` never adequacy pass.
- `TLA-SCXH-004`: generated parity deferral preserved.
- `TLA-SCXH-005`: canonical `.beads/vb-scxh/tla/` path consistency.
- `CI-SCXH-001`: green CI raw evidence audit.
- `MUT-SCXH-001`: mutation `FAIL_UNVIABLE` / `DEFERRED` classification.
- `SCOPE-SCXH-001`: generated parity scope-control audit.
- `TRUTH-SCXH-001`: Truth Serum final evidence decision gate.
- `SCOPEWRITE-SCXH-001`: State 3 write-scope audit.
- `ERR-SCXH-001` through `ERR-SCXH-010`: explicit error taxonomy trace rows.

## Planned Waiver Rows

- `WAIVE-VERUS-SCXH-001`: no Rust-local classifier target; expires before classifier implementation or Verus claim.
- `WAIVE-LEAN-SCXH-001`: no theorem-kernel target; expires before theorem-proven classification claim.
- `WAIVE-KANI-SCXH-001`: no Rust code/harness target changed by `vb-scxh`; referenced `vb-gvmt` Kani evidence is not closure proof.
- `WAIVE-FLUX-SCXH-001`: no Rust refinement target.
- `WAIVE-LOOM-SCXH-001`: no concurrent implementation target.
- `WAIVE-MIRI-SCXH-001`: no unsafe/runtime Rust surface changed.
- `WAIVE-PROPFUZZ-SCXH-001`: no parser/codec/input target.
- `WAIVE-PERF-API-REL-SCXH-001`: performance/API/release-provenance are non-goals for this artifact-only recovery bead.

## Known Downstream Blockers

- Safety anchor remains a downstream `BLOCK_LOCAL` blocker until raw bundle/bookmark verification succeeds; the State 4 planned row status remains `planned`.
- Exact 12 false closure IDs and per-ID raw reopened/linked/follow-up statuses remain State 11 work.
- Current TLA obligations must continue to use canonical `.beads/vb-scxh/tla/ScxhRecovery.tla` and `.beads/vb-scxh/tla/ScxhRecovery.cfg` metadata; no `.beads/vb-scxh/specs/` path is authoritative.
- State 12 must keep final decision blocked if any required raw lane remains missing, blocked, stale, or unsupported.
