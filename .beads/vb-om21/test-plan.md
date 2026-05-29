# Test Plan — vb-om21 State 8

schema_version: test-plan/v1
bead_id: vb-om21
state: 8
sublane: test-planning
isolated_workdir: /home/lewis/isolated/femdation-velvet-ballistics/vb-om21
planned_at_utc: 2026-05-27T22:00:00Z
planner_invocation_id: test-planner-vb-om21-state8-001
parent_invocation_id: proof-reviewer-vb-om21-state7-bridge-001
bead_classification: TEST-FIRST (production code not in scope until State 11)
target_test_file: crates/workspace_tests/tests/restate_journal_tail_scan_fallback_tests.rs

## Inputs

- `contract.md` — 8 requirement IDs, 6 contract clauses
- `delivery-scope.jsonl` — 17 delivery scope entries including keys.rs, replay.rs, error/mod.rs, workspace_tests
- `proof-to-rust-map.md` — 52 proof obligations bridged to 11 planned behavior test functions
- `proof-review.md` — State 6 APPROVED with 52 obligations verified
- `proof-to-rust-review.md` — State 7 bridge APPROVED

## Bead Requirements (From contract.md)

| ID | Requirement |
|---|---|
| REQ-vb-om21-01 | Missing tail metadata must reconstruct tail from the final durable `run_event` key for that run. |
| REQ-vb-om21-02 | Matching declared tail metadata and final durable key must recover without warning/error. |
| REQ-vb-om21-03 | Declared/suspect tail below reconstructed durable key tail must return typed `TailMismatch`. |
| REQ-vb-om21-04 | Recovery-required missing `run_event` prefix must return typed `MissingJournal`. |
| REQ-vb-om21-05 | Empty keyspace/prefix tail query returns zero tail. |
| REQ-vb-om21-06 | Single event key at seq 0 reconstructs tail 1. |
| REQ-vb-om21-07 | Tail scan must be bounded to `[0x11][run_id_u64_be]` and never cross another run prefix. |
| REQ-vb-om21-08 | Reconstructed tail equals `max(encoded_seq) + 1` using checked arithmetic. |

## Contract Clauses Coverage

| Clause | Tests |
|---|---|
| C-vb-om21-prefix-bound | test_tail_scan_prefix_bound, test_bounded_scan |
| C-vb-om21-big-endian-max | test_big_endian_max_seq, test_key_parse_no_panic |
| C-vb-om21-tail-definition | test_zero_tail_empty_journal, test_single_event_tail, test_tail_overflow |
| C-vb-om21-metadata-validation | test_tail_mismatch_rejection, test_typed_error_distinction |
| C-vb-om21-missing-journal | test_missing_journal_recovery |
| C-vb-om21-replay-integrity | test_replay_parity |

## Test Architecture

### Key Layout Knowledge

```
run_event_key  = [0x11][run_id: u64 BE][seq: u64 BE]  (17 bytes)
run_prefix_key = [0x11][run_id: u64 BE]                (9 bytes)
```

### Production Seams (from delivery-scope.jsonl)

| Seam | File | Use in Tests |
|---|---|---|
| `FjallJournal::open` | `journal/core.rs` | Create test database |
| `FjallJournal::append_journaled` | `journal/append.rs` | Write events to create test tails |
| `FjallJournal::inject_raw_event` | `journal/injection.rs` | Write suspect/malformed records for mismatch tests |
| `FjallJournal::get_event_bytes` | `journal/replay.rs` | Query events by (run, seq) key |
| `FjallJournal::events_for_run` | `journal/replay.rs` | Full replay for parity tests |
| `events_for_run_from` | `journal/replay.rs` | Prefix-bounded scan (core logic) |
| `run_event_key` | `keys.rs` | Encode keys for direct Fjall access |
| `run_prefix_key` | `keys.rs` | Encode prefix for scan boundary |

### Existing Test Patterns (from restate_fjall_keyspace_manifest_tests.rs)

Tests use:
- `proptest!` macro for property-based testing
- `#[test]` attribute for unit-style tests
- Direct `FjallJournal` construction (or database open in temp dir)
- `vb_storage::keys::*` imports for key constructors
- Assertions with `prop_assert_eq!`, `assert_eq!`, `assert!`
- No `unwrap`/`expect`/`panic!` (repository rule)

## Test Function Specifications

### 1. test_tail_scan_prefix_bound

**Requirement:** REQ-vb-om21-07 | **Clause:** C-vb-om21-prefix-bound
**Proof IDs:** PO-vb-om21-prefix-bound-*

