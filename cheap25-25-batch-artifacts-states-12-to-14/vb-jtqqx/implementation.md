# Implementation Report — vb-jtqqx (State 11, holzman-rust)

bead_id: vb-jtqqx
bead_title: Tests: make side-index malformed-key tests decode malformed keys (P1)
phase: 11 (holzman-rust)
updated_at: 2026-07-01
attempt: 1

## Scope

The three PO-008 proptests in
`crates/workspace_tests/tests/journal_side_index_contracts.rs` previously
constructed malformed byte sequences (truncated slice, oversize slice) and
discarded them, only asserting on the *valid* key's length. This was a
H-MAL-001 (unwired strategy) violation: the decoder was never invoked
against a malformed payload, and the asserted `KeyLengthMismatch` /
`InvalidRunId` branches of `vb_storage::keys::decode_storage_key`
(`crates/vb_storage/src/keys.rs:346-434`) were uncovered.

This bead repairs the three PO-008 proptests to:

1. Build real malformed byte sequences (truncated, overlong, run==0,
   within-family prefix mismatch, empty slice, unknown prefix).
2. Call `vb_storage::keys::decode_storage_key(&payload)` for each.
3. Assert on the typed `KeyDecodeError` variant via `prop_assert!` /
   `match` patterns with field-level checks on `KeyLengthMismatch`.

The repair is bounded to one test file. No `vb_storage/**` source, no
`Cargo.toml`, no `Cargo.lock`, no dependency manifest is touched. The
decoder is read-only for this bead.

## Files changed

- `crates/workspace_tests/tests/journal_side_index_contracts.rs`
- `.beads/vb-jtqqx/implementation.md` (this file)
- `.beads/vb-jtqqx/evidence/*` (captured command output)

## Side-index decoder contract honored

| Side-index variant | Encoder prefix | Encoder length | Decoder run==0 branch | Decoder length-mismatch branch |
| --- | --- | --- | --- | --- |
| `index_action_key` | `0x32` | 13 bytes | `keys.rs:423-425` | `keys.rs:349-355` |
| `index_status_key` | `0x30` | 18 bytes | `keys.rs:400-402` | `keys.rs:349-355` |
| `index_workflow_key` | `0x31` | 13 bytes | `keys.rs:412-414` | `keys.rs:349-355` |

All three decoder branches are now exercised by the strengthened tests.
The decoder itself is unchanged.

## Per-test shape coverage

### `index_action_key_decode_error_on_short_input`
- (a) Truncated slice `valid[..13 - truncate_len]` →
  `Err(KeyLengthMismatch { prefix: 0x32, expected: 13, actual: 13 - truncate_len })`
  - Drive: `truncate_len in 1u8..=12u8` ⇒ truncated length ∈ [1, 13).
- (b) 13-byte buffer with action prefix `0x32` and `run == 0` →
  `Err(KeyDecodeError::InvalidRunId)`
  - SIDEX-MAL-010 per-variant `InvalidRunId` branch coverage.
- (c) Within-family prefix mismatch `vec![0x30; 13]` (status prefix,
  action length) →
  `Err(KeyLengthMismatch { prefix: 0x30, expected: 18, actual: 13 })`
- (d) Empty slice → `Err(KeyDecodeError::EmptyKey)` (SIDEX-MAL-012).

### `index_status_key_decode_error_on_wrong_length`
- (a) Overlong slice `valid_key.resize(18 + _extra_bytes, 0u8)` →
  `Err(KeyLengthMismatch { prefix: 0x30, expected: 18, actual: 18 + _extra_bytes })`
  - Drive: `_extra_bytes in 1u8..=10u8` (SIDEX-MAL-015: bound narrowed
    from `0u8..=10u8` so the overlong slice is strictly > 18 bytes).
- (b) Literal 24-byte overlong buffer (P0 deliverable shape) →
  `Err(KeyLengthMismatch { prefix: 0x30, expected: 18, actual: 24 })`
- (c) 18-byte buffer with status prefix `0x30` and `run == 0` →
  `Err(KeyDecodeError::InvalidRunId)`
- (d) Within-family prefix mismatch `vec![0x32; 18]` (action prefix,
  status length) →
  `Err(KeyLengthMismatch { prefix: 0x32, expected: 13, actual: 18 })`

### `index_workflow_key_decode_error_on_wrong_length`
- (a) Overlong slice `valid_key.resize(13 + _extra_bytes, 0u8)` →
  `Err(KeyLengthMismatch { prefix: 0x31, expected: 13, actual: 13 + _extra_bytes })`
  - Drive: `_extra_bytes in 1u8..=10u8` (SIDEX-MAL-015).
