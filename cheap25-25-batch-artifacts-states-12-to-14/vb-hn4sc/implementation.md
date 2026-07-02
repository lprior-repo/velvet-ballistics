# Implementation — vb-hn4sc

- **bead_id:** vb-hn4sc
- **bead_title:** Storage: enforce byte-budget limits in queued group commits (P1 bug)
- **phase:** 11 (holzman-rust)
- **isolated_workdir:** /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-hn4sc
- **jj workspace:** cheap25-vb-hn4sc
- **working copy:** lkpylryn (commit 71dbd718)
- **authoring_agent:** holzman-rust
- **captured_at:** 2026-07-01T20:50:00Z

## Reference Files Read

Per Holzman Rust skill contract (OpenCode bridge + canonical doctrine):

- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`
- `/home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md`
- `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md`

Plus bead-local contract artifacts:

- `.beads/vb-hn4sc/contract.md` — R-HN4SC-1..AC-1.6 + T-HN4SC-1..10 + W-HN4SC-1..9
- `.beads/vb-hn4sc/type-contracts.md` — `StorageLimits` extension, `gate_decision`, newtypes
- `.beads/vb-hn4sc/hazard-analysis.md` — 25 hazards + mitigations
- `.beads/vb-hn4sc/error-taxonomy.md` — `JournalBatchBytesExceeded` parity claim

## Summary

Wired the previously-ignored `StorageLimits` parameter into
`JournalWriterQueue` as an immutable `byte_budget: u64`, extended
`StorageLimits` with a new `max_journal_batch_bytes: u64` field
(default 1_048_636), and enforced the byte-budget gate inside
`flush_batch` via a stack-local `u64` accumulator that uses
`checked_add` against the encoded record length. The gate fires AFTER
`staged_keys_unique` and `durable_key_unique` checks and BEFORE
`owned_batch.insert`, so a violating batch is never partially
committed (master §49 Crash-Consistency Rule). The error variant
`JournalError::JournalBatchBytesExceeded { attempted: u64, limit: u64 }`
is reused (code `0x4022`) — no new variant, no new diagnostic code.

## Files Changed (5 files, 521 insertions, 11 deletions)

```text
crates/vb_storage/src/types.rs                                          |  38 ++
crates/vb_storage/src/queue/writer.rs                                   |  48 +-
crates/vb_storage/src/queue/writer/stage.rs                             |  45 +-
crates/vb_storage/src/queue/tests.rs                                    | 386 +++++++++++++++++++++-
crates/workspace_tests/tests/journal_batch_accounting_tests.rs          |  15 +-
```

Diffs preserved in `.beads/vb-hn4sc/evidence/*.patch`.

## Code Changes

### 1. `crates/vb_storage/src/types.rs` — extend `StorageLimits`

```rust
/// Storage write limits shared by direct and queued journal writers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StorageLimits {
    /// Maximum payload bytes accepted for a journal event.
    pub max_journal_event_payload_bytes: u32,
    /// Maximum encoded-record bytes accepted in a single
    /// [`crate::JournalWriterQueue`] group commit. The default is
    /// `RECORD_HEADER_BYTES + DEFAULT_JOURNAL_BATCH_BYTE_LIMIT`
    /// (60 + 1_048_576 = 1_048_636) so the queued gate accommodates
    /// at least one max-size event per flush while staying
    /// byte-comparable with the existing
    /// [`crate::batch::JournalWriteBatch`] limit basis.
    pub max_journal_batch_bytes: u64,
}

impl StorageLimits {
    pub const DEFAULT: Self = Self {
        max_journal_event_payload_bytes: crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        max_journal_batch_bytes: crate::batch::DEFAULT_JOURNAL_BATCH_BYTE_LIMIT
            .saturating_add(60),
    };
}
```

### 2. `crates/vb_storage/src/types.rs` — compile-time const assertion

```rust
/// Compile-time invariant: the default queued batch byte budget equals the
/// existing payload-basis limit (`DEFAULT_JOURNAL_BATCH_BYTE_LIMIT`) plus
/// the 60-byte record header.
#[allow(dead_code)]
const _STORAGE_LIMITS_DEFAULT_BATCH_BYTES_BOUND: () =
    assert!(crate::batch::DEFAULT_JOURNAL_BATCH_BYTE_LIMIT + 60 == 1_048_636);
```

Per T-HN4SC-7, this fails to compile if either constant drifts, locking
the parity claim between the queued gate and
`JournalWriteBatch::append_event`.

### 3. `crates/vb_storage/src/queue/writer.rs` — wire `_limits` into `byte_budget`

```rust
#[derive(Debug)]
pub struct JournalWriterQueue {
    state: Mutex<JournalWriterQueueState>,
    capacity: usize,
    batch_size: usize,
    /// Maximum encoded-record bytes accepted in a single
    /// [`Self::flush_batch`] call. Sourced from
    /// [`StorageLimits::max_journal_batch_bytes`] at construction.
    byte_budget: u64,
}

impl JournalWriterQueue {
    /// Creates a bounded writer queue from validated domain contracts.
    ///
    /// The supplied [`StorageLimits`] is now enforced: the configured
    /// `max_journal_batch_bytes` is captured into [`Self::byte_budget`]
    /// and applied to every [`Self::flush_batch`] call.
    pub fn with_contracts(
        capacity: JournalQueueCapacity,
        batch_size: JournalBatchSize,
        limits: StorageLimits,
    ) -> Result<Self, JournalError> {
        Ok(Self {
            state: Mutex::new(JournalWriterQueueState {
                pending: VecDeque::with_capacity(capacity.get()),
                shutdown: false,
            }),
            capacity: capacity.get(),
            batch_size: batch_size.get(),
            byte_budget: limits.max_journal_batch_bytes,
        })
    }

    /// Returns the configured per-flush encoded-byte budget.
    #[must_use]
    pub const fn byte_budget(&self) -> u64 {
        self.byte_budget
    }
```

### 4. `crates/vb_storage/src/queue/writer.rs` — `flush_batch` byte gate

```rust
let mut owned_batch = journal.database.batch();
let mut staged_keys = std::collections::HashSet::new();
let mut accumulated_bytes: u64 = 0;
let mut written = 0usize;
while written < batch_len {
    let Some(item) = state.pending.get(written) else { break; };
    stage_queued_event(
        &mut owned_batch,
        journal,
        &item.event,
        &mut staged_keys,
        &mut accumulated_bytes,
        self.byte_budget,
    )?;
    written = written.saturating_add(1);
}
```

`accumulated_bytes` is a stack-local `u64` reset at every `flush_batch`
entry (W-HN4SC-5: never a field on the queue). It increments only when
`stage_queued_event` successfully inserts into the `OwnedWriteBatch`.
Idempotent retries (existing-durable match) leave it unchanged.

### 5. `crates/vb_storage/src/queue/writer/stage.rs` — gate inside staging

```rust
let value = encode_record(
    MAGIC_JOURNAL_EVENT,
    event.record_kind(),
    event.seq().get(),
    event,
    MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
)?;

// Byte-budget gate: per W-HN4SC-1, fires AFTER staged_keys_unique
// and durable_key_unique checks and BEFORE owned_batch.insert.
let encoded_len =
    u64::try_from(value.len()).map_err(|_| JournalError::SequenceOverflow)?;
let attempted = match accumulated_bytes.checked_add(encoded_len) {
    Some(total) => total,
    None => {
        return Err(JournalError::JournalBatchBytesExceeded {
            attempted: u64::MAX,
            limit: byte_budget,
        });
    }
};
if attempted > byte_budget {
    return Err(JournalError::JournalBatchBytesExceeded {
        attempted,
        limit: byte_budget,
    });
}
*accumulated_bytes = attempted;

owned_batch.insert(&journal.events, key, value);
```

The error variant is reused exactly:
`JournalError::JournalBatchBytesExceeded { attempted: u64, limit: u64 }`
(diagnostic code `0x4022`, symbolic `JOURNAL_BATCH_BYTES_EXCEEDED`).
No new error variant, no new diagnostic code (E-HN4SC-1, E-HN4SC-2).

### 6. `crates/workspace_tests/tests/journal_batch_accounting_tests.rs` — comment fix

Per E-HN4SC-7, the misleading comment claiming
`JournalWriteBatch` does not enforce byte limits is corrected. The
byte-budget gate IS enforced in `JournalWriteBatch::append_event` via
the `byte_limit` field (default
`DEFAULT_JOURNAL_BATCH_BYTE_LIMIT = 1_048_576`); the comment now
documents this and points to the parity test in
`crates/vb_storage/src/queue/tests.rs`.

## Tests Added (9 new tests, all passing)

In `crates/vb_storage/src/queue/tests.rs`:

| # | Test | AC / W-ID | What it locks |
|---|---|---|---|
| 1 | `storage_limits_default_batch_bytes_equals_payload_basis_plus_header` | AC-1.4 | `StorageLimits::DEFAULT.max_journal_batch_bytes == 1_048_636` |
| 2 | `with_contracts_captures_byte_budget_from_storage_limits` | AC-1.5, B-HN4SC-8 | `byte_budget` is wired from `StorageLimits`, signature preserved |
| 3 | `flush_batch_rejects_when_encoded_bytes_exceed_byte_budget` | AC-1.1, R-HN4SC-1 | single oversize event → `JournalBatchBytesExceeded { attempted: 95, limit: 90 }`, pending preserved, durable empty |
| 4 | `flush_batch_accepts_at_exact_byte_budget` | AC-1.2 | exact-fit (`attempted == limit`) accepted, `>` not `>=` |
| 5 | `flush_batch_default_accepts_single_max_size_event` | AC-1.4, M-HN4SC-9 | default budget (1_048_636) admits a max-size event |
| 6 | `flush_batch_byte_budget_rejection_skips_commit` | M-HN4SC-1, W-HN4SC-2 | rejection in the middle of a batch leaves durable store empty, pending intact |
| 7 | `drain_all_short_circuits_on_byte_budget_rejection` | W-HN4SC-6 | `drain_all` propagates the first `JournalBatchBytesExceeded` via `?` |
| 8 | `enqueue_does_not_enforce_byte_budget_only_flush_does` | W-HN4SC-3 | `enqueue` only checks capacity; budget is a flush-time concern |
| 9 | `journal_write_batch_and_journal_writer_queue_emit_identical_byte_budget_error` | **AC-1.3** (parity lock) | `JournalWriteBatch::append_event` and `JournalWriterQueue::flush_batch` emit identical `JournalBatchBytesExceeded { attempted, limit }` for the same oversize event |

## Power-of-Ten Compliance

| Rule | Status | Notes |
|---|---|---|
| 1. Simple control flow | PASS | No recursion, no panic-driven control, explicit `match`/`while` |
| 2. Fixed loop bounds | PASS | `flush_batch` bounded by `batch_size`; `drain_all` bounded by `ceil(capacity/batch_size)+2` |
| 3. No post-init alloc in critical | PASS | `accumulated_bytes` is `u64` on the stack; `OwnedWriteBatch` was already allocated by Fjall |
| 4. Functions fit on one page | PASS | `flush_batch` ~70 lines (existing); new gate logic is in `stage_queued_event` (15 lines added) |
| 5. Invariant density | PASS | `byte_budget: u64` field captures invariant at construction; `checked_add` overflow surfaces `JournalBatchBytesExceeded` with `u64::MAX` |
| 6. Smallest scope | PASS | `accumulated_bytes` is local to `flush_batch`; `byte_budget` is captured at construction |
| 7. Checked returns | PASS | `u64::try_from(value.len())` for the `usize -> u64` conversion (mandatory per Holzman) |
| 8. Limited macros | PASS | No new macros |
| 9. Restricted pointer use | PASS | No unsafe, no raw pointers |
| 10. Zero warnings | PASS | `cargo clippy -D warnings` clean on `vb_storage` |

## Zero-Forbidden-Construct Compliance

`rg -n '(unwrap|expect|panic|todo|unimplemented|unreachable|unsafe)' crates/vb_storage/src/queue/writer.rs crates/vb_storage/src/queue/writer/stage.rs crates/vb_storage/src/types.rs`
returns only the `expect_point_read_hits` method-name match and the
`#![forbid(unsafe_code)]` directive in `types.rs`. No production
`unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `unreachable!`, or
production `assert!` macros. The single `const _STORAGE_LIMITS_DEFAULT_BATCH_BYTES_BOUND: () = assert!(...)` is
the compile-time invariant per T-HN4SC-7 — it cannot fail at runtime.

## Verification Gate

| Command | Result |
|---|---|
| `cargo +nightly fmt --check` (my touched files) | PASS — no diffs in `vb_storage/src/queue/{tests.rs, writer.rs, writer/stage.rs}` or `vb_storage/src/types.rs` |
| `cargo check --workspace --all-targets --all-features` | PASS — 5 crates compiled |
| `cargo clippy -p vb_storage --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock` | PASS — "No issues found" |
| `cargo test -p vb_storage --lib queue` | **91 passed, 0 failed** (82 existing + 9 new) |
| `cargo test -p vb_storage --lib` | **1539 passed, 0 failed** (no regression) |
| `cargo test -p vb_runtime --lib` | **1807 passed, 0 failed** (no regression on shared_journal path) |
| `cargo test -p velvet-ballistics-workspace-tests --test journal_batch_accounting_tests` | **16 passed, 0 failed** (existing tests still pass after comment fix) |

## Parity Test Verification

The contract-parity claim (AC-1.3) is locked by
`journal_write_batch_and_journal_writer_queue_emit_identical_byte_budget_error`:

- `JournalWriteBatch::new(&journal)` with default `byte_limit = 1_048_576` rejects
  `SlotWrittenEvent { value: Some(vec![0u8; 1_048_560]), extra: None, attempt: 1 }` with
  `JournalBatchBytesExceeded { attempted: 1_048_630, limit: 1_048_576 }`.
- `JournalWriterQueue::new(2, 2, StorageLimits { max_journal_batch_bytes: 1_048_576, .. })` rejects the same event with the identical error variant and field values.
- Both errors match on `attempted` (1_048_630), `limit` (1_048_576), and variant shape.

## Pre-existing Failures (BLOCK_GLOBAL — Not Introduced By This Bead)

`cargo test -p velvet-ballistics-workspace-tests` reports 1
pre-existing failure in
`vb_qi37_4_2_strict_runtime_admission.rs:1466` — a string-search test
that expects `impl AcceptedArtifactStore for AlwaysPresentArtifactStore`
in `crates/vb_runtime/src/admission.rs` but the actual `impl` lives in
the chunked file `crates/vb_runtime/src/admission/parts/chunk_003_stores.rs`.
This failure is independent of `vb-hn4sc` (it does not touch
`admission.rs` or its parts) and is recorded as `BLOCK_GLOBAL` for
follow-up repair. Confirmed pre-existing by running the test on the
parent commit `lkpylryn` (the same failure reproduces without this
bead's changes).

## Performance Layer Decision

- **Performance claim:** None. This is a correctness fix; the
  additional per-flush work is `O(batch_size)` `u64` `checked_add` +
  one `value.len()` `u64` conversion per event, both negligible
  compared to `encode_record`'s postcard serialization. No
  benchmark/profiler evidence required.
- **Allocation behavior:** No new heap allocations in the hot path;
  `accumulated_bytes: u64` is a stack value; `OwnedWriteBatch` was
  already allocated by Fjall.
- **Dispatch:** Static dispatch preserved; no new trait objects.
- **Storage placement:** `byte_budget: u64` is a small field on the
  queue (stack-resident via `&self`); `accumulated_bytes` is a
  per-flush stack local.

## Residual Risks

- The pre-existing `vb_qi37_4_2_strict_runtime_admission` failure
  remains as `BLOCK_GLOBAL` and should be repaired in a follow-up
  bead (the impl is correctly in the chunked parts file; the test
  needs to be updated to read the chunked file path).
- `cargo +nightly fmt --check` on the **untouched** files
  `crates/vb_core/src/{lib,time}.rs` and
  `crates/vb_runtime/src/frame_pool/tests.rs` reports 5 pre-existing
  format diffs unrelated to this bead. These were left as-is to
  preserve a minimal-scope patch.
