# Proof Strategy — vb-rpch TLC Fix Pass

## Scope

Planner-only pass for `RecoveryReplayFull` TLA+/TLC repair and evidence hygiene in `/home/lewis/src/vb-jpq7-jj-fix`.

Primary artifacts in scope:

- `specs/tla/RecoveryReplayFull.tla`
- `specs/tla/RecoveryReplayFull.cfg`
- `evidence/specs/RecoveryReplayFull.tla`
- `evidence/specs/RecoveryReplayFull.cfg`
- `.beads/vb-rpch/*` proof/evidence/review reports
- root `proof-review.md` as stale downstream review artifact

This planner pass does **not** write proof code, TLA+ fixes, production Rust, tests, harnesses, verifier output, or reviewer approval.

## Risk Classification

| Risk class | Applies? | Evidence |
|---|---:|---|
| temporal/state-machine | yes | Recovery replay, snapshot-plus-tail, incomplete-run discovery, action resolution, digest stage ordering are state-machine properties. |
| Rust-local invariant | no for this pass | Verus obligations exist historically, but current user scope is TLC/TLA+ fix pass. |
| bounded state | yes | TLC cfg bounds `RunId`, `StepId`, `ActionId`, `Attempt`, `MAX_SEQ`, `MAX_EVENTS`; current `MAX_SEQ=100`, `MAX_EVENTS=20` is large and prior evidence is partial/stale. |
| refinement/type-state | adjacent only | TLA+ should later bridge to Rust replay/hydration behavior, but current pass repairs model/evidence. |
| concurrency | no | No concurrent algorithm in `RecoveryReplayFull`. |
| unsafe/UB | no | No Rust unsafe/UB work in this pass. |
| untrusted input | yes | Journal events/snapshots/digests model corrupted or adversarial recovery inputs. |
| dependency/supply-chain | no | TLC tooling availability only; no dependency proof. |
| performance | no | State explosion affects proof feasibility, not product performance. |
| release-critical gates | yes | Existing root/bead approval artifacts claim final proof status. |

## Current Findings — Stale or Contradictory Artifacts

Treat these as blockers until proof-writer/formal-verifier/proof-reviewer refresh them:

1. **Root and bead `proof-review.md` are stale approvals.** They say `APPROVED` and cite 144k+ states, while current files contain known model/cfg defects and do not include final raw TLC stdout/stderr.
2. **`.beads/vb-rpch/machine-gate-report.md` overclaims exhaustive proof.** It says 443k+ states were exhaustively explored; `.beads/vb-rpch/formal-verification-report.md` says BFS partial, and `.beads/vb-rpch/truth-serum-report.md` shows a progress line with 443,880 states left on queue.
3. **`specs/tla/RecoveryReplayFull.cfg` and `evidence/specs/RecoveryReplayFull.cfg` diverge.** Source cfg declares `Digest = {0,1,2,3}` even though the spec defines `Digest == ...`; evidence cfg removed that line.
4. **`SetSnapshot(0,0)` is invalid for the declared domains.** `RunId={1,2}`, `StepId={1,2,3}`, `ActionId={1,2}`, `Attempt={1,2}`. The current snapshot event uses `run=0`, `step=0`, `action=0`.
5. **`SetSnapshot` omits a primed assignment or `UNCHANGED` for `snapshot_seq`.** This risks unconstrained next-state behavior or an invalid action shape.
6. **`PROPERTY Spec` is not evidence.** Under `SPECIFICATION Spec`, checking `PROPERTY Spec` is tautological and must be removed or replaced with meaningful temporal/non-vacuity properties.
7. **`Sort(s, less) == s` is suspicious.** If unused, remove it; if intended for replay order, identity sorting cannot support an ordering proof.
8. **RecoveryErrorExhaustive is not proven by `last_error \in {...}`.** Membership is TypeOK; it does not prove each error is reachable from defined inputs.
9. **Non-vacuity is not established.** The review table says antecedents "can be" true, but no reachability run proves they are actually reachable under `Spec`.

## Lane Decisions

Machine-readable lane decisions are in:

- `.beads/vb-rpch/verifier-lane-decisions.tlc-fix.jsonl`

Summary:

- **TLA+ / TLC: required** for this pass.
- **Verus: not applicable to current pass**; existing Verus rows remain historical and should not be silently re-approved here.
- **Kani: not applicable to current pass**; existing Kani rows remain historical/tooling-blocked.
- **BDD: not applicable to current pass**; behavior tests may be needed after implementation bridge, not for this model-fix planning pass.

## Planned Obligations

Machine-readable obligations are in:

- `.beads/vb-rpch/proof-obligations.tlc-fix.planned.jsonl`

Planned obligations:

1. `TLC-FIX-001` — cfg well-formedness; remove undeclared/extra `Digest` constant and sync source/evidence cfg.
2. `TLC-FIX-002` — repair `SetSnapshot` domain values and missing `snapshot_seq'`/`UNCHANGED` assignment.
3. `TLC-FIX-003` — remove or implement identity `Sort` and verify `ReplaySeqOrder` is not vacuous.
4. `TLC-FIX-004` — remove tautological `PROPERTY Spec`; keep all invariant declarations.
5. `TLC-FIX-005` — run small exhaustive smoke TLC cfg after repairs.
6. `TLC-FIX-006` — run primary bounded TLC cfg and capture final raw output; do not call partial BFS exhaustive.
7. `TLC-FIX-007` — add/run non-vacuity reachability obligations for each invariant and each modeled recovery error.
8. `TLC-FIX-008` — sync `evidence/specs` and supersede stale approval/evidence reports after formal execution and proof-reviewer review.