**Given:**
- A FjallJournal with `run_event` keyspace opened
- Two runs, `RUN_A` and `RUN_B`, each with events at sequences 0..N
- Events written via `append_journaled` or `inject_raw_event`
- The scan is initiated for RUN_A

**When:**
- The tail scan scans keys from `run_prefix_key(RUN_A)` upward
- During the scan, RUN_B keys appear in the ordered range

**Then:**
- All keys with prefix matching RUN_A are observed
- The first key NOT matching `run_prefix_key(RUN_A)` terminates the scan
- No RUN_B events are decoded or counted
- The reconstructed tail matches only RUN_A's max sequence + 1

**Test Variants:**
1. RUN_A has 0 events, RUN_B has events → scan observes zero keys for RUN_A
2. RUN_A has 3 events (seq 0,1,2), RUN_B has 5 events (seq 0..4) → tail = 3 for RUN_A
3. RUN_A events come lexicographically after RUN_B events (lower RunId) → scan still bounded
4. RUN_A RunId > RUN_B RunId → RUN_A keys sort after RUN_B keys, scan terminates at first non-matching key
5. Single keyspace with 5 runs, scan target in middle → bounded correctly

**Mutation resistance:** Injected event with wrong run prefix must not be decoded. Malformed key that starts with run_prefix but has wrong length must be rejected, not cause scan termination.

### 2. test_big_endian_max_seq

**Requirement:** REQ-vb-om21-08 | **Clause:** C-vb-om21-big-endian-max
**Proof IDs:** PO-vb-om21-big-endian-max-*

**Given:**
- Keys generated with `sequenced_run_key` for varying sequence values
- Multiple events at different sequences for the same run

**When:**
- The tail scan extracts the sequence bytes from bytes 9..17 of each matching key
- It interprets them as `u64::from_be_bytes`
- It selects the maximum across all matching keys

**Then:**
- For key at seq=0, bytes 9..17 decode to 0u64
- For key at seq=255, bytes 9..17 decode to 255u64
- For key at seq=u64::MAX, bytes 9..17 decode to u64::MAX
- Lexicographic comparison of the last 8 bytes matches numeric comparison of the u64 values
- `max(seq_a, seq_b)` in numeric space equals `max(key_a[9..17], key_b[9..17])` in lexical byte space

**Test Variants:**
1. Single key at seq=0 → max = 0
2. Keys at seq=5, seq=42, seq=3 → max = 42 (not 5, not 3)
3. Keys at seq=u64::MAX-1, seq=u64::MAX → max = u64::MAX
4. Keys at seq=0, seq=1, seq=2 (ascending order) → max = 2
5. Key at seq=1 << 63 (midpoint of u64 range) → big-endian bytes sort correctly

### 3. test_tail_mismatch_rejection

**Requirement:** REQ-vb-om21-03 | **Clause:** C-vb-om21-metadata-validation
**Proof IDs:** PO-vb-om21-tail-mismatch-*

**Given:**
- A FjallJournal with events for RUN_X at sequences 0..5 (tail = 6)
- Declared metadata claiming tail = 4 (below actual)

**When:**
- The tail scan reconstructs actual_tail = 6 from max_key(seq=5) + 1
- It compares declared_meta_tail = 4 against reconstructed_tail = 6

**Then:**
- Returns `TailMismatch { run, declared: 4, actual: 6 }` (or equivalent typed variant)
- Does not proceed to replay or truncation
- Does not return Ok or a different error type

**Test Variants:**
1. declared=6, actual=6 → success, no mismatch
2. declared=10, actual=6 → declared above actual (no conflict? Or requires investigation)
3. declared=0, actual=1 → mismatch (single event case)
4. declared=0, actual=0 → both consistent (empty journal)
5. declared=u64::MAX, actual=5 → mismatch

### 4. test_missing_journal_recovery

**Requirement:** REQ-vb-om21-04 | **Clause:** C-vb-om21-missing-journal
**Proof IDs:** PO-vb-om21-missing-journal-*

**Given:**
- A FjallJournal opened but no `run_event` keys written for RUN_X
- Recovery mode = RecoveryRequiresJournal

**When:**
- The tail scan attempts to scan keys with run_prefix_key(RUN_X)
- The keyspace range from the prefix onward is empty OR has no matching prefix keys

**Then:**
- Returns `MissingJournal { run: RUN_X }` (or equivalent typed variant)
- Does not return Ok with empty event list
- Does not return TailMismatch (different semantic)
- Does not return a generic error that conflates with other failure modes

