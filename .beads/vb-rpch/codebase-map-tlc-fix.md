# Codebase Map — vb-rpch TLC Fix Scout

Scope: RecoveryReplayFull TLA+/TLC fix pass only. Scout inspected source/evidence specs, cfgs, planner artifacts, stale reviews, scoped evidence directories, tool availability, and helper references. Production code/tests/proofs/config/dependency files were not edited.

## Workspace / bead inputs

- Workspace exists: `/home/lewis/src/vb-jpq7-jj-fix`.
- Bead artifact dir exists: `.beads/vb-rpch/`.
- Planner inputs present and read:
  - `.beads/vb-rpch/proof-strategy-tlc-fix-pass.md` lines 33-45 list current blocker findings.
  - `.beads/vb-rpch/proof-obligations.tlc-fix.planned.jsonl` lines 1-8 define `TLC-FIX-001` through `TLC-FIX-008`.
  - `.beads/vb-rpch/verifier-lane-decisions.tlc-fix.jsonl` line 1 marks TLA+/TLC required.
  - `.beads/vb-rpch/proof-coverage-matrix-tlc-fix.md` lines 5-12 mark stale/blocked coverage.
  - `.beads/vb-rpch/trusted-base-plan-tlc-fix.md` lines 10-21 marks old approvals and non-exhaustive runs untrusted.
  - `.beads/vb-rpch/waiver-candidates-tlc-fix.md` lines 11-16 rejects waivers for the current model defects.
  - `.beads/vb-rpch/proof-to-implementation-input-tlc-fix.md` lines 3-12 blocks downstream bridge until TLC fixes/evidence are accepted.

## TLA+/TLC source artifacts

### `specs/tla/RecoveryReplayFull.tla`

Important symbols and risk lines:

- Constants: lines 16-22 declare `RunId`, `StepId`, `ActionId`, `Attempt`, `MAX_SEQ`, `MAX_EVENTS`; `Digest` is **not** a declared constant.
- `Digest == {0, 1, 2, 3}`: line 32. This conflicts with source cfg line 9 assigning `Digest` as a constant.
- `RECORDEvent`: lines 45-54 requires `run ∈ RunId`, `step ∈ StepId`, `action ∈ ActionId`, `attempt ∈ Attempt`, `seq ∈ EventSeqNum`.
- `Sort(s, less) == s`: line 88. Suspicious identity operator; appears unused in current file but dangerous if cited as ordering proof.
- `BuildSeqFromIndices`: lines 103-108 recursively appends by minimum index.
- `AppendEvent`: lines 110-113 includes `Len(journal) < MAX_EVENTS`; good bound present for generic append.
- `SetSnapshot(run, seq)`: lines 115-118 has multiple defects:
  - line 116 guards on current `snapshot_seq >= 0`, making it unreachable from `Init` where `snapshot_seq = -1` (lines 74-80), unless another action unconstrains it.
  - line 117 appends event with `step=0`, `action=0`, `attempt=1`, `seq=seq`; source cfg StepId/ActionId are `{1,...}` so `step=0` and `action=0` violate `RECORDEvent` typing.
  - line 118 omits both `snapshot_seq' = ...` and `UNCHANGED snapshot_seq`, leaving `snapshot_seq'` unconstrained in this action shape.
- `ReplayEvents`: lines 132-144 filters by current journal index, attempt, and run; `tracker.completed` is populated from scheduled actions at lines 136-142. Non-vacuity of relevant resolved/completed/scheduled states is UNKNOWN without reachability evidence.
- `CheckWorkflowDigest`: lines 157-165 can set only `WorkflowSourceDigestMismatch`.
- `CheckIrDigest`: lines 167-176 can set only `CompiledIrDigestMismatch`.
- `TailCausalAfterSnapshot`: lines 178-181. Antecedent reachability is UNKNOWN and likely blocked by `SetSnapshot` guard/prime defect.
- `ReplaySeqOrder`: lines 183-185. Needs non-vacuity evidence for `Len(journal) >= 2` and bad-order prevention.
- `OnlyIncompleteRuns`: lines 187-192. Needs `recovered_runs # {}` reachability evidence.
- `NoResolvedReExecution`: lines 194-202. Needs guard-state reachability evidence; stale review simultaneously says PASS and “known pre-existing violation.”
- `DigestVerificationOrder`: lines 204-208 only constrains non-zero digests on `RunAccepted`; it does not prove workflow-before-IR temporal order by itself.
- `Next`: lines 210-219 includes `SetSnapshot(0, 0)` at line 214. `run=0` violates source cfg `RunId={1,2}` and source/event typing. `RecordError(NoneError)` at line 219 means only `None`, workflow mismatch, and IR mismatch are directly produced by current actions; other `last_error` variants are only type-domain members.
- `THEOREM` declarations: lines 223-228 include six invariants. TLC checks cfg invariants, not these declarations as proofs.

