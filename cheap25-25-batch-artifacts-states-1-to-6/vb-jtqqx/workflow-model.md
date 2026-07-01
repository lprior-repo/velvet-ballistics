# Workflow Model — vb-jtqqx

## Scope

This document models the proptest-body state machine that the three PO-008
tests must implement. There is no production workflow change; the
"workflow" here is the test-execution flow that turns a proptest strategy
sample into a malformed-payload assertion.

## High-Level Test Workflow

```text
  [proptest runner picks strategy sample]
        │
        ▼
  [decode valid keys for the relevant side-index variant(s)]
        │
        ▼
  [build a *set* of malformed payloads from the strategy inputs]
        │
        ▼
  [for each malformed payload: classify → build expected error]
        │
        ▼
  [call decode_storage_key → prop_assert! matches! against expected error]
        │
        ▼
  [proptest runner either shrinks failing case or accepts]
```

## Per-Test State Machine

The three PO-008 tests share a common shape but specialize on which
side-index variant they target. Each test's body is a single proptest
sample-handler; the machine below is what each sample triggers.

```text
  S0 (sample received)
     │  strategies: action_val | state | workflow_val | timestamp | run_val | step_val
     │              + truncate_len | _extra_bytes (per-test signatures differ)
     ▼
  S1 (build valid keys)
     │  - valid_action   = index_action_key(action, run, step)
     │  - valid_status   = index_status_key(state, timestamp, run)
     │  - valid_workflow = index_workflow_key(workflow, run)
     ▼
  S2 (build malformed payloads; one per assertion set)
     │  a) empty:        []
     │  b) unknown:      vec![0xFF; <L>]
     │  c) short:        valid[..max(1, valid.len()-truncate_len)]
     │  d) oversize:     valid.iter().chain(extra_bytes).copied().collect()
     │  e) wrong family: prefix bytes from a *different* side-index variant
     │                   combined with the wrong length envelope
     │  f) zero run:     valid_len buffer with run field zeroed
     ▼
  S3 (classify expected error per payload)
     │  a) → EmptyKey
     │  b) → UnknownPrefix { prefix: 0xFF }
     │  c) → KeyLengthMismatch { prefix: <actual>, expected: <expected>, actual: <short_len> }
     │  d) → KeyLengthMismatch { prefix: <side-index prefix>, expected: <13|18>, actual: <13|18 + extra> }
     │  e) → KeyLengthMismatch { prefix: <actual_wrong_family_prefix>, expected: <expected for that prefix>, actual: <wrong_len> }
     │  f) → InvalidRunId
     ▼
  S4 (call decoder and assert)
     │  for each (payload, expected) in S3:
     │      prop_assert!(matches!(decode_storage_key(payload), Err(expected)))
     ▼
  S5 (accept sample)
```

## Per-Test Specialization

### Test 1 — `index_action_key_decode_error_on_short_input`

- **Strategy inputs**: `action_val in 1u16..=100u16`,
  `run_val in 1u64..=1000u64`, `step_val in 0u16..=50u16`,
  `truncate_len in 1u8..=12u8`.
- **Valid key built**: `index_action_key(action, run, step)` — 13 bytes.
- **Required assertions** (each `prop_assert!`-wrapped `matches!`):
  - **(c) short**: `decode_storage_key(&valid[..13 - truncate_len as usize])`
    → `Err(KeyLengthMismatch { prefix: 0x32, expected: 13, actual: 13 - truncate_len })`.
  - **(a) empty**: `decode_storage_key(&[])`
    → `Err(KeyDecodeError::EmptyKey)`. (Required by Contract M-001;
    recommend placing here.)
  - **(f) zero run** for action: build a 13-byte buffer
    `[0x32, action_be, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, step_be_high, step_be_low]`
    → `Err(KeyDecodeError::InvalidRunId)`.
  - **(e) within-family mismatch** (status prefix, action length):
    `vec![0x30; 13]` → `Err(KeyLengthMismatch { prefix: 0x30, expected: 18, actual: 13 })`.
- **Optional but recommended**:
  - **(b) unknown prefix**: `vec![0xFF; 13]` → `Err(UnknownPrefix { prefix: 0xFF })`.
  - **(d) oversize**: `valid + extra bytes` (this test does not have
    `_extra_bytes`; the test writer may add it or simply append a small
    fixed pad).

### Test 2 — `index_status_key_decode_error_on_wrong_length`

- **Strategy inputs**: `state in 0u8..=2u8`,
  `timestamp in 0u64..=1000u64`, `run_val in 1u64..=1000u64`,
  `_extra_bytes in 0u8..=10u8`.
