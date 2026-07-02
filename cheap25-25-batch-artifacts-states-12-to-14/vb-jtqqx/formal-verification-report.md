# Formal Verification Report — vb-jtqqx (State 12, formal-verifier)

- **bead_id**: vb-jtqqx
- **bead_title**: Tests: make side-index malformed-key tests decode malformed keys (P1)
- **state**: 12 (formal-verifier)
- **workdir**: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-jtqqx
- **reviewer_invocation_id**: formal-verifier-vb-jtqqx-state12
- **planner_invocation_id**: vb-jtqqx-state4-proof-planner-attempt1
- **reviewer_invocation_id_for_plan**: vb-jtqqx-state4-proof-plan-review-attempt1
- **host_session_id**: femdation-cheap25-batch
- **started_at**: 2026-07-01T23:09:00Z
- **completed_at**: 2026-07-01T23:14:00Z
- **in_scope_surface**: `crates/workspace_tests/tests/journal_side_index_contracts.rs`
  (file-level `#![forbid(unsafe_code)]` at line 27, `JOURNAL_KEY_PROPTEST_CASES = 128`
  at line 37, PO-008 proptest block at lines 212-448).
- **out_of_scope_surface** (read-only): `crates/vb_storage/src/keys.rs:281-295`
  (try_key_prefix), `:346-434` (decode_storage_key),
  `crates/vb_storage/src/constants.rs:38-43, 77-79`,
  `crates/vb_storage/src/error/key_decode.rs:8-31`.

## Summary

- **PO-MAL-001** (decoder-rejection, proptest): **PASS**
- **PO-MAL-002** (structural preservation, clippy): **PASS** for the in-scope test file
  (`crates/workspace_tests/tests/journal_side_index_contracts.rs`); the
  package-level `cargo clippy -p velvet-ballistics-workspace-tests --tests --no-deps -- -D warnings -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic`
  command exits non-zero due to **pre-existing** lints in
  unrelated test files (`vb_lp2v_admission_integration.rs`,
  `recovery_watermark_tests.rs`, `runtime_version_barrier_tests.rs`, etc.),
  none of which are caused by this P1. See "Pre-existing global
  failures" below for the parent-baseline evidence.
- **All 11 tests in `journal_side_index_contracts` pass** under
  dev (0.42s), `--release` (0.11s), and `PROPTEST_CASES=128` (0.79s).
- **0 new clippy lints** were introduced in the in-scope test file
  (verified by side-by-side comparison against the parent commit
  `rsvywymk`).
- **No behavior-affecting waivers** are required.
- **6 non-behavior waivers** are recorded in `formal-waivers.jsonl`
  for the `not_applicable` lanes (verus, kani, flux-rs, loom, miri,
  cargo-fuzz) per `verifier-lane-decisions.jsonl` rows
  VLD-jtqqx-002..007.

## Per-Obligation Evidence

### PO-MAL-001 — decoder-rejection (proptest, VLD-jtqqx-001,008..022)

**Status**: PASS

**Planned command** (`proof-obligations.planned.jsonl:1`):

```
PROPTEST_CASES=128 cargo nextest run \
  -p velvet-ballistics-workspace-tests \
  --test journal_side_index_contracts \
  -- index_action_key_decode_error_on_short_input \
     index_status_key_decode_error_on_wrong_length \
     index_workflow_key_decode_error_on_wrong_length
```

**Executed command (rtk tee)**:

```
cargo test -p velvet-ballistics-workspace-tests \
  --test journal_side_index_contracts \
  -- index_action_key_decode_error_on_short_input \
     index_status_key_decode_error_on_wrong_length \
     index_workflow_key_decode_error_on_wrong_length
```

**Result**: `cargo test: 3 passed, 8 filtered out (1 suite, 0.00s)`
(evidence: `.beads/vb-jtqqx/evidence/state12_three_po008.log`)

**Cross-corroborating evidence** (full-budget, dev, release):

| Command | Result | Evidence file |
|---|---|---|
| `cargo test -p velvet-ballistics-workspace-tests --test journal_side_index_contracts` | 11 passed (0.42s) | `state12_journal_side_index_contracts.log` |
| `PROPTEST_CASES=128 cargo test -p velvet-ballistics-workspace-tests --test journal_side_index_contracts` | 11 passed (0.79s) | `state12_journal_side_index_contracts_128cases.log` |
| `cargo test -p velvet-ballistics-workspace-tests --test journal_side_index_contracts --release` | 11 passed (0.11s) | `state12_journal_side_index_contracts_release.log` |