### `specs/tla/RecoveryReplayFull.cfg`

- Lines 2-8 set `RunId={1,2}`, `StepId={1,2,3}`, `ActionId={1,2}`, `Attempt={1,2}`, `MAX_SEQ=100`, `MAX_EVENTS=20`.
- Line 9 assigns `Digest = {0, 1, 2, 3}` even though spec line 32 defines `Digest == ...` and does not declare it as a `CONSTANT`. Treat as cfg well-formedness defect until TLC raw output says otherwise.
- Lines 11-17 list all six invariants.
- Lines 19-20 set `PROPERTY Spec` under `SPECIFICATION Spec`. This is tautological/non-evidence and must not support proof approval.

### `specs/tla/RecoveryReplayFull-depth4.cfg`

- Lines 1-21 duplicate the same extra `Digest` constant and `PROPERTY Spec` tautology, with `DEPTH 4` line 21. Treat as stale/unsafe helper cfg, not proof evidence.

## Evidence/spec divergence

- `cmp specs/tla/RecoveryReplayFull.tla evidence/specs/RecoveryReplayFull.tla` returned equal.
- `cmp specs/tla/RecoveryReplayFull.cfg evidence/specs/RecoveryReplayFull.cfg` returned different.
- `evidence/specs/RecoveryReplayFull.cfg` lines 1-19 removes `Digest = {0,1,2,3}` but retains `PROPERTY Spec` lines 18-19.
- `.beads/vb-rpch/evidence/specs/RecoveryReplayFull.tla` is a different old 156-line model:
  - constants `RUN_ID`, `MAX_STEPS`, `MAX_ACTIONS`, `MAX_EVENTS`, `MAXSEQ` at lines 4-9.
  - old invariants `StepOrderInvariant`, `NoDivergenceInvariant`, `NoDoubleScheduling`, `ActionSafety` at lines 51-73.
  - `Spec` over variables `events`, `replayed`, etc. at line 154.
- `.beads/vb-rpch/evidence/specs/RecoveryReplayFull.cfg` is a different old cfg with `INVARIANTS` lines 2-6 and constants lines 7-12. It does not match current six-invariant RecoveryReplayFull source model.

## Stale / contradictory review artifacts

- Root `proof-review.md`:
  - line 10 says `STATUS: APPROVED`.
  - line 12 claims 144k+ states and current spec sync.
  - lines 38-47 claim non-vacuity because antecedents “can be” true; no reachability evidence is shown.
  - lines 63-68 cite prior/in-progress TLC, not final raw completion.
  - lines 75-80 mark TLA obligations PASS, including line 78 `NoResolvedReExecution` PASS with “known pre-existing violation in spec”.
  - lines 89-90 admit proof obligations not updated and no raw TLC stdout/stderr.
  - line 96 approves despite these gaps.
- `.beads/vb-rpch/proof-review.md` is same stale approval content with the same risky lines.
- `.beads/vb-rpch/contract-verification-review.md`:
  - line 1 says Attempt 17; line 8 claims `RecoveryReplayFull.tla` 293 lines, contradictory with current source/evidence 232-line current file and bead evidence old 156-line file.
  - lines 22-27 cite line numbers from a different model.
  - line 51 claims 443k states BFS depth 5, 0 violations.
  - line 60 ledger row says `result:"PASS"`.
- Root `contract-verification-review.md`:
  - line 2 approved.
  - lines 12-13 says all 5 required invariants / TypeOK passes; later lines 48-61 say all six invariants. Inconsistent count.
  - lines 65-69 say evidence is state files and proof-evidence, no raw stdout/stderr.
  - lines 105-112 approve while marking no raw TLC stdout/stderr non-blocking.
