# Proof Strategy: vb-scxh

## State Boundary

- State: 4 REPAIR, proof planning only.
- Workspace: `/home/lewis/src/vb-scxh`.
- Artifact write scope: `.beads/vb-scxh/` only.
- Forbidden write scope: `/home/lewis/src/Velvet-ballistics` and all production code, tests, proof code, harnesses, TLA/Lean/Verus/Kani specs, dependencies, and CI config.
- This artifact is a plan. It does not approve proof execution, close `vb-scxh`, or unblock `vb-engine-yaml`.

## Skill Basis

- Read/cited `/home/lewis/.claude/skills/proof-planner/SKILL.md`.
- Lines 15-24 define the proof-planner mission, planning-only rule, traceability rule, waiver rule, workflow, output artifacts, and required JSONL schema fields.
- Lines 31-35 state that proof-planner decides what must be proven and writes planning artifacts only, without proof code, tests, production code, harnesses, models, specs, dependencies, or CI config.
- Lines 41-54 require workspace-scoped discovery and recording blocked discovery if a command cannot run.
- Lines 56-70 forbid hallucinated verifier results and require stable IDs, exact artifact paths/commands, explicit assumptions/model bounds, and explicit skipped-lane rows.

## Inputs Read

- `.beads/vb-scxh/STATE.md`: current State 4 repair after State 6 rejection; State 4 must align planned obligations with repaired TLA paths, waiver ledger, and evidence-integrity obligations.
- `.beads/vb-scxh/contract.md`: repaired Truth Serum recovery/evidence-integrity contract with canonical `.beads/vb-scxh/tla/` paths and `BLOCK_LOCAL` safety-anchor requirement.
- `.beads/vb-scxh/domain-model-review.md`: illegal states include subagent-only evidence acceptance, missing BD IDs, safety-anchor failure papered over, mutation `FAIL_UNVIABLE` as PASS, generated parity scope conflation, and specs/tla path mismatch.
- `.beads/vb-scxh/tla-spec.md`: State 5 must target `.beads/vb-scxh/tla/ScxhRecovery.tla` and `.beads/vb-scxh/tla/ScxhRecovery.cfg`; `.beads/vb-scxh/specs/` is rejected unless State 5 moves/rewrites and reruns exact commands.
- `.beads/vb-scxh/lean-contract.md`: theorem lane is waived/deferred because no theorem target exists.
- `.beads/vb-scxh/verification-layers.md`: TLA+ owns temporal/evidence workflow; raw evidence owns BD, safety anchor, CI, mutation, scope, and final decision; skipped Rust/theorem lanes are primary-ledger waivers.
- `.beads/vb-scxh/proof-obligations.jsonl`: current repaired primary ledger has 33 rows.
- `.beads/vb-scxh/traceability-matrix.jsonl`: current repaired traceability matrix has 27 rows.
- `.beads/vb-scxh/contract-repair-report.md`: State 3 repair added missing clauses, error rows, primary waiver rows, and canonical TLA paths.
- State 6 rejection inputs: `.beads/vb-scxh/contract-verification-review.md`, `.beads/vb-scxh/proof-review.md`, `.beads/vb-scxh/proof-findings.jsonl`.

## Discovery Evidence

- `pwd -P` from `/home/lewis/src/vb-scxh` returned `/home/lewis/src/vb-scxh`.
- Required State 4 repair inputs were checked non-empty with `test -s`: `STATE.md`, `contract.md`, `domain-model-review.md`, `tla-spec.md`, `lean-contract.md`, `verification-layers.md`, `proof-obligations.jsonl`, `traceability-matrix.jsonl`, `contract-repair-report.md`, `proof-findings.jsonl`, `proof-review.md`, and `contract-verification-review.md`.
- Scoped risk-trigger search over `.beads/vb-scxh` found recovery state, transition, blocked/unblock, safety-anchor, mutation, BD, TLA, waiver, and Truth Serum markers. Output was truncated by the tool, so this strategy records only the observed risk classes, not exhaustive line coverage.
- Scoped verifier-trigger search over `.beads/vb-scxh` found canonical TLA paths, repaired TLA obligations, primary waiver rows for Verus/Lean/Kani/Flux/Loom/Miri/proptest/fuzz, and State 6 findings for missing liveness/fairness and tautological laundering proof.
- No discovery command required for State 4 planning was blocked.

## Risk Classification

- `evidence-integrity`: critical. `SUBAGENT_CLAIM` cannot satisfy required evidence without distinct raw command or artifact evidence.
- `false-closure-recovery`: critical. State 11 must capture exactly 12 false closure IDs and per-ID raw BD reopened/linked/follow-up evidence.
- `safety-anchor`: critical. Safety rows remain `status:"planned"` in the State 4 ledger while preserving downstream `failure_classification:"BLOCK_LOCAL"` semantics until raw bundle/bookmark verification passes.
- `ci-evidence`: critical. Green CI requires raw `moon ci` markers: command, PASS, 19 completed tasks, 8276/8276 tests, runtime marker, and artifact path or fresh rerun.
- `mutation-classification`: high. Mutation `FAIL_UNVIABLE` / `DEFERRED` is not mutation adequacy PASS.
- `scope-control`: high. Generated parity remains deferred to `vb-gvmt` / `vb-qi37.10` and is not closure proof for `vb-scxh`.
- `temporal-workflow`: proof. TLA+ must prove close/unblock cannot occur before approved evidence, exact BD recovery, and no local blockers.
- `path-consistency`: critical. State 5 proof artifacts and commands must use `.beads/vb-scxh/tla/` consistently, or explicitly move/repair paths and rerun exact moved commands.
- `waiver-integrity`: medium/high. Skipped verifier lanes must remain machine-readable primary-ledger obligations with owner, reason, expiry, compensating evidence, and rerun trigger.