**Test Variants:**
1. Fresh database, no events at all → MissingJournal
2. Events exist for RUN_Y but not RUN_X → MissingJournal for RUN_X, OK for RUN_Y
3. Events exist for RUN_X in a different keyspace (e.g., run_header) → MissingJournal (only run_event counts)
4. QueryAllowsEmpty mode → returns tail=0 (not MissingJournal)

### 5. test_zero_tail_empty_journal

**Requirement:** REQ-vb-om21-05 | **Clause:** C-vb-om21-tail-definition
**Proof IDs:** PO-vb-om21-zero-tail-query-*

**Given:**
- A FjallJournal with no events for RUN_X
- The keyspace range for run_prefix_key(RUN_X) has zero matching keys

**When:**
- The tail scan iterates from run_prefix_key(RUN_X) and finds the first key does not start with the prefix (or the range is empty)

**Then:**
- Returns tail = EventSeq(0)
- Does not fabricate a zero-sequence event
- Does not return an error (empty is valid for tail query)
- The returned replay event list is empty (if events are also requested)

**Test Variants:**
1. Empty keyspace, QueryAllowsEmpty mode → tail=0, empty events
2. Events exist for RUN_Y only → tail=0 for RUN_X
3. Key exists for RUN_X in header keyspace but not run_event → tail=0
4. Prefix key decompression — `run_prefix_key(RUN_X)` produces 9 bytes, not 17

### 6. test_single_event_tail

**Requirement:** REQ-vb-om21-06 | **Clause:** C-vb-om21-tail-definition
**Proof IDs:** PO-vb-om21-single-event-tail-*

**Given:**
- A FjallJournal with exactly one event for RUN_X at sequence 0
- The key is `run_event_key(RUN_X, EventSeq(0))`

**When:**
- The tail scan iterates and finds exactly one matching key
- It decodes the sequence bytes to 0u64

**Then:**
- Reconstructed tail = EventSeq(1) (max_seq 0 + 1)
- The single event is available for replay if requested
- Tail is not 0 (that would lose the event)
- Tail is not 2 (off by one error)

**Test Variants:**
1. Single event at seq=0 → tail=1
2. Single event at seq=7 (non-zero start) → tail=8
3. Single event at seq=EventSeq(u64::MAX - 1) → tail=u64::MAX
4. Two events at seq=0 and seq=1 → tail=2 (not single event case, but validates correctness)

### 7. test_tail_overflow

**Requirement:** REQ-vb-om21-08 | **Clause:** C-vb-om21-tail-definition
**Proof IDs:** PO-vb-om21-tail-overflow-*

**Given:**
- A FjallJournal where the max encoded sequence in the prefix range is u64::MAX
- This could be an event at seq=u64::MAX, or a key with that encoded value

**When:**
- `checked_add(max_seq, 1)` is called

**Then:**
- Returns typed overflow error (e.g., `TailOverflow { max_seq: u64::MAX }` or equivalent)
- Does NOT wrap to 0 (unsigned overflow PANICS in debug, wraps in release — we want neither)
- Does NOT return Ok with a valid tail value
- Error is distinct from MissingJournal and TailMismatch

**Test Variants:**
1. max_seq = u64::MAX → overflow error
2. max_seq = u64::MAX - 1 → tail = u64::MAX (no overflow)
3. max_seq = 0 → tail = 1 (no overflow, single event case)
4. max_seq = 1 → tail = 2 (no overflow)

### 8. test_key_parse_no_panic

**Requirement:** REQ-vb-om21-07 | **Clause:** C-vb-om21-big-endian-max

**Given:**
- Various well-formed and malformed key bytes presented to the key parsing/extraction logic

**When:**
- The tail scan encounters keys of varying forms during prefix-bounded iteration

**Then:**
- Keys shorter than 17 bytes are rejected without panic
- Keys with correct prefix but insufficient length are rejected
- Keys with wrong prefix byte (not 0x11) are rejected by the prefix check
- Empty keys are rejected
- Keys with exactly 17 bytes but wrong prefix at offset 0 are rejected
- Panic-free behavior is maintained across all malformed inputs

**Test Variants:**
1. Key = [] (0 bytes) → rejected
2. Key = [0x11] (1 byte, prefix only) → rejected
3. Key = [0x11, run_id bytes] (9 bytes, prefix+run, no seq) → rejected
4. Key = [0x11, bad run bytes, seq bytes] (17 bytes but invalid RunId) → rejected
5. Key = [0x12, ...] (17 bytes, wrong prefix) → rejected by starts_with
6. Key = [0x11, run bytes, 0xFF × 8] (max sequence) → accepted, decoded correctly
7. Key with valid structure from a different run → accepted by prefix match, decoded correctly