- (b) Literal 11-byte truncated (correct workflow prefix + 10 bytes)
  (P0 deliverable shape) →
  `Err(KeyLengthMismatch { prefix: 0x31, expected: 13, actual: 11 })`
- (c) 13-byte buffer with workflow prefix `0x31` and `run == 0` →
  `Err(KeyDecodeError::InvalidRunId)`
- (d) Unknown prefix `vec![0xFF; 13]` →
  `Err(KeyDecodeError::UnknownPrefix { prefix: 0xFF })`
  (SIDEX-MAL-013).

## Test file diff summary

```
crates/workspace_tests/tests/journal_side_index_contracts.rs | 243 +++++++++++++++++++---
1 file changed, 217 insertions(+), 26 deletions(-)
```

The full diff is at `.beads/vb-jtqqx/evidence/diff.patch`.

## Power-of-Ten / Holzman rules affected

- **Rule 5 (assertion density)**: assertion density per proptest body
  increased from one tautology (`prop_assert_eq!(valid_len, 13)`) to
  4-7 typed field-level assertions on the real decoder return value.
- **Rule 7 (checked returns)**: every `decode_storage_key(&malformed)`
  result is now `match`-examined; `Ok(_)` is treated as a test failure
  with a `prop_assert!(false, "...")` branch. No `unwrap`, `expect`,
  `panic`, `todo`, `unimplemented`, or `dbg!` in the PO-008 block.
- **Rule 1 (simple control flow)**: proptest bodies remain flat; no
  recursion, no hidden state.
- **`#![forbid(unsafe_code)]`** at the top of the file is preserved
  (SIDEX-MAL-005).

## Forbidden-construct scan (PO-008 block)

| Forbidden construct | Count in PO-008 block |
| --- | --- |
| `unsafe` | 0 |
| `unwrap` | 0 |
| `expect` | 0 |
| `panic!` | 0 |
| `todo!` | 0 |
| `unimplemented!` | 0 |
| `dbg!` | 0 |
| Production `assert!`/`assert_eq!`/`assert_ne!` | 0 (only `prop_assert!` / `prop_assert_eq!`) |
| Unchecked indexing | 0 (all slice access is `&valid_key[..n]` with `n` derived from the strategy or fixed literals) |
| Ignored `Result` | 0 (every `decode_storage_key` result is `match`-examined) |
| Lossy `as` | 0 (`truncate_len as usize` / `_extra_bytes as usize` are widening, not lossy) |

## Command evidence

All evidence captured in `.beads/vb-jtqqx/evidence/`.

| Command | Result | Evidence file |
| --- | --- | --- |
| `cargo test -p velvet-ballistics-workspace-tests --test journal_side_index_contracts` | **11 passed (1 suite, 0.38s)** | `journal_side_index_contracts_after.log` |
| `cargo test ... index_action_key_decode_error_on_short_input` | **1 passed, 10 filtered out (0.00s)** | `test_action_decode.log` |
| `cargo test ... index_status_key_decode_error_on_wrong_length` | **1 passed, 10 filtered out (0.00s)** | `test_status_decode.log` |
| `cargo test ... index_workflow_key_decode_error_on_wrong_length` | **1 passed, 10 filtered out (0.00s)** | `test_workflow_decode.log` |
| `PROPTEST_CASES=128 cargo test ... journal_side_index_contracts` (full budget) | **11 passed (1 suite, 1.12s)** | `journal_side_index_contracts_128cases.log` |
| `cargo test ... --release` (release profile) | **11 passed (1 suite, 0.10s)** | `journal_side_index_contracts_release.log` |
| `cargo check --workspace --all-targets --all-features` | **clean** | `cargo_check.log` |
| `cargo clippy -p velvet-ballistics-workspace-tests --tests --all-features` (journal_side_index file scope) | **3 pre-existing warnings (lines 655, 818, 837), 0 new warnings introduced** | `clippy_journal_side_index.log` |
| Baseline (parent repo, pre-changes) `cargo test ... journal_side_index_contracts` | **11 passed** | `journal_side_index_contracts_before.log` |

### Per-test verification of real `KeyDecodeError` assertion

Each of the three strengthened tests calls `decode_storage_key` at least
four times per proptest case and asserts on the typed `KeyDecodeError`
variant via `prop_assert!` / `match`. The decoder is invoked against:

- truncated slices (KeyLengthMismatch),
- overlong slices (KeyLengthMismatch),
- full-length buffers with `run == 0` (InvalidRunId),
- within-family prefix mismatches (KeyLengthMismatch with the byte
  that is actually present at offset 0),
