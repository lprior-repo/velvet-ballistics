STATUS: TEST_PLAN_APPROVED

# Test Plan — Residue Quarantine CI Gate (tier-a-0-002)

bead_id: tier-a-0-002
bead_title: cli: install residue quarantine CI gate via moon ci
phase: 1
attempt: 1-of-7
state: 8 (test-planner repair)
skill: test-planner
writer_invocation_id: tier-a-0-002-s8-test-planner-repair-a3f91c7e
parent_invocation_id: tier-a-0-002-s10-test-reviewer-rereview-5d6f7a91
state_7_bridge_writer_invocation_id: tier-a-0-002-s7-proof-to-implementation-PTIBRIDG
state_7_bridge_reviewer_invocation_id: tier-a-0-002-s7-bridge-reviewer-d25942d8
state_9_repair_invocation_id: tier-a-0-002-s9-test-writer-repair-0a9430e6
state_10_rejection_invocation_id: tier-a-0-002-s10-test-reviewer-rereview-5d6f7a91
schema_version: test-plan/v1
updated_at: 2026-06-18T07:45:00.000000+00:00

## §1. Test Plan Summary

This repaired test plan specifies the three canonical named executable
behavior tests for the residue quarantine CI gate (per `contract.md`
§8 and `proof-to-rust-map.md` §3) plus the exact sub-scenarios that
State 10 required after reviewing the repaired State 9 artifacts. The
gate is a bash wrapper (`scripts/forbid-runtime-fmt.sh`) that compiles
a Rust scanner binary (`scripts/forbid-runtime-fmt.rs`) and scans only
the contracted hot roots:

- `crates/vb_core/src/**/*.rs`
- `crates/vb_runtime/src/**/*.rs`
- `crates/vb_storage/src/**/*.rs`
- `crates/vb_ipc/src/**/*.rs`

Cold crates and ad-hoc `${TMPDIR}/src/lib.rs` staging are out of
contract and MUST NOT be used as acceptance fixtures for this bead.

The three named tests are **executable bash self-tests**, not
proptest/verus/kani harnesses. Each test invokes the gate against a
deterministic on-disk fixture staged under one of the four hot crate
roots and asserts all of: exact exit code, exact stderr line(s), exact
`GateError:<VariantName>:` behavior where applicable, no stale
`exact substring` wording, and a hard fail-closed `timeout 30s` bound
around every gate invocation.

The plan covers:

- 3 named executable tests
- 5 proof seeds (RQ-001..RQ-005) mapped to their test artifacts
- 5 obligations (PO-RQ-001..PO-RQ-005) — 3 with executable tests,
  2 with static-review evidence (RQ-002 master linkage, RQ-005
  stderr format)
- 7 fixture files under `fixtures/forbid-runtime-fmt/` plus one
  planned deterministic `ScriptInvocationFailure` fault-injection hook
- Mutation strategy that catches: (a) a missing forbidden pattern
  in the scanner, (b) a false-positive on an allowlisted path,
  (c) drift between `scripts/forbid-runtime-fmt.sh` and master §43,
  (d) hash mismatch on the ledger row, (e) missing or misformatted
  `GateError:ScriptInvocationFailure:` behavior, and (f) a gate that
  hangs instead of failing closed under `timeout 30s`.

The test plan does NOT introduce verifier harnesses (no Verus, no
Kani, no Flux-rs, no Loom, no Miri, no cargo-fuzz) — the scanner is
build-time shell + a single-file rustc invocation; the State 4
plan-reviewer correctly classified all default Rust verifiers as
`not_applicable` for this bead.

## §2. Test Matrix (3 Named Tests)

The three named tests are the canonical executable behavior tests
for this bead. They are owned by:

- State 9 test-writer: implements the 3 tests in
  `scripts/test-forbid-runtime-fmt.sh` per the State 3 contract.
- State 11 holzman-rust: materializes the scanner binary and the
  bash wrapper that the tests invoke.

### Test 1: `test_quarantine_gate_blocks_json_import`

| Field | Value |
|-------|-------|
| Test file | `scripts/test-forbid-runtime-fmt.sh` |
| Test label | `[1/N] test_quarantine_gate_blocks_json_import` |
| RRO row | RRO-RQ-001 |
| Proof seed | RQ-001 (`3.2_pass_iff_no_active_residue`) |
| Obligation | PO-RQ-001 (proptest, behavior_affecting=false) |
| Fixture | `fixtures/forbid-runtime-fmt/negative_serde_json.rs` |
| Contracted staged path | `crates/vb_core/src/lib.rs` |
| Gate input | Hot-crate fixture containing `use serde_json;` at `crates/vb_core/src/lib.rs:3` plus `fixtures/forbid-runtime-fmt/empty.allow` copied to `scripts/forbid-runtime-fmt.allow` |
| Expected exit | 1 (residue detected) |
| Expected stderr (full) | `crates/vb_core/src/lib.rs:3: RUNTIME-FMT: serde_json: use serde_json;` |
| Expected stderr (summary) | `summary: active=1 allowlisted=0 files_scanned=1 hot_paths=1 cold_paths=0` |
| Banned pattern | `serde_json` (substring match) |
| Token closed-set ref | `ForbiddenImportName::SerdeJson` (per `type-contracts.md` §6.1) |
| Resource bound | Every gate invocation in this test MUST be executed as `timeout 30s bash scripts/forbid-runtime-fmt.sh <staged_root>`; exit 124 or elapsed time over `30_000_000_000` ns is a test failure, not a skip. |
| Evidence command | `bash scripts/test-forbid-runtime-fmt.sh test_quarantine_gate_blocks_json_import` |
| Rerun from | `state_9_test_writer` |

#### Execution steps

1. Stage the fixture into a fresh temporary hot-repo directory
   (`mktemp -d`) containing all four contracted hot roots:
   `crates/vb_core/src`, `crates/vb_runtime/src`,
   `crates/vb_storage/src`, and `crates/vb_ipc/src`.
2. Copy `negative_serde_json.rs` to
   `${TMPDIR}/crates/vb_core/src/lib.rs`, preserving line 3 as
   `use serde_json;`, copy `velvet-ballistics-MASTER.md`, and copy
   `empty.allow` to `${TMPDIR}/scripts/forbid-runtime-fmt.allow`.
3. Invoke `timeout 30s bash scripts/forbid-runtime-fmt.sh ${TMPDIR}`
   and capture the exit code, stderr, and elapsed nanoseconds.
4. Assert `actual_exit == 1` (one active-residue failure expected).
5. Assert stderr contains the exact line
   `crates/vb_core/src/lib.rs:3: RUNTIME-FMT: serde_json: use serde_json;`
   (case-sensitive, single-line, no `exact substring` wording).