### 9. test_replay_parity

**Requirement:** REQ-vb-om21-01 | **Clause:** C-vb-om21-replay-integrity
**Proof IDs:** PO-vb-om21-replay-parity-*

**Given:**
- A FjallJournal with contiguous events for RUN_X at sequences 0..N
- Events written through normal `append_journaled` path

**When:**
- `events_for_run(RUN_X)` (or equivalent with tail-scan fallback) replays events
- The tail is reconstructed through the scan fallback path

**Then:**
- All N+1 events are returned in contiguous sequence order (0, 1, ..., N)
- `WrongRun` is raised if any event's run field does not match RUN_X
- `SequenceGap` is raised if there is a gap in expected sequences
- The tail scan fallback does NOT weaken or replace these existing validation checks
- If the scan encounters a key that decodes to a wrong run, it still raises WrongRun

**Test Variants:**
1. Contiguous 0..5 → returns 6 events, tail=6
2. Gap at seq=3 (keys exist for 0,1,2,4,5) → returns events 0,1,2 then SequenceGap at seq=3
3. Event at seq=2 with wrong run field → WrongRun raised
4. Mixed: events at seq 0,1 (correct), seq 2 (wrong run) → WrongRun after first two events
5. Max-bounded: events at seq 0..100 with EventReplayLimit(50) → TooManyEvents at event 51

### 10. test_bounded_scan

**Requirement:** REQ-vb-om21-07 | **Clause:** C-vb-om21-prefix-bound
**Proof IDs:** PO-vb-om21-bounded-scan-*

**Given:**
- A FjallJournal with many events for RUN_X (100+ events)
- Other runs also exist in the keyspace

**When:**
- A pure tail query (no event replay) scans the prefix range
- The scan tracks only max_seq, not collecting events

**Then:**
- Accumulator state is O(1): a single `Option<u64>` for max_seq
- The scan does NOT collect all journal events into a Vec just to find the max
- The `classify_replay_push_len` function is NOT called for pure tail queries
- Memory usage does not grow with the number of events in the run

**Test Variants:**
1. 1000 events for RUN_X → tail computed without allocating 1000-event Vec
2. Other-run keys interleaved → scan terminates at first non-prefix key, not after collecting all
3. tail query after replay → tail is consistent with events_for_run output size
4. Repeated tail queries on same database → consistent, deterministic, idempotent

### 11. test_typed_error_distinction

**Requirement:** REQ-vb-om21-02 | **Clause:** C-vb-om21-metadata-validation
**Proof IDs:** PO-vb-om21-typed-errors-*

**Given:**
- Multiple scenarios producing different outcomes

**When:**
- The tail scan fallback produces results across these scenarios

**Then:**
- Match (declared == reconstructed) → success, no error
- Stale declared < reconstructed → TailMismatch (not MissingJournal)
- Absent journal → MissingJournal (not TailMismatch)
- Overflow → TailOverflow (not TailMismatch)
- Wrong run → WrongRun (not any tail-specific error)
- Sequence gap → SequenceGap (not any tail-specific error)
- All error types are distinct and non-overlapping in their triggering conditions

**Test Variants:**
1. declared=5, reconstructed=5 → Ok
2. declared=3, reconstructed=5 → TailMismatch
3. no events, RecoveryRequiresJournal → MissingJournal
4. max_seq=u64::MAX → TailOverflow
5. Event has wrong run in metadata → WrongRun takes precedence over tail mismatch

## Test Infrastructure Needs

### 1. Target File Registration

The new test file must be registered in the workspace manifest. Check `crates/workspace_tests/Cargo.toml` for existing `[[test]]` entries and add:

```toml
[[test]]
name = "restate_journal_tail_scan_fallback_tests"
path = "tests/restate_journal_tail_scan_fallback_tests.rs"
```

### 2. Required Dependencies

From `crates/workspace_tests/Cargo.toml`:
- `vb_storage` (already present) — FjallJournal, keys, errors, types
- `vb_core` (already present) — RunId, EventSeq
- `proptest` (already present from restate_fjall_keyspace_manifest_tests)
- `tempfile` or equivalent for temporary database directories (check existing patterns)

### 3. Helper Functions (Suggested)