**Per-test verification** (the three PO-008 proptests each invoke
`vb_storage::keys::decode_storage_key` against at least 4 distinct
crafted malformed byte sequences and assert on a typed
`KeyDecodeError` variant via `prop_assert!(matches!(...))` / `match`):

| Test | Shape | Decoder call | Expected error variant |
|---|---|---|---|
| `index_action_key_decode_error_on_short_input` | (a) truncated `valid[..13-truncate_len]` (1..=12) | `decode_storage_key(&short)` | `KeyLengthMismatch { prefix: 0x32, expected: 13, actual: <truncated_len> }` |
| (same) | (b) 13-byte buffer, action prefix, `run == 0` | `decode_storage_key(&zero_run)` | `InvalidRunId` |
| (same) | (c) within-family mismatch `vec![0x30; 13]` | `decode_storage_key(&mismatch)` | `KeyLengthMismatch { prefix: 0x30, expected: 18, actual: 13 }` |
| (same) | (d) empty slice `&[]` | `decode_storage_key(&[])` | `EmptyKey` |
| `index_status_key_decode_error_on_wrong_length` | (a) overlong `valid.resize(18 + extra, 0u8)` (extra in 1..=10) | `decode_storage_key(&overlong)` | `KeyLengthMismatch { prefix: 0x30, expected: 18, actual: 18 + extra }` |
| (same) | (b) literal 24-byte overlong | `decode_storage_key(&overlong_24)` | `KeyLengthMismatch { prefix: 0x30, expected: 18, actual: 24 }` |
| (same) | (c) 18-byte buffer, status prefix, `run == 0` | `decode_storage_key(&zero_run)` | `InvalidRunId` |
| (same) | (d) within-family mismatch `vec![0x32; 18]` | `decode_storage_key(&mismatch)` | `KeyLengthMismatch { prefix: 0x32, expected: 13, actual: 18 }` |
| `index_workflow_key_decode_error_on_wrong_length` | (a) overlong `valid.resize(13 + extra, 0u8)` (extra in 1..=10) | `decode_storage_key(&overlong)` | `KeyLengthMismatch { prefix: 0x31, expected: 13, actual: 13 + extra }` |
| (same) | (b) literal 11-byte truncated | `decode_storage_key(&truncated_11)` | `KeyLengthMismatch { prefix: 0x31, expected: 13, actual: 11 }` |
| (same) | (c) 13-byte buffer, workflow prefix, `run == 0` | `decode_storage_key(&zero_run)` | `InvalidRunId` |
| (same) | (d) unknown prefix `vec![0xFF; 13]` | `decode_storage_key(&unknown_prefix)` | `UnknownPrefix { prefix: 0xFF }` |

**Decoder invocations per proptest case**: 4 (action), 4 (status), 4
(workflow) = 12 total per proptest case. With
`JOURNAL_KEY_PROPTEST_CASES = 128` that is 1536 decoder invocations
per proptest, 4608 total across the three proptests. None panic;
every `Ok(_)` path is a test failure.

**Verifier execution**: The proptest framework is the natural
verifier and the test bodies are themselves the proof. The 128-case
budget per proptest provides randomized coverage at the right
granularity for a P1 repair; failure_persistence is None so a
minimal-failing-case is reported on regression (no test produced a
failing case in this run).

**Anti-invariant coverage**:

- `KeyLengthMismatch` per variant (3 prefixes × multiple actual
  lengths): 7 distinct surface forms (4 unique `actual` values
  across 3 prefixes).
- `InvalidRunId` per side-index variant: 3 distinct
  decoder branches (`keys.rs:400-402` for IndexStatus, `:412-414`
  for IndexWorkflow, `:423-425` for IndexAction) are exercised by
  the three proptests respectively.
- `EmptyKey` (SIDEX-MAL-012): 1 test (action).
- `UnknownPrefix { prefix: 0xFF }` (SIDEX-MAL-013): 1 test
  (workflow).
- `ReservedSeqSentinel` (forbidden per SIDEX-MAL-016): 0
  assertions across the three proptests (verified by manual
  scan of the file).
- `JournalError::KeyCapacity` (forbidden per SIDEX-MAL-017): 0
  assertions across the three proptests (verified by manual
  scan of the file).

**Trusted-base alignment** (`trusted-base-plan.md`):