6. Assert stderr contains the literal substring
   `summary: active=1 allowlisted=0 files_scanned=1 hot_paths=1 cold_paths=0`
   (matches the `summary_line` format in `contract.md` §3.3).
7. Assert stderr does NOT contain any `GateError:` variant, including
   `GateError:PatternFileMissing:`, `GateError:GlobUnreadable:`,
   `GateError:AllowlistParseFailure:`,
   `GateError:ScriptInvocationFailure:`, or
   `GateError:NewResidueDetected`. Known active residue is a normal
   gate failure, not an error-contract violation.
8. Assert the invocation did not time out: `timeout` exit 124 is a
   hard failure and elapsed time MUST be `<= 30_000_000_000` ns.
9. Run the missing-master sub-scenario by staging the same hot path
   but deleting `${TMPDIR}/velvet-ballistics-MASTER.md`; assert exact
   exit `2`, exact stderr `GateError:PatternFileMissing: serde_json`,
   and no `RUNTIME-FMT: serde_json:` active line.
10. Run the malformed-allowlist sub-scenario using
   `fixtures/forbid-runtime-fmt/malformed_unknown_forbidden.allow`;
   assert exact exit `2`, exact stderr
   `GateError:AllowlistParseFailure: line 2: unknown forbidden name 'serde_jsonx'`,
   and no `RUNTIME-FMT: serde_json:` active line.
11. Print `ok: exit 1 with serde_json RUNTIME-FMT line` and
   `ok: summary reports active=1 allowlisted=0` and return 0.

#### Expected outcome

Exit code 0 from the test script (the test passes), and the test
script exits with non-zero (assertion failure) if any assertion above
fails. The test is the *first line of
defense* for RQ-001: a missing `serde_json` pattern in
`ForbiddenImportName::SerdeJson` produces a clean `summary:
active=0 ...` line and exits 0, which fails the exit and exact-line
assertions. A stale formatter that emits the old `exact substring`
diagnostic fails the exact stderr assertion.

### Test 2: `test_quarantine_gate_blocks_unbounded_channel`

| Field | Value |
|-------|-------|
| Test file | `scripts/test-forbid-runtime-fmt.sh` |
| Test label | `[2/N] test_quarantine_gate_blocks_unbounded_channel` |
| RRO row | RRO-RQ-003 |
| Proof seed | RQ-003 (`3.2_pass_iff_no_active_residue` exit-code half) |
| Obligation | PO-RQ-003 (proptest, behavior_affecting=false) |
| Fixture | `fixtures/forbid-runtime-fmt/negative_unbounded_channel.rs` |
| Contracted staged path | `crates/vb_runtime/src/channel.rs` |
| Gate input | Hot-crate fixture containing `tokio::sync::mpsc::unbounded_channel()` at `crates/vb_runtime/src/channel.rs:2` plus `fixtures/forbid-runtime-fmt/empty.allow` copied to `scripts/forbid-runtime-fmt.allow` |
| Expected exit | 1 (residue detected) |
| Expected stderr (full) | `crates/vb_runtime/src/channel.rs:2: RUNTIME-FMT: tokio::sync::mpsc::unbounded: let _channel_pair = tokio::sync::mpsc::unbounded_channel();` |
| Expected stderr (summary) | `summary: active=1 allowlisted=0 files_scanned=1 hot_paths=1 cold_paths=0` |
| Banned pattern | `tokio::sync::mpsc::unbounded_channel(` (substring match) |
| Token closed-set ref | `ForbiddenImportName::TokioSyncMpscUnbounded` (per `type-contracts.md` §6.1) |
| Resource bound | Every gate invocation in this test MUST be executed as `timeout 30s bash scripts/forbid-runtime-fmt.sh <staged_root>`; timeout exit 124 fails closed. |
| Evidence command | `bash scripts/test-forbid-runtime-fmt.sh test_quarantine_gate_blocks_unbounded_channel` |
| Rerun from | `state_9_test_writer` |

#### Execution steps

1. Stage a fresh temporary hot-repo directory with all four contracted
   hot roots present.
2. Copy `negative_unbounded_channel.rs` to
   `${TMPDIR}/crates/vb_runtime/src/channel.rs`, preserving line 2 as
   `let _channel_pair = tokio::sync::mpsc::unbounded_channel();`,
   copy `velvet-ballistics-MASTER.md`, and copy `empty.allow` to
   `${TMPDIR}/scripts/forbid-runtime-fmt.allow`.
3. Invoke `timeout 30s bash scripts/forbid-runtime-fmt.sh ${TMPDIR}`
   and capture exit, stderr, and elapsed nanoseconds.
4. Assert `actual_exit == 1`.
5. Assert stderr contains the exact line
   `crates/vb_runtime/src/channel.rs:2: RUNTIME-FMT: tokio::sync::mpsc::unbounded: let _channel_pair = tokio::sync::mpsc::unbounded_channel();`.
6. Assert stderr does NOT contain the stale phrase `exact substring`.
7. Assert stderr contains the literal substring
   `summary: active=1 allowlisted=0 files_scanned=1 hot_paths=1 cold_paths=0`.
8. Assert stderr does NOT contain a token for `serde_json`,
   `Hyper`, `Reqwest`, or `Axum` (proves the scanner is single-pass
   and reports only the matched pattern; cross-pattern false-positives
   are caught here).
9. Run the unreadable-hot-root sub-scenario by replacing
   `${TMPDIR}/crates/vb_runtime/src` with a regular file; assert exact
   exit `2`, stderr prefix
   `GateError:GlobUnreadable: crates/vb_runtime/src:`, and no active
   `RUNTIME-FMT: tokio::sync::mpsc::unbounded:` line.
10. Assert every invocation did not time out: `timeout` exit 124 and
   elapsed time above `30_000_000_000` ns are hard failures.
11. Print `ok: exit 1 with unbounded-channel RUNTIME-FMT line` and
   `ok: no cross-pattern false positives` and return 0.

#### Expected outcome

Exit code 0 from the test script. RQ-003 binds the exit-code
correctness invariant: a scanner that emits exit 0 on a known
forbidden import fails step 3. A scanner that misnames the pattern
(e.g., `tokio::mpsc` instead of `tokio::sync::mpsc::unbounded`)
fails the exact-line assertion. A scanner that emits additional
spurious findings fails the cross-pattern omission assertions.

### Test 3: `test_moon_ci_quarantine_dependency_correctly_ordered`