## Planned Verifier Lanes

- `tla-plus`: required for `TLA-SCXH-001` through `TLA-SCXH-005`. State 5 must use `tlc -config .beads/vb-scxh/tla/ScxhRecovery.cfg .beads/vb-scxh/tla/ScxhRecovery.tla` and report canonical paths. The non-laundering invariant must model `AttemptLaunderSubagentEvidence` or equivalent required-evidence candidate behavior, not merely `Subagent != Raw`.
- `tla-liveness/fairness`: conditional. If State 5 keeps liveness/fairness claims, it must configure and report them. If not, State 5 must explicitly waive temporal/liveness evidence and state that TLC PASS proves safety only, not eventual closure.
- `path-guard-audit`: required for workspace and write-scope obligations.
- `artifact-presence-audit`: required for local and referenced evidence inputs.
- `bd-closure-audit`: required for exact 12 false closures and raw reopened/linked status.
- `safety-anchor-audit`: required and planned; any raw bundle/bookmark failure must be recorded as downstream `BLOCK_LOCAL` and block State 11/12 close/unblock.
- `moon-ci-evidence-audit`: required for green CI raw markers or fresh rerun evidence.
- `mutation-classification-audit`: required for `FAIL_UNVIABLE` / `DEFERRED` non-pass classification.
- `scope-control-audit`: required for generated parity deferral ownership.
- `truth-serum`: required for assurance bundle review and final evidence decision; must block close/unblock if any required raw lane remains missing, blocked, or unsupported.
- `waiver-ledger`: required for Verus, Lean/Aeneas/Hax, Kani, Flux, Loom/Shuttle, Miri/cargo-careful, proptest/fuzz, and performance/API/release-provenance non-goals.

## Planned State 5 Repairs

- Reuse canonical `.beads/vb-scxh/tla/` paths in every target, config, command, report, and obligation row.
- Repair or rewrite the existing TLA model so `TLA-SCXH-002` exercises a required evidence item or acceptance candidate supplied only by `Subagent`, records an attempted laundering transition, and proves package acceptance, final approval, close, and engine unblock remain impossible until distinct raw/artifact evidence exists.
- Include safety-anchor state in the TLA model and prove `OpenFailed` or `Missing` blocks approval/unblock.
- Preserve mutation `FAIL_UNVIABLE` and generated parity deferral invariants.
- Either configure liveness/fairness properties in `.beads/vb-scxh/tla/ScxhRecovery.cfg` and report them, or record an explicit State 5 waiver that removes liveness from closure evidence and limits TLA evidence to safety.
- Rerun the exact TLC command and update `tla-report.md`, `proof-evidence.md`, and `proof-writer-report.md` before State 6 review.

## Planned State 11 Repairs

- Produce `.beads/vb-scxh/bd-closure-audit.md` with `EXACT_FALSE_CLOSURE_COUNT=12`, all 12 bead IDs, and per-ID raw reopened/linked/follow-up evidence from `bd --db /home/lewis/src/.beads/dolt ...`; missing or truncated BD evidence remains blocked.
- Produce `.beads/vb-scxh/safety-anchor-report.md` from raw `git bundle verify /home/lewis/src/Velvet-ballistics-rescue-20260513T022011Z.bundle` and `git show-ref rescue-vb-scxh-ci-green-20260513T030158Z`; any bundle-open/ref failure remains downstream `BLOCK_LOCAL` until repaired.
- Produce `.beads/vb-scxh/moon-ci-evidence-audit.md` with `moon ci`, PASS, 19 completed tasks, 8276/8276 tests passed, runtime marker, and artifact path or a fresh rerun.
- Produce `.beads/vb-scxh/mutation-classification-audit.md` preserving `FAIL_UNVIABLE` / `DEFERRED`, never adequacy PASS.
- Produce `.beads/vb-scxh/scope-control-audit.md` proving generated parity gaps remain owned by `vb-gvmt` / `vb-qi37.10`.

## Planned State 12 Decision Gate

- Produce `.beads/vb-scxh/truth-serum-report.md` and `.beads/vb-scxh/final-evidence-decision.md` only after State 11 evidence artifacts exist.
- Final decision may state `APPROVE_CLOSE_OR_UNBLOCK` only if exact 12 BD recovery, safety anchor, green CI, mutation classification, generated parity deferral, path-consistent TLA proof, and waiver ledger all pass or have approved waivers.
- If safety bundle/bookmark is unverifiable in State 11/12 raw evidence, final decision must remain `BLOCKED` with `Error::SafetyAnchorMissing` / `BLOCK_LOCAL`.

## Output Ledger

- `.beads/vb-scxh/proof-obligations.planned.jsonl` contains 33 rows with `status:"planned"`, one for every current `proof-obligations.jsonl` ID.
- Every planned row includes: `id`, `requirement_id`, `contract_clause`, `risk`, `verifier`, `artifact`, `command`, `expected_evidence`, `assumptions`, `required`, `mode`, `owner_state`, `rerun_from`, `status`, and `waiver`.
- There are no unowned `blocked` rows and no non-planned statuses. Safety-anchor rows are explicitly owned planned rows with downstream `failure_classification:"BLOCK_LOCAL"`, `downstream_blocker:true`, State 11/12 ownership, and rerun points preserved from the repaired primary ledger.