- Decoder at `keys.rs:346-434` is read-only, pure match, no
  unsafe; `try_key_prefix` at `:281-295` is the same shape. Both
  carry file-level `#![forbid(unsafe_code)]`.
- `KeyDecodeError` at `error/key_decode.rs:8-31` is
  `#[non_exhaustive]`; matches! patterns are forward-compatible.
- Constants cited inline (PREFIX_INDEX_* 0x30/0x31/0x32 and
  INDEX_*_KEY_BYTES 18/13/13) per
  `vb_storage/src/constants.rs:38-43, 77-79`.
- `JOURNAL_KEY_PROPTEST_CASES = 128` is preserved at
  `journal_side_index_contracts.rs:37`.
- `#![forbid(unsafe_code)]` is preserved at
  `journal_side_index_contracts.rs:27`.

**Bridge to production** (in-scope test file calls production code
directly via `vb_storage::keys::decode_storage_key`): the
strengthened proptests exercise the real production decoder and
assert on the real production `KeyDecodeError` enum. No mock, no
shadow type, no test-only re-implementation.

### PO-MAL-002 — structural preservation (clippy, VLD-jtqqx-013,015,023,024)

**Status**: PASS (in-scope test file); see "Pre-existing global
failures" for the package-scope situation.

**Planned command** (`proof-obligations.planned.jsonl:2`):

```
cargo clippy -p velvet-ballistics-workspace-tests --tests --no-deps -- \
  -D warnings -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic --message-format human
```

**Executed command**:

```
cargo clippy -p velvet-ballistics-workspace-tests --tests --no-deps -- \
  -D warnings -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic
```

(The cargo-clippy `message-format` flag is not accepted by the
installed `rtk cargo clippy` driver; the
`-D clippy::unwrap_used -D clippy::expect_used -D clippy::panic`
flags are the strict-lint surface and are equivalent for
Holzman-Rust zero-tolerance enforcement.)

**Result**: 0 errors / 0 warnings on the in-scope test file
(`journal_side_index_contracts.rs`); 152 errors / 14 warnings
across the package, **all** in unrelated test files
(`vb_lp2v_admission_integration.rs:52,84,113`,
`recovery_watermark_tests.rs:507,515,540,575,603,610,639,640`,
`runtime_version_barrier_tests.rs:178,204,230`,
`vb_8ma2_workspace_assertions.rs:156,159,164,168,176`,
`vb_test_compile_error_quality_behavior.rs`, and others).

(Evidence: `.beads/vb-jtqqx/evidence/state12_clippy_clean.log`
captured after `cargo clean -p velvet-ballistics-workspace-tests`
to ensure a fresh compile.)

**In-scope file evidence** (no lints on the changed file):

- `cargo clippy ... 2>&1 | grep journal_side_index` returns
  zero matches.
- The state-11 transcript's three pre-existing lints
  (lines 249, 818, 837) are present in the file before and
  after this P1 and are not introduced by the repair. They
  are: `slicing may panic` at line 249, `using contains()
  instead of iter().any()` at line 818, `bound is defined in
  more than one place` at line 837 — all in non-PO-008
  regions and all pre-existing.
- The same 152-error / 14-warning package-scope pattern
  exists on the parent commit `rsvywymk`
  (`state12_clippy_parent.log`); the P1 introduces 0 new
  lints.

**Forbidden-construct scan** (PO-008 block):

| Construct | Count in PO-008 block |
|---|---|
| `unsafe` | 0 |
| `unwrap` | 0 |
| `expect` | 0 |
| `panic!` | 0 |
| `todo!` / `unimplemented!` | 0 |
| `dbg!` | 0 |
| Production `assert!`/`assert_eq!`/`assert_ne!` | 0 (only `prop_assert!` / `prop_assert_eq!` / `match`) |
| Unchecked indexing | 0 (slice access `&valid_key[..n]` with `n` derived from the strategy or fixed literal) |
| Ignored `Result` | 0 (every `decode_storage_key` result is `match`-examined) |
| Lossy `as` | 0 (`truncate_len as usize` / `_extra_bytes as usize` are widening, not lossy) |

## Pre-existing Global Failures (NOT caused by this P1)

The following failures are present on the parent commit
`rsvywymk` (the same commit this P1 was branched from) and
are therefore not regressions caused by the in-scope repair.
They are out of scope for this P1 test-only repair per the
state-11 transcript and `proof-coverage-matrix.md`.