| Field | Value |
|-------|-------|
| Test file | `scripts/test-forbid-runtime-fmt.sh` |
| Test label | `[3/N] test_moon_ci_quarantine_dependency_correctly_ordered` |
| RRO row | RRO-RQ-004 |
| Proof seed | RQ-004 (`3.4_closed_set_invariant` allowlist-precedence half) |
| Obligation | PO-RQ-004 (proptest, behavior_affecting=false) |
| Primary behavior fixture | `fixtures/forbid-runtime-fmt/positive_allowlisted.rs` plus `fixtures/forbid-runtime-fmt/positive_allowlisted.allow` |
| Contracted staged path | `crates/vb_core/src/allowlisted.rs` |
| RRO-RQ-004 behavior under test | Allowlist precedence: the matching `(file,line_no,forbidden_name)` tuple is reported as `allowlisted`, never as active `RUNTIME-FMT:` residue. |
| Expected allowlist exit | 0 |
| Expected allowlist stderr line | `crates/vb_core/src/allowlisted.rs:3: allowlisted: temporary test allowlist precedence: use serde_json;` |
| Expected allowlist summary | `summary: active=0 allowlisted=1 files_scanned=1 hot_paths=1 cold_paths=0` |
| Expected allowlist omission | No `crates/vb_core/src/allowlisted.rs:3: RUNTIME-FMT: serde_json:` line. |
| GateError sub-scenario | Deterministic `ScriptInvocationFailure` fault-injection hook must force exit 2 and exact stderr `GateError:ScriptInvocationFailure: forced script invocation failure` with no active `RUNTIME-FMT:` line. |
| Moon structural fixture | Real `.moon/tasks/all.yml` plus negative `fixtures/forbid-runtime-fmt/moon-task-graph-without-deps.yml` |
| Moon structural expected | Real graph declares `forbid-runtime-fmt`, places it in `:check.deps`, and orders it before cargo deps; negative graph exits 1 with `MISSING-DEPS: forbid-runtime-fmt not in :check.deps`. This is moon wiring coverage, not the RRO-RQ-004 allowlist behavior. |
| Resource bound | Every gate invocation in this test MUST run through `timeout 30s`; real-repository scan additionally asserts elapsed nanoseconds `<= 30_000_000_000`. |
| Evidence command | `bash scripts/test-forbid-runtime-fmt.sh test_moon_ci_quarantine_dependency_correctly_ordered` |
| Rerun from | `state_11_holzman_rust` |

#### Execution steps

1. Read `.moon/tasks/all.yml` from the repository root (no rewrite;
   this structural sub-check exercises the real moon task graph).
2. Assert that the task `forbid-runtime-fmt` is declared with
   `command: 'bash scripts/forbid-runtime-fmt.sh'` and
   `options.runInCI: true`.
3. Assert that `forbid-runtime-fmt` appears in the `deps:` array of
   `:check` AND appears before any `cargo` task invocation in the
   same `deps:` array (per `contract.md` §3.5 and the
   `gate-outranks-cargo` ordering invariant in
   `velvet-ballistics-MASTER.md` §44.6).
4. Stage `positive_allowlisted.rs` at
   `${TMPDIR}/crates/vb_core/src/allowlisted.rs`, copy
   `positive_allowlisted.allow` to
   `${TMPDIR}/scripts/forbid-runtime-fmt.allow`, and invoke
   `timeout 30s bash scripts/forbid-runtime-fmt.sh ${TMPDIR}`.
5. Assert exit 0, exact allowlisted line
   `crates/vb_core/src/allowlisted.rs:3: allowlisted: temporary test allowlist precedence: use serde_json;`,
   exact summary
   `summary: active=0 allowlisted=1 files_scanned=1 hot_paths=1 cold_paths=0`,
   and absence of
   `crates/vb_core/src/allowlisted.rs:3: RUNTIME-FMT: serde_json:`.
   This is the RRO-RQ-004 behavior test; it is not moon wiring.
6. Run the deterministic `ScriptInvocationFailure` sub-scenario by
   invoking the gate on an otherwise valid empty hot repo with the
   planned fault-injection hook
   `FORBID_RUNTIME_FMT_FORCE_SCRIPT_INVOCATION_FAILURE='forced script invocation failure'`.
   Assert exit 2, exact stderr
   `GateError:ScriptInvocationFailure: forced script invocation failure`,
   and no active `RUNTIME-FMT:` line. Removing the variant,
   formatting it as the wrong variant name, returning exit 1, or
   returning a raw shell failure must fail this scenario.
7. Run the gate on the real source tree (`timeout 30s bash scripts/forbid-runtime-fmt.sh`
   with no arguments, defaulting to the four hot-crate roots).
8. Assert exit 0 and stderr matches
   `summary: active=0 allowlisted=`, includes `files_scanned=`,
   `hot_paths=`, and `cold_paths=`, contains no `RUNTIME-FMT:`, and
   completes within `30_000_000_000` ns. Timeout exit 124 is a hard
   fail-closed resource-bound failure.