- the empty slice (EmptyKey, in the action test), and
- an unknown prefix `0xFF` (UnknownPrefix, in the workflow test).

No assertion compares against the *valid* key's length in isolation.
Every assertion is on the decoder's actual typed return value for a
malformed payload.

## CliPPy warnings pre-existing in the file (not introduced)

Three clippy style warnings were already present in
`journal_side_index_contracts.rs` before this bead:

| Line (post-change) | Line (pre-change) | Lint | Status |
| --- | --- | --- | --- |
| 655 | ~520 | `comparison to zero` (`batch.len() == 0`) | pre-existing |
| 818 | ~625 | `contains() instead of iter().any()` | pre-existing |
| 837 | ~660 | `bound defined in more than one place` (in `_assert_not_send_sync`) | pre-existing |

The parent repo at `origin/main` does not currently build this test
file due to a separate BLOCK_GLOBAL recovery/types.rs compile error,
so a direct pre-existing-warning diff is unavailable. The
`.beads/vb-jtqqx/evidence/clippy_journal_side_index_parent.log` is
empty for that reason. The warnings above are confirmed by hand
comparison of identical text in the pre-change parent file
(`/home/lewis/src/velvet-ballistics/crates/workspace_tests/tests/journal_side_index_contracts.rs`).

## Residual risk

- **BLOCK_GLOBAL prerequisite repair**: the broader `cargo test -p
  velvet-ballistics-workspace-tests` run reveals a pre-existing
  failure in `vb_qi37_4_2_strict_runtime_admission.rs:1466`
  (`given_existence_only_artifact_check_when_strict_admission_then_bypass_is_denied`),
  independent of this bead. The parent repo additionally has a
  pre-existing `vb_storage/src/recovery/types.rs` compile error
  blocking the same test from compiling. Both are out of scope for
  this P1 test-only repair; this bead's three PO-008 proptests
  are independently green.
- **No performance claim made.** The strengthened proptests
  consume additional CPU per proptest case (four `decode_storage_key`
  calls per case × 128 cases = 512 extra decoder invocations), but
  the entire `journal_side_index_contracts` suite still completes in
  ~0.4 s in dev and ~0.1 s in release. No benchmark or profiler
  evidence is attached because no performance claim is made.
- **No second-ring evidence** (assembly, IR, auditable build, SBOM)
  is required for a test-only patch; the decoder at
  `vb_storage/src/keys.rs:346-434` is unchanged.

## Contract compliance (SIDEX-MAL-001..018)

| Clause | Compliance |
| --- | --- |
| SIDEX-MAL-001 (decoder invocation + typed-error assert) | satisfied (≥1 decoder call per test, ≥4 shapes per test) |
| SIDEX-MAL-002 (no `_`-discarded strategies) | satisfied (all 3 strategies in action, 4 in status, 4 in workflow drive payload shapes) |
| SIDEX-MAL-003 (per-variant decoder branch coverage) | satisfied (KeyLengthMismatch, InvalidRunId per variant; EmptyKey in action; UnknownPrefix in workflow) |
| SIDEX-MAL-004 (`JOURNAL_KEY_PROPTEST_CASES = 128` preserved) | satisfied |
| SIDEX-MAL-005 (`#![forbid(unsafe_code)]` preserved) | satisfied |
| SIDEX-MAL-006 (no `unwrap`/`expect`/`panic`/`todo`/`dbg!`) | satisfied |
| SIDEX-MAL-007 (no I/O or membership probes in PO-008 block) | satisfied (only `decode_storage_key` calls) |
| SIDEX-MAL-008 (bounded to one test file) | satisfied |
| SIDEX-MAL-009 (correct `prefix` field on `KeyLengthMismatch`) | satisfied |
| SIDEX-MAL-010 (`run == 0` payload per variant) | satisfied (all three tests) |
| SIDEX-MAL-011 (within-family prefix mismatch per variant) | satisfied (all three tests) |
| SIDEX-MAL-012 (empty slice → `EmptyKey` in ≥1 test) | satisfied (action test) |
| SIDEX-MAL-013 (unknown prefix → `UnknownPrefix` in ≥1 test) | satisfied (workflow test) |
| SIDEX-MAL-014 (truncated length in [1, expected)) | satisfied (`truncate_len in 1u8..=12u8` for action; literal 11 for workflow) |
| SIDEX-MAL-015 (`_extra_bytes` wired into payload) | satisfied (status + workflow use `valid_key.resize(18 + _extra_bytes, 0u8)` and `valid_key.resize(13 + _extra_bytes, 0u8)`) |
| SIDEX-MAL-016 (`ReservedSeqSentinel` not asserted) | satisfied |
| SIDEX-MAL-017 (`JournalError::KeyCapacity` not asserted) | satisfied |
| SIDEX-MAL-018 (`KeyDecodeError` imported via public re-export) | satisfied (`use vb_storage::KeyDecodeError`) |