- `.beads/vb-rpch/formal-verification-report.md`:
  - line 3 `SUBSTANTIAL_COVERAGE`, not exhaustive proof.
  - lines 7 and 15-20 claim PASS from 443k states.
  - line 44 explicitly says `BFS partial (443k states)`.
  - line 52 ledger says `PASS_LOCAL`; this conflicts with line 44 partial BFS if presented as proof.
- `.beads/vb-rpch/machine-gate-report.md`:
  - lines 27-29 say PASS and “exhaustively explored 443k+ states at depth 5”. Contradicts formal report partial BFS and truth-serum queue line.
- `.beads/vb-rpch/truth-serum-report.md`:
  - lines 27-31 quote only TLC progress lines, including `443,880 states left on queue`.
  - lines 34-36 convert “no Error found in progress log” into `TLC PASS`, which is unsound without final completion/stop evidence.
- `.beads/vb-rpch/verification-ledger.jsonl`:
  - lines 11-13 mark TLA rows PASS based on `formal-verification-report.md`; these are stale until fresh TLC logs exist.
  - lines 15-17 include approved waivers; outside this TLC fix pass unless proof-reviewer refreshes them.

## Raw evidence/log availability

Scoped discovery found these evidence files:

- `evidence/proof-evidence.md` lines 138-146 says `tlc ... RecoveryReplayFull` was `NOT_RUN` because TLC not available. This conflicts with later review claims.
- `.beads/vb-rpch/evidence/proof-evidence.md` lines 43-51 records partial 144,036+ states and says full exhaustive model checking still running.
- `.beads/vb-rpch/evidence/proof-evidence.md` lines 85-95 records old simulation mode for different invariants, not current six-invariant model checking.
- `.beads/vb-rpch/truth-serum-report.md` lines 27-31 quotes `tlc-fixed.log` progress, but scoped file discovery found no `tlc-fixed.log`, no `*.tlc.log`, no `RecoveryReplayFull*.log`, and no `states/` directory under the current workspace.
- `states/` directory at workspace root: MISSING.
- Raw final TLC stdout/stderr for current `specs/tla/RecoveryReplayFull.tla` + current cfg: MISSING.
- Raw small smoke cfg TLC log: MISSING.
- Raw non-vacuity TLC logs: MISSING.

## Tool/helper availability

- `command -v tla2tools`: `/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tla2tools`.
- `command -v tlc`: `/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc`.
- `command -v java`: `/home/lewis/.local/share/mise/installs/java/26.0.1/bin/java`.
- Repository-local `tla2tools.jar`: not found by scoped `rg --files` search.
- Helper references:
  - `xtask/src/proof.rs` lines 136-160 emits `tla2tools {tla}` for obligations; not tailored to cfg-driven `tlc -config ...` command.
  - `xtask/src/lanes.rs` lines 99-104 builds `verification/tla/{crate}.tla`; not tailored to `specs/tla/RecoveryReplayFull.tla`.
  - `scripts/` has no direct TLC helper match from scoped grep.

## Risk tags for downstream agents

- `temporal/state-machine`: `Next`, `SetSnapshot`, replay filtering, digest checks, terminal filtering.
- `proof-evidence`: approvals lack final raw TLC output and include progress-only evidence.
- `non-vacuity`: antecedent reachability for all invariants and error variants is UNKNOWN/MISSING.
- `stale-approval`: multiple root/bead reviews approve stale or contradictory evidence.
- `bounded-arithmetic`: `MAX_SEQ=100`, `MAX_EVENTS=20` may be too large; partial BFS must not be called exhaustive.
- `model-well-formedness`: cfg extra constant, invalid domain values, missing prime assignment.
- `cfg-divergence`: source/evidence/bead-evidence cfg/spec copies disagree.

## Recommended next owner handoff

- Proof-writer: repair model/cfg and create smoke/non-vacuity artifacts only after acknowledging current raw evidence is missing/stale.
- Formal-verifier: execute exact TLC commands and preserve raw stdout/stderr; classify completion honestly as exhaustive or partial BFS.
- Proof-reviewer: reject old approvals as stale; review only fresh spec/cfg hashes plus raw logs.