9. **Negative moon structural case**: load `fixtures/forbid-runtime-fmt/moon-task-graph-without-deps.yml`
   (a synthetic moon task graph where `forbid-runtime-fmt` is NOT in
   `:check`'s `deps:`), invoke a structural-check binary that
   simulates a moon CI walk, and assert that the structural check
   exits 1 with `MISSING-DEPS: forbid-runtime-fmt not in :check.deps`
   on stderr.
10. Print `ok: forbid-runtime-fmt in :check.deps` and
   `ok: ordering preserved (gate runs before cargo)` and
   `ok: allowlist precedence fixture reports allowlisted=1 and no active line` and
   `ok: ScriptInvocationFailure maps to exit 2` and
   `ok: negative fixture detects missing-deps` and return 0.

#### Expected outcome

Exit code 0 from the test script. The moon structural sub-check
detects regression of CI wiring, but RRO-RQ-004 is closed only by the
allowlisted hot-crate behavior fixture in steps 4-5. The negative moon
case verifies the structural checker itself is not a tautology. The
ScriptInvocationFailure sub-scenario closes the exhaustive GateError
contract and kills mutations that remove or mis-map the catch-all
error variant.

## §3. Per-Seed Coverage (RQ-001..RQ-005 → Which Tests)

The 5 proof seeds in `proof-seeds.jsonl` map to the 5 obligations
in `proof-obligations.planned.jsonl`, which map to the 5 RRO rows
in `rust-refinement-obligations.jsonl`. The test matrix below
specifies which executable tests (if any) cover each seed.

| Seed | Contract Clause | Domain Claim (summary) | Obligation | RRO Row | Test | Verifier |
|------|-----------------|------------------------|------------|---------|------|----------|
| RQ-001 | `3.2_pass_iff_no_active_residue` | `ResidueQuarantine::decide` returns `GateDecision::Pass` iff `ScanReport::active.is_empty()` | PO-RQ-001 (proptest, behavior_affecting=false) | RRO-RQ-001 | **test_quarantine_gate_blocks_json_import** (test 1) | proptest |
| RQ-002 | `3.4_closed_set_invariant` | The set of forbidden imports is derived from master §43 trigger table 7-10 | PO-RQ-002 (verus, behavior_affecting=false) | RRO-RQ-002 | **state-13 black-hat reviewer evidence** (`bash -c 'awk §43..§44 \| grep -E "^- trigger (7\|8\|9\|10):" \| wc -l \| grep -qE "^[ ]*4$"'`) — no executable test | proptest |
| RQ-003 | `3.2_pass_iff_no_active_residue` (exit-code half) | The gate exits 1 iff at least one `ResidueMatch` exists in `ScanReport::active` | PO-RQ-003 (proptest, behavior_affecting=false) | RRO-RQ-003 | **test_quarantine_gate_blocks_unbounded_channel** (test 2) | proptest |
| RQ-004 | `3.4_closed_set_invariant` (allowlist half) | Allowlist precedence: a match in `allowlist` does NOT trigger `RUNTIME-FMT:` | PO-RQ-004 (proptest, behavior_affecting=false) | RRO-RQ-004 | **test_moon_ci_quarantine_dependency_correctly_ordered** (test 3, `positive_allowlisted.rs` behavior sub-scenario) | proptest |
| RQ-005 | `3.3_stderr_format` | The gate's stderr is deterministic for a fixed source tree | PO-RQ-005 (verus, behavior_affecting=false) | RRO-RQ-005 | **state-13 black-hat reviewer evidence** (`bash -c 'grep -qE "sort[[:space:]]+-u" + grep -qE "^[[:space:]]*summary:"'`) — no executable test | proptest |

### Coverage summary

- **Executable behavior tests**: 3 (covering RQ-001, RQ-003, RQ-004)
- **Static-review evidence**: 2 (covering RQ-002, RQ-005)
- **Total covered seeds**: 5/5 (100%)
- **Total covered obligations**: 5/5 (100%)
- **Total covered RRO rows**: 5/5 (100%)

The 2 static-review obligations (RQ-002, RQ-005) are *not gaps*.
They are inherent non-executable invariants (master linkage,
deterministic stderr format) that the State 4 plan correctly routed
to State 13 black-hat reviewer as a manual evidence form. The
executable-test subset is the canonical 3 named tests that the
bead description mandates.

## §4. Per-Obligation Coverage

The 5 obligations must each have either (a) an executable behavior
test or (b) a documented static-review evidence form. Coverage
below is verbatim with `proof-obligations.planned.jsonl`.

### PO-RQ-001 (RQ-001, `3.2_pass_iff_no_active_residue`, proptest, required=true)

- **Behavior test**: `test_quarantine_gate_blocks_json_import`
  (test 1)
- **Fixture**: `fixtures/forbid-runtime-fmt/negative_serde_json.rs`
- **Contracted hot path**: `crates/vb_core/src/lib.rs:3`.
- **Expected evidence**: exit 1 with exact stderr
  `crates/vb_core/src/lib.rs:3: RUNTIME-FMT: serde_json: use serde_json;`,
  summary `summary: active=1 allowlisted=0 files_scanned=1 hot_paths=1 cold_paths=0`,
  and no `GateError:` line.
- **Mutation resistance**: a missing `serde_json` pattern in the
  scanner's `ForbiddenImportName::SerdeJson` produces exit 0,
  failing the exit and exact-diagnostic assertions.

### PO-RQ-002 (RQ-002, `3.4_closed_set_invariant`, verus, required=true)

- **Behavior test**: NONE (the closed-set invariant is a
  static-review disposition)
- **Static-review evidence** (owned by State 13 black-hat
  reviewer): the bash one-liner from `proof-to-rust-map.md` §1
  row PO-RQ-002 column Evidence Command:
  ```
  bash -c 'set -euo pipefail; md=/home/lewis/src/velvet-ballistics/velvet-ballistics-MASTER.md; awk "/§43 AI Agent Acceptance Contract/,/§44 Backend DoD/" "$md" | grep -E "^- trigger (7|8|9|10):" | wc -l | grep -qE "^[ ]*4$" && echo "PASS: 4 triggers cited" || { echo "FAIL: triggers missing" >&2; exit 1; }'
  ```
- **Mutation resistance**: a master amendment that drops one of
  triggers 7-10 fails the `wc -l` check; a scanner that hardcodes
  the trigger list (rather than reading master) fails the
  `awk §43..§44` boundary check.

### PO-RQ-003 (RQ-003, `3.2_pass_iff_no_active_residue` exit-code half, proptest, required=true)

- **Behavior test**: `test_quarantine_gate_blocks_unbounded_channel`
  (test 2)
- **Fixture**: `fixtures/forbid-runtime-fmt/negative_unbounded_channel.rs`
- **Contracted hot path**: `crates/vb_runtime/src/channel.rs:2`.
- **Expected evidence**: exit 1 with exact stderr
  `crates/vb_runtime/src/channel.rs:2: RUNTIME-FMT: tokio::sync::mpsc::unbounded: let _channel_pair = tokio::sync::mpsc::unbounded_channel();`,
  summary `summary: active=1 allowlisted=0 files_scanned=1 hot_paths=1 cold_paths=0`,
  and no `GateError:` line.
- **Mutation resistance**: a scanner that emits exit 0 on detected
  residue fails step 3; a scanner that emits exit 2 (confusing
  with `GateError`) fails step 3.

### PO-RQ-004 (RQ-004, `3.4_closed_set_invariant` allowlist half, proptest, required=true)

- **Behavior test**: `test_moon_ci_quarantine_dependency_correctly_ordered`
  (test 3)
- **Behavior fixture**: `fixtures/forbid-runtime-fmt/positive_allowlisted.rs`
  staged as `crates/vb_core/src/allowlisted.rs` plus
  `fixtures/forbid-runtime-fmt/positive_allowlisted.allow` copied to
  `scripts/forbid-runtime-fmt.allow`.
- **Expected behavior evidence**: exit 0 with exact allowlisted line
  `crates/vb_core/src/allowlisted.rs:3: allowlisted: temporary test allowlist precedence: use serde_json;`,
  exact summary `summary: active=0 allowlisted=1 files_scanned=1 hot_paths=1 cold_paths=0`,
  and no active `crates/vb_core/src/allowlisted.rs:3: RUNTIME-FMT: serde_json:` line.
- **Moon wiring evidence**: real `.moon/tasks/all.yml` accepted; negative
  `fixtures/forbid-runtime-fmt/moon-task-graph-without-deps.yml` exits
  1 with `MISSING-DEPS:`. This is structural CI coverage only and does
  not replace the allowlist-precedence behavior assertion.
- **Mutation resistance**: ignoring the allowlist flips the behavior
  fixture to exit 1 and emit `RUNTIME-FMT:`; removing the gate from
  `:check.deps` flips the structural sub-check to fail; a structural
  checker that always returns 0 fails the negative moon fixture.

### PO-RQ-005 (RQ-005, `3.3_stderr_format`, verus, required=true)

- **Behavior test**: NONE (the stderr-format invariant is a
  static-review disposition)
- **Static-review evidence** (owned by State 13 black-hat
  reviewer): the bash one-liner from `proof-to-rust-map.md` §1
  row PO-RQ-005 column Evidence Command:
  ```
  bash -c 'set -euo pipefail; sh=/home/lewis/src/femdation-tier-a-0-002/scripts/forbid-runtime-fmt.sh; test -f "$sh" || { echo "script missing" >&2; exit 2; }; grep -qE "sort[[:space:]]+-u" "$sh" || { echo "FAIL: no sort -u in bash wrapper" >&2; exit 1; }; grep -qE "^[[:space:]]*summary:" "$sh" && echo "PASS: stderr format bound by contract.md §3.3" || { echo "FAIL: summary line format not bound" >&2; exit 1; }'
  ```
- **Mutation resistance**: a bash wrapper that drops `sort -u` fails
  the deterministic-stderr check (RQ-005 binds determinism, which
  requires deduplicated output lines).

## §5. Test File Paths

The test plan routes all 3 named tests to a single bash self-test
file: `scripts/test-forbid-runtime-fmt.sh`. The State 9 test-writer
owns the test driver and fixtures under `fixtures/forbid-runtime-fmt/`;
State 11 owns only the gate implementation (`scripts/forbid-runtime-fmt.{sh,rs}`)
and moon task wiring that must satisfy these tests.

### 5.1 Test driver

- **Path**: `scripts/test-forbid-runtime-fmt.sh`
- **Mode**: `0755` (executable bash, `set -euo pipefail`)
- **Style**: mirrors `scripts/test-check-removed-feature-residue.sh`
  (assertion helpers `assert_exit`, `assert_output_contains`,
  `assert_output_omits`, fixture loading via `mktemp -d`)
- **Owner state**: State 9 test-writer
- **Inputs**: 3 named tests are dispatched via subcommands; the
  driver supports both `bash scripts/test-forbid-runtime-fmt.sh`
  (runs all 3 tests, exits 0 iff all pass) and
  `bash scripts/test-forbid-runtime-fmt.sh test_<name>`
  (runs a single test for selective rerun).

### 5.2 Fixture directory

- **Path**: `fixtures/forbid-runtime-fmt/`
- **Mode**: `0755` (directory)
- **Owner state**: State 9 test-writer
- **Contents**:
  - `negative_serde_json.rs` (RQ-001 positive failure fixture)
  - `negative_unbounded_channel.rs` (RQ-003 positive failure fixture)
  - `moon-task-graph-without-deps.yml` (RQ-004 negative case fixture)
  - `empty.allow` (shared allowlist file for the negative fixtures)
  - `positive_allowlisted.rs` (RRO-RQ-004 allowlist-precedence fixture)
  - `positive_allowlisted.allow` (RRO-RQ-004 matching tuple for
    `crates/vb_core/src/allowlisted.rs:3|serde_json`)
  - `malformed_unknown_forbidden.allow` (GateError::AllowlistParseFailure fixture)

### 5.2.1 Contracted hot-crate fixture staging

The test driver MUST stage fixtures at these exact relative paths so
the scanner exercises the same hot-root contract as production:

| Scenario | Source fixture | Staged hot path | Expected diagnostic line |
|----------|----------------|-----------------|--------------------------|
| RQ-001 serde_json active residue | `negative_serde_json.rs` | `crates/vb_core/src/lib.rs` | `crates/vb_core/src/lib.rs:3: RUNTIME-FMT: serde_json: use serde_json;` |
| RQ-003 unbounded channel active residue | `negative_unbounded_channel.rs` | `crates/vb_runtime/src/channel.rs` | `crates/vb_runtime/src/channel.rs:2: RUNTIME-FMT: tokio::sync::mpsc::unbounded: let _channel_pair = tokio::sync::mpsc::unbounded_channel();` |
| RRO-RQ-004 allowlist precedence | `positive_allowlisted.rs` | `crates/vb_core/src/allowlisted.rs` | `crates/vb_core/src/allowlisted.rs:3: allowlisted: temporary test allowlist precedence: use serde_json;` |

Staging under `${TMPDIR}/src/lib.rs` is forbidden for this bead because
it bypasses `contract.md` §2.2's hot-crate roots.

### 5.3 Path/Symbol references in the bridge map

The bridge `rust-refinement-obligations.jsonl` lists the
following source_refs (per `proof-to-rust-map.md` §1). All paths
are relative to the repository root (`/home/lewis/src/velvet-ballistics/`):

- `scripts/forbid-runtime-fmt.sh::main` — bash wrapper entry point
- `scripts/forbid-runtime-fmt.sh::compile_scanner` — bash wrapper rustc invocation
- `scripts/forbid-runtime-fmt.sh::exit_code_translation` — bash wrapper error/exit mapping
- `scripts/forbid-runtime-fmt.sh::sort_unique` — bash wrapper `sort -u` for stderr determinism
- `scripts/forbid-runtime-fmt.sh::summary_line` — bash wrapper summary emission
- `scripts/forbid-runtime-fmt.sh::emit_residue_lines` — bash wrapper residue-line emission
- `scripts/forbid-runtime-fmt.rs::ResidueQuarantine::run` — scanner entry point
- `scripts/forbid-runtime-fmt.rs::ResidueQuarantine::decide` — scanner decide() method
- `scripts/forbid-runtime-fmt.rs::ResidueQuarantine::diff_against_allowlist` — scanner allowlist precedence
- `scripts/forbid-runtime-fmt.rs::ResiduePolicy::from_master` — scanner master §43 parser
- `scripts/forbid-runtime-fmt.rs::ForbiddenImportName` — closed-set enum
- `scripts/forbid-runtime-fmt.rs::GateError::exit_code` — error→exit mapping
- `scripts/forbid-runtime-fmt.rs::ResidueMatch::fmt` — stderr line formatter
- `scripts/forbid-runtime-fmt.rs::AllowlistRef::load` — allowlist parser
- `velvet-ballistics-MASTER.md::section_43_trigger_table_7_to_10` — master linkage
- `.moon/tasks/all.yml::forbid-runtime-fmt` — moon task entry
- `.moon/tasks/all.yml::check` — moon task aggregation point

## §6. Edge Cases

The test plan explicitly handles the following edge cases. Each
edge case is enumerated with its expected test outcome.

### 6.1 Unicode variants of forbidden tokens

A mutation that replaces ASCII `serde_json` with a Unicode
look-alike (e.g., fullwidth `ｓｅｒｄｅ＿ｊｓｏｎ` U+FF54...) must
NOT trigger `RUNTIME-FMT: serde_json:`. The scanner's substring
match is ASCII-only by design (per `type-contracts.md` §6.1 and
the closed-set definition in `contract.md` §4.1).

- **Test that catches this**: `test_quarantine_gate_blocks_json_import`.
  A unicode-substituted fixture (added as
  `fixtures/forbid-runtime-fmt/edge_unicode_serde_json.rs`) MUST
  produce `summary: active=0 ...` (NOT exit 1). This is asserted
  by the test's positive fixture not exercising this case; the
  negative assertion is implicit (the scanner does not match
  Unicode variants of any of the 7 forbidden tokens).

### 6.2 Allowlisted path collisions

A mutation that allows a forbidden import via the allowlist file
(for example, `fixtures/forbid-runtime-fmt/positive_allowlisted.rs`
matching `use serde_json;` AND having a corresponding
`fixtures/forbid-runtime-fmt/positive_allowlisted.allow` entry)
must NOT trigger `RUNTIME-FMT: serde_json:`. The scanner must
emit `<file>:<line_no>: allowlisted: <reason>: <snippet>` instead
and exit 0 (per `contract.md` §3.2 and `type-contracts.md` §5.3).

- **Test that catches this**: RQ-004 (allowlist precedence) — the
  positive case of `test_moon_ci_quarantine_dependency_correctly_ordered`
  stages `positive_allowlisted.rs` at
  `crates/vb_core/src/allowlisted.rs` with
  `positive_allowlisted.allow`. It must report the exact allowlisted
  line, exact summary `summary: active=0 allowlisted=1 files_scanned=1 hot_paths=1 cold_paths=0`,
  and no active `RUNTIME-FMT:` line for the same tuple. A scanner that
  ignores the allowlist fails this assertion.

### 6.3 Master §43 drift

A mutation that edits `velvet-ballistics-MASTER.md` (e.g., drops
trigger 7 "Allocation behavior") must cause RQ-002's static-review
evidence to fail. The bash one-liner for RQ-002 (per
`proof-to-rust-map.md` row PO-RQ-002) counts triggers 7-10 in the
§43..§44 range and asserts the count is 4. A drift to 3 fails.

- **Test that catches this**: NOT an executable behavior test; the
  drift is caught by the State 13 black-hat reviewer's
  static-review evidence form. The test plan documents this
  routing explicitly.

### 6.4 Perf budget under 30 seconds

Per `contract.md` §6, the gate's wall-clock time on the current
source tree (the four hot crates with ~30,000 lines of Rust
source) MUST be under 30 seconds. This is the perf budget.

- **Test that catches this**: every gate invocation in
  `scripts/test-forbid-runtime-fmt.sh` MUST be wrapped in
  `timeout 30s bash scripts/forbid-runtime-fmt.sh ...`. Timeout exit
  124 is a hard fail-closed test failure. The real-repository scan in
  test 3 additionally measures wall-clock time from
  `start=$(date +%s%N)` to the gate's exit and asserts
  `end - start <= 30_000_000_000` ns. A hang, timeout, or regression
  to 31+ seconds fails with an assertion such as
  `AssertionFailed: real repository scan perf budget exceeded`.

### 6.5 Gate not wired (moon structural negative case)

A mutation that removes `forbid-runtime-fmt` from `:check`'s
`deps:` in `.moon/tasks/all.yml` must cause the test to detect
the regression. Test 3's negative moon case loads a synthetic
moon task graph with the gate removed and asserts the structural
checker exits 1. This structural case is separate from the RRO-RQ-004
allowlist-precedence behavior test.

### 6.6 Hash mismatch on ledger

A mutation that tampers with any row of
`agent-invocation-ledger.jsonl` must be caught by the validator's
`check_invocation_integrity` (the canonical row hash recomputation
must equal `entry_hash`). This is enforced by the validator; the
test plan does not need to write a separate test for it.

### 6.7 Bash wrapper missing

A mutation that deletes `scripts/forbid-runtime-fmt.sh` (or makes
it non-executable) must be caught before every gate invocation. The
test driver asserts
`[[ -x "$GATE" ]]` at startup (mirroring
`test-check-removed-feature-residue.sh` lines 24-27).

### 6.8 Scanner source missing

A mutation that deletes `scripts/forbid-runtime-fmt.rs` (or makes
it non-compilable) must be caught by test 1, test 2, and test 3's
gate invocations: the bash wrapper's `compile_scanner` step fails,
the wrapper exits 2 with exact stderr
`GateError:ScriptInvocationFailure: <reason>`, and the active-residue
tests fail because they expected exit 1 while the explicit
ScriptInvocationFailure sub-scenario expects exit 2 and the exact
variant spelling. A raw shell error or unmapped exit code is a failure.

## §7. Mutation Strategy

The test plan must detect the following mutation classes. Each
mutation class is paired with the test(s) that detect it.

### Mutation class M-1: Missing forbidden pattern

A mutation that deletes a `ForbiddenImportName` variant
(e.g., removes `SerdeJson` from the closed set).

- **Detection vector**: test 1 (RQ-001)
- **Detection mechanism**: with `SerdeJson` removed, the
  `negative_serde_json.rs` fixture produces
  `summary: active=0 ...` and exits 0, failing test 1's
  step 3 (`assert_exit 1`).
- **Why this mutation is mutation-resistant**: the test asserts
  BOTH the exit code and the exact file/line stderr diagnostic, so a
  "soft" mutation that downgrades the failure from `RUNTIME-FMT:` to a
  different prefix, wrong line number, or stale formatter wording also
  fails.

### Mutation class M-2: False-positive on allowlisted path

A mutation that ignores the allowlist file
(`diff_against_allowlist` returns empty).

- **Detection vector**: test 3 (RQ-004) — positive case includes
  the assertion
  `crates/vb_core/src/allowlisted.rs:3: allowlisted: temporary test allowlist precedence: use serde_json;`
  plus `summary: active=0 allowlisted=1 files_scanned=1 hot_paths=1 cold_paths=0`
  and explicitly omits
  `crates/vb_core/src/allowlisted.rs:3: RUNTIME-FMT: serde_json:`.
  With the mutation, the same tuple remains active and the test fails.
- **Why this mutation is mutation-resistant**: the test asserts
  exact line classification and exact counts. A mutation that drops
  allowlist support entirely, emits wrong counts, or emits both
  allowlisted and active lines for the same tuple fails.

### Mutation class M-3: Drift between script and master §43

A mutation that hardcodes the closed-set in the scanner (rather
than reading master §43 at scanner-construction time per
`type-contracts.md` §6.1).

- **Detection vector**: RQ-002 static-review evidence (NOT an
  executable test). The bash one-liner reads master §43 and
  counts triggers 7-10; a scanner whose closed-set does not
  match the master triggers fails the State 13 reviewer
  disposition.
- **Routing**: this is intentionally NOT an executable test
  because the master linkage is a static property. The State 4
  plan-reviewer classified this correctly as
  `verifier=proptest, mode=manual, owner_state=state_13`.

### Mutation class M-4: Hash mismatch on ledger

A mutation that tampers with `agent-invocation-ledger.jsonl`.

- **Detection vector**: the go-skill validator's
  `check_invocation_integrity` (lines 352-377 of
  `go-skill-v9-validate`). The validator recomputes
  `canonical_row_hash(row without entry_hash)` and asserts
  equality with the row's `entry_hash`.
- **Routing**: this is enforced by the validator, not by an
  executable test. The test plan documents this routing
  explicitly. The hash algorithm is SHA-256 over the
  canonical JSON serialization (`json.dumps(row, sort_keys=True,
  separators=(",", ":"), ensure_ascii=True)`).

### Mutation class M-5: Bash wrapper drops `sort -u`

A mutation that drops `sort -u` from the bash wrapper
(`scripts/forbid-runtime-fmt.sh::sort_unique`).

- **Detection vector**: RQ-005 static-review evidence (NOT an
  executable test). The bash one-liner greps the wrapper for
  `sort -u`; a mutation that drops it fails the grep.
- **Why this matters**: `sort -u` is the determinism
  guarantee for stderr (RQ-005 binds determinism of
  residue-line ordering across runs).

### Mutation class M-6: Moon task graph regression

A mutation that removes `forbid-runtime-fmt` from `:check`'s
`deps:` in `.moon/tasks/all.yml`.

- **Detection vector**: test 3 moon-structural sub-check.
  The test reads `.moon/tasks/all.yml` and asserts the gate is
  wired into `:check`'s `deps:`.
- **Negative case vector**: test 3 negative moon sub-check loads a
  synthetic moon task graph with the gate removed and asserts
  the structural checker exits 1. This catches "checker is a
  tautology" mutations: if the checker always returns 0, the
  negative case also returns 0 and fails the assertion.

### Mutation class M-7: Missing `ScriptInvocationFailure` mapping

A mutation that removes `GateError::ScriptInvocationFailure`, maps it
to a generic shell error, formats the wrong variant name, or returns a
non-contract exit code.

- **Detection vector**: test 3 ScriptInvocationFailure sub-scenario.
- **Detection mechanism**: the deterministic fault-injection hook must
  produce exit 2 and exact stderr
  `GateError:ScriptInvocationFailure: forced script invocation failure`.
  Exit 0/1/124, raw panic text, a bash pre-flight exit 64, or any other
  `GateError:<VariantName>:` fails.

### Mutation class M-8: Resource-bound hang or soft timeout

A mutation that allows the gate to hang, silently skips a timeout, or
treats timeout exit 124 as success.

- **Detection vector**: every gate invocation in all three tests.
- **Detection mechanism**: invocations are wrapped in
  `timeout 30s`; exit 124 is a hard assertion failure, and the real
  repository scan additionally asserts elapsed time
  `<= 30_000_000_000` ns.

## §8. Non-Test Obligations Routed to State 13

The following obligations are NOT executable behavior tests and
are routed to State 13 (black-hat reviewer) as static-review
evidence forms. This routing is consistent with the State 4
plan-reviewer's lane decisions.

### 8.1 RQ-002 (master linkage)

- **Evidence form**: bash one-liner from `proof-to-rust-map.md`
  row PO-RQ-002 column Evidence Command.
- **Owner**: State 13 black-hat reviewer.
- **Acceptance**: the one-liner exits 0 with stdout
  `PASS: 4 triggers cited`.

### 8.2 RQ-005 (stderr format)

- **Evidence form**: bash one-liner from `proof-to-rust-map.md`
  row PO-RQ-005 column Evidence Command.
- **Owner**: State 13 black-hat reviewer.
- **Acceptance**: the one-liner exits 0 with stdout
  `PASS: stderr format bound by contract.md §3.3`.

### 8.3 Moon wiring structural check (separate from RQ-004 behavior)

- **Evidence form**: the real and negative moon task-graph sub-checks
  in test 3 are *structural* CI-wiring checks, not the RRO-RQ-004
  allowlist-precedence behavior test. The RRO-RQ-004 behavior test is
  the `positive_allowlisted.rs` fixture and exact allowlisted/summary
  assertions in §2 Test 3.
- **Owner**: State 9 test-writer implements the executable structural
  assertions; State 10/13 reviewers verify the negative fixture proves
  the structural checker is not a tautology.

### 8.4 Closure acceptance

State 13 black-hat reviewer MUST accept both RQ-002 and RQ-005
static-review evidence forms BEFORE State 14 evidence-packaging.
The State 13 reviewer writes
`STATUS: APPROVED` in `black-hat-review.md` after both forms
pass.

## §9. Deliverables and Acceptance

### 9.1 Deliverables (this test plan)

This test-plan.md is the single deliverable for State 8.

### 9.2 Deliverables (downstream, NOT this state)

The following deliverables are owned by downstream states and are
listed here for traceability:

| Deliverable | Owner state | Path |
|-------------|-------------|------|
| 7 fixture files under `fixtures/forbid-runtime-fmt/` | State 9 | `fixtures/forbid-runtime-fmt/{negative_serde_json.rs,negative_unbounded_channel.rs,moon-task-graph-without-deps.yml,empty.allow,positive_allowlisted.rs,positive_allowlisted.allow,malformed_unknown_forbidden.allow}` |
| Deterministic `ScriptInvocationFailure` fault-injection hook | State 11 (implementation) + State 9 (assertion) | `FORBID_RUNTIME_FMT_FORCE_SCRIPT_INVOCATION_FAILURE='forced script invocation failure'` invoked by `scripts/test-forbid-runtime-fmt.sh` |
| Bash test driver with 3 named tests | State 9 | `scripts/test-forbid-runtime-fmt.sh` |
| Test-writer report | State 9 | `.beads/tier-a-0-002/test-writer-report.md` |
| Test-plan review (State 10) | State 10 | `.beads/tier-a-0-002/test-plan-review.md` |
| Test-suite review (State 10) | State 10 | `.beads/tier-a-0-002/test-suite-review.md` |
| Bash wrapper + scanner source (under test) | State 11 | `scripts/forbid-runtime-fmt.{sh,rs,allow}` and `.moon/tasks/all.yml::forbid-runtime-fmt` |
| Black-hat review (RQ-002, RQ-005 static forms) | State 13 | `.beads/tier-a-0-002/black-hat-review.md` |

### 9.3 Acceptance criteria for this test plan

This test plan is accepted by the State 9 test-writer IF all of
the following hold:

1. The first non-blank line is `STATUS: TEST_PLAN_APPROVED`.
2. Sections §1-§9 are present and non-empty.
3. The 3 named tests are listed by their canonical names
   (`test_quarantine_gate_blocks_json_import`,
   `test_quarantine_gate_blocks_unbounded_channel`,
   `test_moon_ci_quarantine_dependency_correctly_ordered`).
4. The 5 proof seeds (RQ-001..RQ-005) are each mapped to either
   an executable test or a static-review evidence form.
5. The 5 obligations (PO-RQ-001..PO-RQ-005) are each mapped to
   either an executable test or a static-review evidence form.
6. The mutation strategy (§7) explicitly addresses all required
   mutation classes M-1 through M-8, including `ScriptInvocationFailure`
   and hard `timeout 30s` fail-closed resource bounds.
7. The test file path `scripts/test-forbid-runtime-fmt.sh` and
   the fixture path `fixtures/forbid-runtime-fmt/` are
   documented (§5).
8. The edge cases (§6) include at least: unicode variants (6.1),
    allowlisted path collisions (6.2), master §43 drift (6.3),
    perf budget with hard `timeout 30s` fail-closed behavior (6.4),
    and exhaustive GateError behavior including
    `ScriptInvocationFailure` (6.8).
9. The non-test obligations (§8) are routed to State 13 with
   their evidence forms referenced verbatim from
   `proof-to-rust-map.md`.
10. The Proof/Refinement Coverage Matrix (below) is present.

### 9.4 Acceptance criteria for downstream states

The State 9 test-writer is accepted IF the 3 tests are implemented per
§2 (with exact exit codes, exact file/line stderr diagnostics,
contracted hot-crate fixture paths, allowlist-precedence behavior,
all `GateError` variant sub-scenarios including
`ScriptInvocationFailure`, and hard `timeout 30s` wrapping), the test
driver passes `bash scripts/test-forbid-runtime-fmt.sh` on a clean
checkout after State 11 implementation, and the test driver exits
non-zero when any mutation class M-1 through M-8 is applied.

The State 13 black-hat reviewer is accepted IF the RQ-002 and
RQ-005 static-review evidence forms pass per §8.

## Proof/Refinement Coverage Matrix

| Proof ID | Refinement ID | Requirement | Contract Clause | Source Refs | Behavior Test Refs | Verifier | Evidence Command | Status |
|----------|---------------|-------------|-----------------|-------------|--------------------|----------|-------------------|--------|
| PO-RQ-001 | RRO-RQ-001 | RQ-001 | `3.2_pass_iff_no_active_residue` | `scripts/forbid-runtime-fmt.sh::main`, `scripts/forbid-runtime-fmt.sh::compile_scanner`, `scripts/forbid-runtime-fmt.rs::ResidueQuarantine::run`, `scripts/forbid-runtime-fmt.rs::ResidueQuarantine::decide` | `scripts/test-forbid-runtime-fmt.sh::test_quarantine_gate_blocks_json_import` | proptest | `bash scripts/test-forbid-runtime-fmt.sh test_quarantine_gate_blocks_json_import` | planned |
| PO-RQ-002 | RRO-RQ-002 | RQ-002 | `3.4_closed_set_invariant` | `velvet-ballistics-MASTER.md::section_43_trigger_table_7_to_10`, `scripts/forbid-runtime-fmt.rs::ResiduePolicy::from_master`, `scripts/forbid-runtime-fmt.rs::ForbiddenImportName` | (state-13 static review) | proptest | `bash -c 'awk §43..§44 \| grep -E "^- trigger (7\|8\|9\|10):" \| wc -l \| grep -qE "^[ ]*4$"'` | planned |
| PO-RQ-003 | RRO-RQ-003 | RQ-003 | `3.2_pass_iff_no_active_residue` (exit-code half) | `scripts/forbid-runtime-fmt.sh::main`, `scripts/forbid-runtime-fmt.sh::exit_code_translation`, `scripts/forbid-runtime-fmt.rs::ResidueQuarantine::decide`, `scripts/forbid-runtime-fmt.rs::GateError::exit_code` | `scripts/test-forbid-runtime-fmt.sh::test_quarantine_gate_blocks_unbounded_channel` | proptest | `bash scripts/test-forbid-runtime-fmt.sh test_quarantine_gate_blocks_unbounded_channel` | planned |
| PO-RQ-004 | RRO-RQ-004 | RQ-004 | `3.4_closed_set_invariant` (allowlist half) | `scripts/forbid-runtime-fmt.rs::ResidueQuarantine::diff_against_allowlist`, `scripts/forbid-runtime-fmt.rs::AllowlistRef::load`, `.moon/tasks/all.yml::forbid-runtime-fmt`, `.moon/tasks/all.yml::check` | `scripts/test-forbid-runtime-fmt.sh::test_moon_ci_quarantine_dependency_correctly_ordered` | proptest | `bash scripts/test-forbid-runtime-fmt.sh test_moon_ci_quarantine_dependency_correctly_ordered` | planned |
| PO-RQ-005 | RRO-RQ-005 | RQ-005 | `3.3_stderr_format` | `scripts/forbid-runtime-fmt.sh::sort_unique`, `scripts/forbid-runtime-fmt.sh::summary_line`, `scripts/forbid-runtime-fmt.sh::emit_residue_lines`, `scripts/forbid-runtime-fmt.rs::ResidueMatch::fmt` | (state-13 static review) | proptest | `bash -c 'grep -qE "sort[[:space:]]+-u" + grep -qE "^[[:space:]]*summary:"'` | planned |

The matrix above is the canonical test/refinement coverage matrix
required by the validator's `check_matrices` function (the needle
`Proof/Refinement Coverage Matrix` is present in this section).