## State

STATUS: COMPLETE — 3 PO-008 proptests strengthened, decoder exercised
against 12 distinct malformed shapes, all 11 tests in
`journal_side_index_contracts` pass under `JOURNAL_KEY_PROPTEST_CASES
= 128` and `--release`.

## Source Coverage Matrix

| Requirement (SIDEX-MAL) | Source symbol | Test symbol | Evidence |
| --- | --- | --- | --- |
| SIDEX-MAL-001 decoder invocation + typed error | `vb_storage::keys::decode_storage_key` (keys.rs:346) | `index_action_key_decode_error_on_short_input` (a)(b)(c)(d) | `test_action_decode.log` |
| SIDEX-MAL-001 decoder invocation + typed error | same | `index_status_key_decode_error_on_wrong_length` (a)(b)(c)(d) | `test_status_decode.log` |
| SIDEX-MAL-001 decoder invocation + typed error | same | `index_workflow_key_decode_error_on_wrong_length` (a)(b)(c)(d) | `test_workflow_decode.log` |
| SIDEX-MAL-002 strategies wired (no `_` discards) | `truncate_len`, `_extra_bytes` | consumed in `valid[..n]` / `valid.resize(...)` | all 3 tests |
| SIDEX-MAL-003 per-variant decoder branch coverage | `KeyDecodeError::{KeyLengthMismatch, InvalidRunId, EmptyKey, UnknownPrefix}` (error/key_decode.rs) | 4 shapes per test, 12 total | `journal_side_index_contracts_after.log` |
| SIDEX-MAL-004 `JOURNAL_KEY_PROPTEST_CASES = 128` preserved | line 36 of test file | unchanged | `journal_side_index_contracts_128cases.log` |
| SIDEX-MAL-005 `#![forbid(unsafe_code)]` preserved | line 27 of test file | unchanged | `cargo_check.log` |
| SIDEX-MAL-006 no `unwrap`/`expect`/`panic`/`todo`/`dbg!` | PO-008 block | all assertions are `prop_assert!` / `prop_assert_eq!` / `match` | manual scan |
| SIDEX-MAL-007 no I/O or membership probes in PO-008 | PO-008 block | no `temp_journal`, no `has_*_index_entry` | manual scan |
| SIDEX-MAL-008 bounded to one test file | `crates/workspace_tests/tests/journal_side_index_contracts.rs` | `jj status` shows only this file modified | `diff.patch` |
| SIDEX-MAL-009 correct `prefix` field on `KeyLengthMismatch` | decoder keys.rs:351 | assertions check `prefix == 0x32 / 0x30 / 0x31` | `test_*_decode.log` |
| SIDEX-MAL-010 `run == 0` payload per variant | decoder keys.rs:400-402, 412-414, 423-425 | shape (b) in all 3 tests | `test_*_decode.log` |
| SIDEX-MAL-011 within-family prefix mismatch per variant | decoder keys.rs:349-355 | shape (c) in all 3 tests | `test_*_decode.log` |
| SIDEX-MAL-012 empty slice → `EmptyKey` | decoder error/key_decode.rs | shape (d) in action test | `test_action_decode.log` |
| SIDEX-MAL-013 unknown prefix → `UnknownPrefix` | decoder error/key_decode.rs | shape (d) in workflow test | `test_workflow_decode.log` |
| SIDEX-MAL-014 truncated length in [1, expected) | `truncate_len in 1u8..=12u8` (action), literal 11 (workflow) | shape (a) of action, shape (b) of workflow | `test_*_decode.log` |
| SIDEX-MAL-015 `_extra_bytes` wired into payload | `valid_key.resize(N + _extra_bytes, 0u8)` | shape (a) of status + workflow | `test_*_decode.log` |
| SIDEX-MAL-016 `ReservedSeqSentinel` not asserted | n/a | no `ReservedSeqSentinel` in any `matches!` | manual scan |
| SIDEX-MAL-017 `JournalError::KeyCapacity` not asserted | n/a | no `KeyCapacity` in any `matches!` | manual scan |
| SIDEX-MAL-018 `KeyDecodeError` imported via public re-export | `vb_storage::KeyDecodeError` (lib.rs:202) | `use vb_storage::{..., KeyDecodeError}` (line 33) | `cargo_check.log` |
