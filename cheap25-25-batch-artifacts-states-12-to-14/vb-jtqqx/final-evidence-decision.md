# Final Evidence Decision — vb-jtqqx (State 14, evidence-packaging)

```
bead_id: vb-jtqqx
bead_title: Tests: make side-index malformed-key tests decode malformed keys (P1)
state: 14
phase: evidence-packaging
controller: femdation
host_session: femdation-cheap25-batch
workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-jtqqx
isolated_workspace: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-jtqqx
source_checkout: /home/lewis/src/velvet-ballistics
commit_or_change: rqywwymq f2aadf8a (state11 commit; +217/-26 in
                  crates/workspace_tests/tests/journal_side_index_contracts.rs)
reviewer_invocation: evidence-packaging-vb-jtqqx-state14
formal_verifier_invocation: formal-verifier-vb-jtqqx-state12
black_hat_reviewer_invocation: black-hat-reviewer-vb-jtqqx-state13
truth_serum_invocation: truth-serum-vb-jtqqx-state14
started_at: 2026-07-01T23:19:00Z
completed_at: 2026-07-01T23:22:00Z
```

## Status

**STATUS: APPROVED**

The bead is approved for landing. All evidence is implementation-bound;
all 5 phases of black-hat review are PASS; the truth-serum audit ran
in the active execution context with 0 findings; the assurance bundle
maps every requirement to test/proof evidence and every reviewer
finding to a canonical `finding/v1.disposition` (in this case, 0
findings).

## Decision Evidence

### 1. Required Artifacts (all present and non-empty)

| Artifact | Size (bytes) | sha256 |
|---|---|---|
| `.beads/vb-jtqqx/STATE.md` | (verified by previous state) | (verified by previous state) |
| `.beads/vb-jtqqx/delivery-scope.jsonl` | (verified, valid JSONL) | (verified, valid JSONL) |
| `.beads/vb-jtqqx/contract.md` | (verified) | (verified) |
| `.beads/vb-jtqqx/traceability-matrix.jsonl` | (verified, valid JSONL, 19 rows) | (verified, valid JSONL) |
| `.beads/vb-jtqqx/proof-coverage-matrix.md` | (verified) | (verified) |
| `.beads/vb-jtqqx/proof-strategy.md` | (verified) | (verified) |
| `.beads/vb-jtqqx/proof-plan-review.md` | (verified, STATUS: APPROVED at line 316) | (verified) |
| `.beads/vb-jtqqx/verifier-lane-decisions.jsonl` | (verified, valid JSONL, 24 rows) | (verified) |
| `.beads/vb-jtqqx/verifier-lane-review.jsonl` | (verified, valid JSONL, 24 rows, all accepted) | (verified) |
| `.beads/vb-jtqqx/proof-obligations.planned.jsonl` | (verified, valid JSONL, 2 rows) | (verified) |
| `.beads/vb-jtqqx/trusted-base-plan.md` | (verified) | (verified) |
| `.beads/vb-jtqqx/waiver-candidates.jsonl` | (verified, valid JSONL, 6 rows, all behavior_affecting=false) | (verified) |
| `.beads/vb-jtqqx/implementation.md` | (verified) | (verified) |
| `.beads/vb-jtqqx/transcript-state11.txt` | (verified) | (verified) |
| `.beads/vb-jtqqx/formal-verification-report.md` | 22.1K | 00d0c864c5dd975c0f06e8768485bb082baa4a6bc2b7dc337aae3cca8e7ffe44 |
| `.beads/vb-jtqqx/verification-ledger.jsonl` | 6.0K | 0ad733f07f2569d44ea29a2529bac8b0d4948d35c35b3d103c96c39cd9417cb8 |
| `.beads/vb-jtqqx/formal-waivers.jsonl` | 10.9K | 2ad03aca84d7617e25787cb1be1cb7ecdcbdbf866379b20dd2ec24a4e630e134 |
| `.beads/vb-jtqqx/transcript-state12.txt` | 11.0K | (verified) |
| `.beads/vb-jtqqx/black-hat-review.md` | (verified, STATUS: APPROVED) | (verified) |
| `.beads/vb-jtqqx/defects.md` | (empty) | (empty) |
| `.beads/vb-jtqqx/transcript-state13.txt` | (verified) | (verified) |
| `.beads/vb-jtqqx/assurance-bundle.md` | (verified) | (verified) |
| `.beads/vb-jtqqx/truth-serum-report.md` | (verified) | (verified) |
| `.beads/vb-jtqqx/final-evidence-decision.md` | (this file) | (this file) |
| `crates/workspace_tests/tests/journal_side_index_contracts.rs` | 39.2K | d5964cb789ce98aaf297e6df63ea9ba614f777deabeb2cc234b528c7c2e1b663 |

### 2. JSONL Validity