- **Valid key built**: `index_status_key(state, timestamp, run)` — 18 bytes.
- **Required assertions**:
  - **(d) oversize**: `valid + vec![0u8; _extra_bytes as usize]`
    → `Err(KeyLengthMismatch { prefix: 0x30, expected: 18, actual: 18 + _extra_bytes })`.
    **This is the contract that wires the `_extra_bytes` strategy.**
  - **(c) short**: `valid[..17]` → `Err(KeyLengthMismatch { prefix: 0x30, expected: 18, actual: 17 })`.
  - **(f) zero run** for status: build an 18-byte buffer
    `[0x30, state, timestamp_be, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]`
    → `Err(KeyDecodeError::InvalidRunId)`.
  - **(e) within-family mismatch** (action prefix, status length):
    `vec![0x32; 18]` → `Err(KeyLengthMismatch { prefix: 0x32, expected: 13, actual: 18 })`.
- **Optional but recommended**:
  - **(b) unknown prefix**: `vec![0xFF; 18]` → `Err(UnknownPrefix { prefix: 0xFF })`.

### Test 3 — `index_workflow_key_decode_error_on_wrong_length`

- **Strategy inputs**: `workflow_val in 1u32..=100u32`,
  `run_val in 1u64..=1000u64`, `_extra_bytes in 0u8..=10u8`.
- **Valid key built**: `index_workflow_key(workflow, run)` — 13 bytes.
- **Required assertions**:
  - **(d) oversize**: `valid + vec![0u8; _extra_bytes as usize]`
    → `Err(KeyLengthMismatch { prefix: 0x31, expected: 13, actual: 13 + _extra_bytes })`.
    **This is the contract that wires the `_extra_bytes` strategy.**
  - **(c) short**: `valid[..min(12, valid.len()-1)]` → `Err(KeyLengthMismatch { prefix: 0x31, expected: 13, actual: <short_len> })`.
  - **(f) zero run** for workflow: build a 13-byte buffer
    `[0x31, workflow_be, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]`
    → `Err(KeyDecodeError::InvalidRunId)`.
  - **(e) within-family mismatch** (status prefix, workflow length):
    `vec![0x30; 13]` → `Err(KeyLengthMismatch { prefix: 0x30, expected: 18, actual: 13 })`.
- **Optional but recommended**:
  - **(b) unknown prefix**: `vec![0xFF; 13]` → `Err(UnknownPrefix { prefix: 0xFF })`.

## Cross-Cutting Workflow Rules

1. **No mutation before assertion.** The decoder is pure; the test body
   must not write to any storage, journal, or shared state. The test
   file imports `tempfile` and `FjallJournal` for *other* PO tests, but
   the PO-008 block must not construct a `temp_journal()`.
2. **Strategy inputs are not decorative.** Every strategy variable on the
   proptest signature must drive at least one malformed-payload shape.
   Removing strategies is forbidden. Unbound strategies are a black-hat
   finding (see `H-MAL-006` in hazard-analysis).
3. **Per-sample assertion order.** The valid-key construction is the
   first thing the body does (any error there fails the sample fast).
   The malformed-payload construction is the second thing. The
   decoder-call + `prop_assert!` is the third and only purpose of the
   rest of the body. There is no fourth step.
4. **Failure semantics.** A failing `prop_assert!` is reported by proptest
   with the failing sample shrunk to the smallest reproducer. The
   shrinking path must not be confused with mutation; the failing
   assertion is the only output that matters for coverage.
5. **Coverage matrix.** Across the three tests, the following decoder
   branches must each be exercised at least once:
   - `try_key_prefix` empty branch.
   - `try_key_prefix` unknown-prefix branch.
   - `decode_storage_key` length-mismatch branch (multiple actual/expected
     combinations).
   - `IndexStatus` decoder → `InvalidRunId` branch.
   - `IndexWorkflow` decoder → `InvalidRunId` branch.
   - `IndexAction` decoder → `InvalidRunId` branch.

## Out-of-Workflow Steps (explicitly forbidden)

- Calling `FjallJournal::open` / `temp_journal()` inside the PO-008
  proptest block. The decoder is a pure function; routing through
  storage would lose the malformed-payload shape (the journal never
  decodes).
- Calling `vb_storage::keys::index_*_key(...)` and asserting only on
  its `Ok` result. The encoder's success is already covered by
  `crates/vb_storage/src/keys/tests.rs`.
- Asserting on `JournalError::KeyCapacity` directly. The PO-008 header
  comment (line 184 of the test file) is a stale misnomer — the decoder
  returns `KeyDecodeError`, not `JournalError::KeyCapacity`. The repair
  corrects this and asserts on `KeyDecodeError` instead. (If a future
  bead wants to add coverage for `JournalError::KeyCapacity`, that is a
  different code path and out of scope here.)
- Asserting on `KeyDecodeError::ReservedSeqSentinel`. This variant is
  unreachable from side-index payloads.
- Asserting on `IndexStatusStateCollision`. That variant belongs to the
  *encoder* and is already covered in
  `crates/vb_storage/src/batch/t_putters_b.rs:178-189` and
  `crates/vb_storage/src/type_tests.rs`.