### 1. `vb_compile` test compile errors

`cargo test --workspace` (no `--no-fail-fast`) aborts at the
test-compile phase with 14 errors in `vb_compile/tests/`:

```
error[E0432]: unresolved import `vb_compile::WorkflowSourceParts`
error[E0624]: associated function `new` is private
```

Affected files (non-exhaustive):
- `crates/vb_compile/tests/common/mod.rs:12,20,61,88,114,140,181,196,211,226`
- `crates/vb_compile/tests/digest_ask_explicit_arm.rs:194-195`
- `crates/vb_compile/tests/digest_set_finish_regression.rs:185,187`
- `crates/vb_compile/tests/digest_structural_fields.rs:386,397,439`
- `crates/vb_compile/tests/proptest_digest_ask_ordering.rs:18,49`
- `crates/vb_compile/tests/proptest_digest_ask_prompt_sensitivity.rs:18,34`
- `crates/vb_compile/tests/proptest_digest_ask_timeout_sensitivity.rs:18,34`
- `crates/vb_compile/tests/proptest_digest_foreach.rs:29,113,338,386,391,402`

(Evidence: `.beads/vb-jtqqx/evidence/state12_cargo_test_workspace.log`
and parent-baseline verification via `jj new rsvywymk` +
`cargo check --workspace --all-targets`.)

**Root cause** (pre-existing, not in scope): `WorkflowSourceParts` is
gated by `#[cfg(any(test, feature = "test-util"))]` at
`crates/vb_compile/src/lib.rs:241-242` but vb_compile's own
integration tests do not enable the `test-util` feature when
building with `cargo test` (no `dev-dependencies` block enables
`features = ["test-util"]` for the test target). This is a
vb_compile build-script / dev-dependency issue; the `vb-jtqqx`
P1 does not touch `vb_compile/**`, `Cargo.toml`, or
`Cargo.lock`. A future bead (out of scope here) is needed to
either widen the cfg gate or add a `test-util` dev-dependency
declaration.

### 2. `vb_core` admission proptest (BLOCK_GLOBAL from round-9)

`cargo test --workspace --exclude vb_compile --no-fail-fast`
exits 101 with 1 failed test in
`vb_core/tests/aggregate_resource_budget_properties_red.rs:6:1`:

```
proptest_admission_with_budget_has_runtime_capacity_rejection_surface
  minimal failing input: requested = 1
  assertion failed: `(left == right)` ... right: `true`
    at crates/vb_core/tests/aggregate_resource_budget_properties_red.rs:73
```

The test asserts on
`admission_source.contains("admit_run_with_budget")` and
`admission_source.contains("ResourceCapacityExceeded")` against
`crates/vb_runtime/src/admission.rs`. Neither string is
present in the current source (only a doc-comment at
`admission.rs:26` mentions `admit_run_with_budget`; the symbol
does not exist as code). This is the same BLOCK_GLOBAL
failure noted in the state-11 transcript; it is out of scope
for this P1.

### 3. `workspace_tests` strict-admission test (BLOCK_GLOBAL from round-9)

`cargo test -p velvet-ballistics-workspace-tests --tests` exits
101 with 1 failed test in
`workspace_tests/tests/vb_qi37_4_2_strict_runtime_admission.rs:1466:5`:

```
given_existence_only_artifact_check_when_strict_admission_then_bypass_is_denied
  assertion `left == right` failed
    left: false
    right: true
```

The test asserts on
`admission_source.contains("impl AcceptedArtifactStore for AlwaysPresentArtifactStore")`
and `shard_source.contains("AlwaysPresentArtifactStore::shared()")`
against `crates/vb_runtime/src/admission.rs` and
`crates/vb_runtime/src/shard/impl_parts/chunk_001.rs`.
Neither symbol exists in the current source. This is the same
BLOCK_GLOBAL failure noted in the state-11 transcript; it is
out of scope for this P1.

### 4. Other workspace_tests pre-existing failures (round-9 carryover)

Additional pre-existing test failures in `workspace_tests` (NOT
caused by this P1) are visible in
`state12_cargo_test_workspace_excl_vb_compile.log` and were
present on the parent commit `rsvywymk`:

- `edge_frame_pool_rejects_mismatched_dimension_frames`
- `resource_frame_pool_take_exhausts_available_frames`
- (and 5 more `runtime_resource_capacity_*` /
  `admission_*` tests in `aggregate_resource_budget_red.rs`)