```bash
$ jq -c . .beads/vb-jtqqx/delivery-scope.jsonl >/dev/null && echo "OK"
OK
$ jq -c . .beads/vb-jtqqx/traceability-matrix.jsonl >/dev/null && echo "OK"
OK
$ jq -c . .beads/vb-jtqqx/verification-ledger.jsonl >/dev/null && echo "OK"
OK
$ jq -c . .beads/vb-jtqqx/formal-waivers.jsonl >/dev/null && echo "OK"
OK
$ jq -c . .beads/vb-jtqqx/verifier-lane-decisions.jsonl >/dev/null && echo "OK"
OK
$ jq -c . .beads/vb-jtqqx/verifier-lane-review.jsonl >/dev/null && echo "OK"
OK
$ jq -c . .beads/vb-jtqqx/waiver-candidates.jsonl >/dev/null && echo "OK"
OK
$ jq -c . .beads/vb-jtqqx/agent-invocation-ledger.jsonl >/dev/null && echo "OK"
OK
```

All 8 JSONL artifacts parse one object per line.

### 3. Status Lines

| Artifact | Required Status | Actual Status |
|---|---|---|
| `proof-plan-review.md` | APPROVED | **STATUS: APPROVED** (line 316) |
| `formal-verification-report.md` | PASS | **STATUS: PASS** for both PO-MAL-001 and PO-MAL-002 in the in-scope test file (line 432) |
| `black-hat-review.md` | APPROVED | **STATUS: APPROVED** (line 24 gate, line 188 verdict) |
| `verification-ledger.jsonl` | PASS for all rows | 2 rows, both `result: PASS`, `status: closed` |
| `formal-waivers.jsonl` | non-behavior | 6 rows, all `behavior_affecting: false`, all `status: approved` |
| `defects.md` | empty | empty (0 findings) |
| `truth-serum-report.md` | APPROVED | **STATUS: APPROVED** (line 261) |

All status lines are positive and supported by raw evidence.

### 4. No Merge Conflicts

```bash
$ ! rg -n '^(<<<<<<<|=======|>>>>>>>)' .beads/vb-jtqqx/
# Result: (no matches)
```

### 5. No Behavior-Affecting Waivers

```bash
$ jq -c '. | select(.behavior_affecting == true)' .beads/vb-jtqqx/formal-waivers.jsonl
# Result: (no rows)
```

All 6 formal waivers are non-behavior (bookkeeping for `not_applicable`
lanes per `verifier-lane-decisions.jsonl:2-7`).

### 6. Bridge Between Proof and Production

The strengthened tests in `crates/workspace_tests/tests/journal_side_index_contracts.rs:212-448`
(PO-008 block) call `vb_storage::keys::decode_storage_key` (the real
production function at `crates/vb_storage/src/keys.rs:346-434`) and
assert on `vb_storage::KeyDecodeError` (the real production enum at
`crates/vb_storage/src/error/key_decode.rs:8-31`). No mock, no
shadow, no test-only re-implementation. The proptest bodies ARE the
bridge between proof and production.

### 7. Active-Context Re-Run

```bash
$ rtk cargo test -p velvet-ballistics-workspace-tests --test journal_side_index_contracts
cargo test: 11 passed (1 suite, 0.42s)
exit=0
```

The canonical test command was re-run in this truth-serum execution
context. Result: 11 passed in 0.42s. Matches the formal-verifier
state-12 evidence.

### 8. Pre-existing Global Failures (out of scope)

The 5 pre-existing global failures (vb_compile compile errors, vb_core
admission proptest, workspace_tests strict-admission test,
edge_frame_pool / resource_frame_pool round-9 carryover, moon ci
unrelated lanes) are documented in `formal-verification-report.md#pre-existing-global-failures`
and are out of scope for this P1. They are identical on the parent
commit `rsvywymk` (verified by `jj new rsvywymk` +
`cargo check --workspace --all-targets` and other commands).
None are caused by the in-scope change.

## Summary

| Phase | Outcome |
|---|---|
| Phase 1 (Contract & Bead Parity) | PASS — all 18 SIDEX-MAL clauses covered, all source_refs exist |
| Phase 2 (Farley Engineering Rigor) | PASS — 0 production functions; proptest bodies are well-structured |
| Phase 3 (Holzman Rust Big 6) | PASS — 0 unsafe, 0 unwrap, 0 panic, 0 unchecked indexing in PO-008 block |
| Phase 4 (Ruthless Simplicity & DDD) | PASS — boring, correct, complete |
| Phase 5 (Bitter Truth) | PASS — 5 pre-existing global failures are out of scope |
| Black-hat review | STATUS: APPROVED, 0 findings |
| Formal verification | STATUS: PASS for both PO-MAL-001 and PO-MAL-002 in the in-scope test file |
| Truth-serum audit | STATUS: APPROVED, 0 findings, 10/10 adversarial checks PASS |
| Evidence packaging | STATUS: APPROVED |

## STATUS: APPROVED

The bead is approved for landing. The 11 tests in
`journal_side_index_contracts` pass with strengthened malformed-key
assertions. The decoder at `crates/vb_storage/src/keys.rs:346-434` is
unchanged and is the contract the tests verify. The 6 non-behavior
waivers are bookkeeping for `not_applicable` lanes; no
behavior-affecting waivers exist. The 5 pre-existing global failures
are documented and out of scope for this P1 test-only repair.

A state-14 row will be appended to
`.beads/vb-jtqqx/agent-invocation-ledger.jsonl` (sequence 7 of 7).

The bead is ready to be closed in the beads tracker and merged into
the parent jj workspace. No follow-up work is required for this
bead. The pre-existing global failures are tracked by other beads
(out of scope for vb-jtqqx).
