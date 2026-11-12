# Contract Verification Review — vb-rpch (Attempt 17)

## Reviewer Role
contract-verification-reviewer (state 6)

## Evidence Reviewed
- `contract.md` (.beads/vb-rpch/ — 135 lines)
- `RecoveryReplayFull.tla` (293 lines — attempt 17)
- `formal-verification-report.md` (62 lines)

---

## 1. Does contract.md correctly specify the recovery/replay behavior?

**YES.**

| Contract Element | Spec Coverage | Status |
|---|---|---|
| PRE-001 to PRE-005 | Dimension bounds, seq ordering, non-empty events | OK |
| POST-001 to POST-010 | Digest checks, recovery hydration, replay, tracker | OK |
| INV-001 to INV-006 | Error distinctness, unsupported state, seed field bounds, tracker monotonicity, digest hierarchy, incomplete-run discovery | OK |
| TLA-001 (ReplaySeqOrder) | Line 213-215: `\A i, j : i < j => journal[i].seq <= journal[j].seq` | OK |
| TLA-002 (TailCausalAfterSnapshot) | Line 208-211: `snapshot_seq >= 0 => \A i : journal[i].seq > snapshot_seq` | OK |
| TLA-003 (OnlyIncompleteRuns) | Line 217-222: no terminal event of max attempt for runs in `recovered_runs` | OK |
| TLA-004 (NoResolvedReExecution) | Line 224-237: pending/completed/failed mutual exclusion + blocking | OK |
| TLA-005 (RecoveryErrorExhaustive) | `last_error` domain (line 80) covers all 9 error variants | OK |
| TLA-006 (DigestVerificationOrder) | Line 239-243: RunAccepted digests non-zero | OK |

**Attempt 17 fix confirmed**: `ReplayEvents` (line 170-172) now correctly moves tuples from `pending` to `completed` via `tracker.pending \ resolved` — this was the core defect in prior attempts.

---

## 2. Are the deferred gaps properly documented?

**YES.**

| Gap | Location in contract.md | Description | Status |
|---|---|---|---|
| GAP-1 | Line 133 | `hydrate_run_frame` does NOT call `set_max_parallel_in_flight`; snapshot-based path lacks observed-peak tracking | DOCUMENTED |
| POST-007-gap | Line 134 | `RecoveryFrameSeed.unsupported` exists but not propagated to `RunFrame.unsupported` | DOCUMENTED |
| GAP-3 | Line 124, 133 | ActionAbiMismatch/PolicyDigestMismatch not reachable via public API | DOCUMENTED, deferred to vb-ty9 |

No undocumented gaps found.

---

## 3. Any remaining contract gaps?

**NO new gaps identified.**

TLC ran 443k states (BFS, depth 5) with 0 invariant violations across all 6 invariants. State space bounded by MAX_EVENTS=20. Structural invariants (TypeOK, TailCausalAfterSnapshot, DigestVerificationOrder) are proof-verified in the spec itself.

The GAP-1 and POST-007-gap represent implementation-level deviations from POST-006/POST-007 postconditions — both are explicitly called out and do not represent silent contract violations.

---

## Verification Ledger Entry

```jsonl
{"bead":"vb-rpch","state":6,"reviewer":"contract-verification-reviewer","spec":"RecoveryReplayFull.tla","all_6_invariants_defined":true,"tlc_states":443000,"tlc_depth":5,"invariant_violations":0,"gaps_documented":3,"contract_adequate":true,"result":"PASS"}
```

---

## STATUS: APPROVED