## Exact Commands for Writer/Formal-Verifier

Discovered locally by planner:

- `tlc`: `/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc`
- `java`: `/home/lewis/.local/share/mise/installs/java/26.0.1/bin/java`
- No repository-local `tla2tools.jar` was discovered by the planner.

Primary command from repository root:

```bash
cd /home/lewis/src/vb-jpq7-jj-fix && tlc -config specs/tla/RecoveryReplayFull.cfg specs/tla/RecoveryReplayFull.tla
```

Smoke command after writer creates a small cfg:

```bash
cd /home/lewis/src/vb-jpq7-jj-fix && tlc -config specs/tla/RecoveryReplayFull-smoke.cfg specs/tla/RecoveryReplayFull.tla
```

Non-vacuity command after writer creates the non-vacuity cfg/predicates:

```bash
cd /home/lewis/src/vb-jpq7-jj-fix && tlc -config specs/tla/RecoveryReplayFull-nonvacuity.cfg specs/tla/RecoveryReplayFull.tla
```

Fallback shape only if a real jar path is discovered by writer/formal-verifier:

```bash
cd /home/lewis/src/vb-jpq7-jj-fix && java -jar /path/to/tla2tools.jar -config specs/tla/RecoveryReplayFull.cfg specs/tla/RecoveryReplayFull.tla
```

Evidence sync check after repairs:

```bash
cd /home/lewis/src/vb-jpq7-jj-fix && cmp -s specs/tla/RecoveryReplayFull.tla evidence/specs/RecoveryReplayFull.tla && cmp -s specs/tla/RecoveryReplayFull.cfg evidence/specs/RecoveryReplayFull.cfg
```

## Required Bounded Model Limits

Use two levels:

1. **Smoke/exhaustive sanity bounds**: `RunId={1}`, `StepId={1}`, `ActionId={1}`, `Attempt={1}`, `MAX_SEQ=3`, `MAX_EVENTS=3`. This should finish and catch parser/type/action-shape defects.
2. **Primary bounded cfg**: current declared domains are `RunId={1,2}`, `StepId={1,2,3}`, `ActionId={1,2}`, `Attempt={1,2}`, `MAX_SEQ=100`, `MAX_EVENTS=20`. If this does not complete, formal-verifier must call it **partial BFS**, not exhaustive PASS, and must record queue/depth/states.

## Non-Vacuity Plan

Proof-writer should add named reachability predicates or separate negated-invariant cfgs. Required reachability evidence:

- `Len(journal) >= 2` reachable for `ReplaySeqOrder`.
- `snapshot_seq >= 0` and at least one post-snapshot event reachable for `TailCausalAfterSnapshot`.
- `recovered_runs # {}` reachable for `OnlyIncompleteRuns`.
- At least one terminal event is reachable and excluded from `recovered_runs`.
- `ActionCompleted` / resolved action guard states reachable for `NoResolvedReExecution`.
- `RunAccepted` with digest checks reachable for `DigestVerificationOrder`.
- Each modeled `last_error` value is reachable from a defined input, or the claim `RecoveryErrorExhaustive` must be downgraded/staled.

Acceptable technique: define `CanReachX` predicates and check `INVARIANT NotCanReachX == ~CanReachX` in a dedicated cfg; TLC should produce a counterexample demonstrating reachability. Store each raw log under `evidence/specs/RecoveryReplayFull-nonvacuity.*.log`.

## Trusted Base / Waiver Notes for This Pass

- TLC itself and finite bounds are trusted.
- If primary cfg cannot complete, any claim must be **bounded partial evidence**, not proof of exhaustive correctness.
- Existing GAP-3 and terminal mismatch waivers are outside this pass; do not re-approve them here.
- Existing approval files must not be treated as proof-reviewer approval for repaired files. A fresh proof-reviewer pass is required after writer/formal-verifier.

## Proof-to-Implementation Input for Later Bridge

- TLA `ReplaySeqOrder` maps to Rust replay ordering and sequence-gap behavior in `crates/vb_storage/src/recovery/replay/core.rs` and related BDD scenarios.
- TLA `TailCausalAfterSnapshot` maps to `recover_snapshot_plus_tail` / `hydrate_run_frame` snapshot-tail preconditions.
- TLA `OnlyIncompleteRuns` maps to `recover_all_incomplete_runs` latest-attempt terminal filtering.
- TLA `NoResolvedReExecution` maps to `ActionReplayTracker` and non-idempotent replay blocking.
- TLA `DigestVerificationOrder` maps to `verify_digests`, `check_workflow_source_digest`, and `check_compiled_ir_digest`.
- TLA `RecoveryErrorExhaustive` currently lacks reachability evidence; bridge must not map it as proven until `TLC-FIX-007` succeeds.