```rust
/// Open a temporary FjallJournal for testing.
fn open_test_journal() -> FjallJournal { ... }

/// Write events seq 0..n for a given run.
fn seed_events(journal: &FjallJournal, run: RunId, n: u64) -> Result<(), JournalError> { ... }

/// Write a single event at a specific sequence.
fn seed_event(journal: &FjallJournal, run: RunId, seq: u64) -> Result<(), JournalError> { ... }

/// Helper to construct RunId from a u64 for deterministic test values.
fn run_id(val: u64) -> RunId { ... }
```

### 4. Test Organization

```rust
//! Journal tail scan fallback tests (vb-om21).
//!
//! Covers:
//! - Tail reconstruction from final durable run_event key (REQ-vb-om21-01)
//! - Prefix-bound scan termination (REQ-vb-om21-07)
//! - Big-endian max sequence selection (REQ-vb-om21-08)
//! - TailMismatch rejection (REQ-vb-om21-03)
//! - MissingJournal detection (REQ-vb-om21-04)
//! - Empty keyspace zero tail (REQ-vb-om21-05)
//! - Single event tail 1 (REQ-vb-om21-06)
//! - Checked arithmetic overflow (REQ-vb-om21-08)
//! - Panic-free key parsing (REQ-vb-om21-07)
//! - Replay parity preservation (REQ-vb-om21-01)
//! - O(1) bounded resource scan (REQ-vb-om21-07)
//! - Typed error distinction (REQ-vb-om21-02)
//!
//! Run with: `cargo test -p workspace_tests --test restate_journal_tail_scan_fallback_tests`

use proptest::prelude::*;
use vb_core::RunId;
use vb_storage::keys::{run_event_key, run_prefix_key};
use vb_storage::types::EventSeq;
use vb_storage::FjallJournal;

// ── Helper section ──

// ── Unit tests ──

#[test]
fn test_tail_scan_prefix_bound() { ... }

// ── Property tests ──

proptest! {
    #[test]
    fn test_big_endian_max_seq_proptest(a: u64, b: u64) { ... }
}
```

## Proof/Refinement Coverage Matrix

| Obligation ID | Proof ID | Verifier | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Evidence Command |
|---|---|---|---|---|---|---|---|

## Test Execution

### Primary Command

```bash
cargo nextest run -p workspace_tests --test restate_journal_tail_scan_fallback_tests
```

### Related Baseline Commands

```bash
# Existing key ordering baseline
cargo nextest run -p workspace_tests --test restate_fjall_keyspace_manifest_tests

# Storage unit tests
cargo nextest run -p vb_storage

# Full workspace test suite
cargo nextest run -p workspace_tests
```

## Mutation Resistance

Each test function must resist these mutations:
1. Line removal: removing the prefix check should cause the prefix-bound test to fail
2. Off-by-one: `max_seq + 1` vs `max_seq` — single event and zero tail tests catch this
3. Wrong arithmetic: using wrapping_add instead of checked_add — overflow test catches this
4. Incorrect byte range: using bytes 0..8 instead of 9..17 for sequence — big-endian max test catches this
5. Overly broad error: returning MissingJournal for TailMismatch case — typed error distinction catches this
6. Panic injection: inserting panic before length check — key parse test catches this

## Open Questions to Resolve Before Test Writing (State 9)

1. **API surface for tail query:** Is there a public `scan_tail` or `query_tail` function, or must tests access the `FjallJournal` through `events_for_run_from` or a new public method? Contract requires "public or integration-observable surface" (contract.md L48). Current `events_for_run_from` is `pub(crate)` — tests in `workspace_tests` cannot call it directly.

2. **Metadata injection mechanism:** Where does declared tail metadata enter the system? Exploration (contract.md L51) found no tail metadata field in `RunHeaderRecord`. Tests need to either:
   - Construct metadata externally and call a new comparison function
   - Use `inject_raw_event` to shape the keyspace, then call a tail query function

3. **Error type placement:** Should TailMismatch/MissingJournal/TailOverflow be variants on `JournalError`, `RecoveryError`, or a new tail-specific error type? Delivery scope (entries 9-10) points to `RecoveryError` in `recovery/types.rs` as well as `JournalError`. This decision affects which `use` imports tests need.

4. **FjallJournal construction in integration tests:** How do existing workspace tests construct a `FjallJournal`? Check `restate_fjall_keyspace_manifest_tests.rs` for the pattern (possibly an `open` function with a temp path).

5. **Concurrent run isolation:** Do tail scans need to hold a Fjall `Snapshot` or equivalent read consistency mechanism while computing max? The existing `events_for_run_from` holds a snapshot (line 100). Tests should follow the same pattern.