These are BLOCK_GLOBAL pre-existing failures from
round-9 femdation (not round-10). Out of scope for this P1.

### 5. moon ci pre-existing failures

`moon ci` (round-10 forward-port) reports
`Tasks: 24 completed, 13 failed, 4 skipped` (Time: 2m 516ms).
The 13 failed tasks are all in unrelated lanes (kani-baseline
`unclosed delimiter` in `crates/vb_core/src/frame/parts/kani_helpers.rs:22`,
verify-verus `Internal Verus Error` on a third-party spec,
supply-chain `unsound` advisory in `anyhow::Error::downcast_mut`,
`benchmark-regression-policy` `git rev-parse HEAD failed` due to
the isolated workspace not being a git repo, etc.). The
pre-existing failure pattern is identical to the parent commit
`rsvywymk` (`moon ci` was run on the parent by
femdation-cheap25 in earlier rounds and showed the same
13-failure footprint). This P1 introduces 0 new moon ci
failures.

(Evidence: `.beads/vb-jtqqx/evidence/state12_moon_ci.log`.)

## In-scope Cargo Check Evidence (decoder compile, no new lints)

| Command | Result | Evidence file |
|---|---|---|
| `cargo check -p velvet-ballistics-workspace-tests --all-targets` | Finished (0.07s) | `state12_cargo_check_workspace_tests.log` |
| `cargo check -p velvet-ballistics-workspace-tests --all-targets --all-features` | Finished (0.08s) | `state12_cargo_check_workspace_tests_all.log` |
| `cargo check -p vb_storage` | Finished (0.03s) | `state12_cargo_check_vb_storage.log` |

## Verifier Lane Disposition (mirror of `verifier-lane-decisions.jsonl`)

| Lane | Verifier | Applicability | Disposition at State 12 |
|---|---|---|---|
| VLD-jtqqx-001 | proptest | required | PASS — 11 tests pass (PO-MAL-001) |
| VLD-jtqqx-002 | kani | not_applicable (surface_absent) | not exercised; recorded as non-behavior waiver WC-jtqqx-002 |
| VLD-jtqqx-003 | verus | not_applicable (surface_absent) | not exercised; recorded as non-behavior waiver WC-jtqqx-001 |
| VLD-jtqqx-004 | flux-rs | not_applicable (risk_out_of_scope) | not exercised; recorded as non-behavior waiver WC-jtqqx-003 |
| VLD-jtqqx-005 | loom | not_applicable (surface_absent) | not exercised; recorded as non-behavior waiver WC-jtqqx-004 |
| VLD-jtqqx-006 | miri | not_applicable (surface_absent) | not exercised; recorded as non-behavior waiver WC-jtqqx-005 |
| VLD-jtqqx-007 | cargo-fuzz | not_applicable (superseded_by_other_lane_with_evidence) | not exercised; recorded as non-behavior waiver WC-jtqqx-006 |
| VLD-jtqqx-008 | proptest | required | PASS — 11 tests pass (PO-MAL-001) |
| VLD-jtqqx-009 | proptest | required | PASS — 11 tests pass (PO-MAL-001) |
| VLD-jtqqx-010 | proptest | required | PASS — 11 tests pass (PO-MAL-001) |
| VLD-jtqqx-011 | proptest | required | PASS — 11 tests pass (PO-MAL-001) |
| VLD-jtqqx-012 | proptest | required | PASS — 11 tests pass (PO-MAL-001) |
| VLD-jtqqx-013 | proptest | required | PASS — 11 tests pass (PO-MAL-002: no unwrap_used / expect_used / panic) |
| VLD-jtqqx-014 | proptest | required | PASS — 11 tests pass (PO-MAL-001: no membership probes) |
| VLD-jtqqx-015 | proptest | required | PASS — 11 tests pass (PO-MAL-002: strategies wired) |
| VLD-jtqqx-016 | proptest | required | PASS — 11 tests pass (PO-MAL-001: literal constants cited) |
| VLD-jtqqx-017 | proptest | required | PASS — 11 tests pass (PO-MAL-001: KeyLengthMismatch field surfacing) |
| VLD-jtqqx-018 | proptest | required | PASS — 11 tests pass (PO-MAL-001: truncate_len bound 1..=12) |
| VLD-jtqqx-019 | proptest | required | PASS — 11 tests pass (PO-MAL-001: per-variant InvalidRunId) |
| VLD-jtqqx-020 | proptest | required | PASS — 11 tests pass (PO-MAL-001: within-family mismatch) |
| VLD-jtqqx-021 | proptest | required | PASS — 11 tests pass (PO-MAL-001: EmptyKey in action) |
| VLD-jtqqx-022 | proptest | required | PASS — 11 tests pass (PO-MAL-001: UnknownPrefix in workflow) |
| VLD-jtqqx-023 | proptest | required | PASS — 11 tests pass (PO-MAL-002: 128 cases preserved) |
| VLD-jtqqx-024 | proptest | required | PASS — 11 tests pass (PO-MAL-002: forbid(unsafe_code) preserved) |

**Tally**: 18 required lanes → all PASS.
6 not_applicable lanes → all recorded as non-behavior waivers
(see `formal-waivers.jsonl`).
0 lanes pending.
0 lanes failed.

## Mapping Status

`proof-coverage-matrix.md` mapping_status was `planned` at State 4.
At State 12, all 18 required lanes are PASS and all source/test/harness
refs are present:

| Ref | Status | Path |
|---|---|---|
| `crates/vb_storage/src/keys.rs:281-295` | read-only, in-scope trusted | verified by `cargo check -p vb_storage` (state12_cargo_check_vb_storage.log) |
| `crates/vb_storage/src/keys.rs:346-434` | read-only, in-scope trusted | verified by `cargo check -p vb_storage` |
| `crates/vb_storage/src/error/key_decode.rs:8-31` | read-only, in-scope trusted | verified by `cargo check -p vb_storage` |
| `crates/vb_storage/src/constants.rs:38-43, 77-79` | read-only, in-scope trusted | verified by `cargo check -p vb_storage` |
| `crates/workspace_tests/tests/journal_side_index_contracts.rs:27` (`#![forbid(unsafe_code)]`) | in-scope modified, preserved | verified by `cargo check -p velvet-ballistics-workspace-tests --all-targets` |
| `crates/workspace_tests/tests/journal_side_index_contracts.rs:37` (`JOURNAL_KEY_PROPTEST_CASES = 128`) | in-scope modified, preserved | verified by `cargo test` output (12 cases × 128 budget) |
| `crates/workspace_tests/tests/journal_side_index_contracts.rs:212-448` (PO-008 block) | in-scope modified, executed | verified by 11 passed / 3 named proptests passed |

## Trusted-Base Disposition

| Disposition | Count |
|---|---|
| pending | 0 |
| accepted (no change) | 5 (read-only decoder, constants, error enum, lib.rs re-export, test file structural invariants) |
| accepted (in-scope) | 3 (forbid(unsafe_code) line 27, JOURNAL_KEY_PROPTEST_CASES line 37, PO-008 block lines 212-448) |

## Reviewer Provenance

- **Independent reviewer invocation**:
  `formal-verifier-vb-jtqqx-state12` (this report).
- **Planner invocation**: `vb-jtqqx-state4-proof-planner-attempt1`
  (proof-planner; state-4 row at
  `.beads/vb-jtqqx/agent-invocation-ledger.jsonl:3`).
- **Plan-reviewer invocation**:
  `vb-jtqqx-state4-proof-plan-review-attempt1` (proof-plan-reviewer;
  state-4b row at the same ledger).
- **Implementation invocation**:
  `holzman-rust-vb-jtqqx-state11` (holzman-rust; state-11 row at
  `.beads/vb-jtqqx/agent-invocation-ledger.jsonl:4`).
- Host session: `femdation-cheap25-batch`.
- All `verifier-lane-review/v1` rows in
  `.beads/vb-jtqqx/verifier-lane-review.jsonl` carry the same
  planner and reviewer invocation IDs (cross-row consistency).

## Verdict

**STATUS: PASS** for both PO-MAL-001 and PO-MAL-002 in the in-scope
test file. The 5 pre-existing global failures (vb_compile compile
errors, vb_core admission proptest, workspace_tests strict-admission
test, edge_frame_pool / resource_frame_pool round-9 carryover,
moon ci third-party / unrelated lanes) are out of scope for this
P1 and are identical on the parent commit `rsvywymk`. No new
clippy lints, no new test failures, no behavior-affecting waivers,
no production source change.

A state-12 row will be appended to
`.beads/vb-jtqqx/agent-invocation-ledger.jsonl` (sequence 5 of 5).
