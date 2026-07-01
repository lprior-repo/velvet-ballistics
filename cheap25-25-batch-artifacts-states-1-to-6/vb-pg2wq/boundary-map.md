# Boundary Map — vb-pg2wq

**Bead:** vb-pg2wq — Tests: make duplicate-event test assert one exact contract (P1 bug)
**Lane:** Rust-local + test-only assertion repair

## Boundary Diagram

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  TEST CODE (in scope: 5 proptest functions in 4 files)                       │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │  proptest! {                                                          │  │
│  │      fn ps001_duplicate_rejected(run in 1u64..1000, seq in 0u64..100) │  │
│  │      fn ps003_dup_fields(run in 1u64..1000, seq in 0u64..100)         │  │
│  │      fn ps004_no_persist(run in 1u64..1000)                          │  │
│  │      fn ps004_empty_commit_after_rej(run in 1u64..1000, seq in 0u64..100) │
│  │      fn ps008_dup_before_queue(run in 1u64..1000, seq in 0u64..100)  │  │
│  │      fn ps009_dup_rejected(run in 1u64..1000, seq in 0u64..100)      │  │
│  │  }                                                                    │  │
│  │      │                                                                │  │
│  │      │ make_event(run, seq) — pure construction                       │  │
│  │      ▼                                                                │  │
│  │      ┌──────────────────────────────────────────────────────────┐     │  │
│  │      │  JournalEvent::RunAccepted { run, seq, workflow }       │     │  │
│  │      │  (immutable, copy-by-reference)                          │     │  │
│  │      └──────────────────────────────────────────────────────────┘     │  │
│  │      │                                                                │  │
│  │      │ temp_journal() — I/O BOUNDARY                                 │  │
│  │      ▼                                                                │  │
│  │      ┌──────────────────────────────────────────────────────────┐     │  │
│  │      │  (tempfile::TempDir, FjallJournal)                        │     │  │
│  │      │  filesystem-backed; ephemeral; fsynced at commit         │     │  │
│  │      └──────────────────────────────────────────────────────────┘     │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                              │                                               │
│                              │ b1/b2.append_event(&event)                    │
│                              ▼                                               │
├──────────────────────────────┼───────────────────────────────────────────────┤
│  PRODUCTION CODE (out of scope; bound-by-contract)                          │
│                              ▼                                               │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │  JournalWriteBatch::append_event (crates/vb_storage/src/batch/...)   │   │
│  │    1. run_event_key construction                                      │   │
│  │    2. event.is_valid() guard                                         │   │
│  │    3. staged_event_keys.contains(&key) -> DuplicateStagedKey         │   │
│  │    4. journal.events.contains_key(key) -> DuplicateEvent + abort  ←──┤   │
│  │    5. count admission -> QueueFull                                    │   │
│  │    6. encode_record -> PayloadTooLarge                                │   │
│  │    7. byte admission -> JournalBatchBytesExceeded                     │   │
│  │    8. inner.insert + action_index staging                             │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                              │                                               │
│                              │ Err(DuplicateEvent { run, seq })              │
│                              ▼                                               │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │  Result<(), JournalError>  ──→  TEST ASSERTION (the fix)              │   │
│  │  Result<(), JournalError>  ──→  Production behavior (already correct)│   │
│  └──────────────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────────────┘
```

## Boundary 1: Proptest Strategy → Test Body

**Type:** Pure-function boundary (no I/O).
**Inputs:** Proptest generates `run: u64` in `1u64..1000u64` and `seq: u64` in `0u64..100u64` (or `run` only, for PS_004 `no_persist` which fixes `seq = 0`).
**Validation:** Proptest bounds are tight (`1..1000`, `0..100`); no overflow risk in `make_event` which only constructs a `JournalEvent::RunAccepted`.
**Test fix:** No change to the strategy signature. The `run`/`seq` bindings flow into the test body unchanged.

## Boundary 2: `make_event` Helper → Production Event Type

**Type:** Pure constructor.
**Signature:** `fn make_event(run: u64, seq: u64) -> JournalEvent`.
**Returns:** `JournalEvent::RunAccepted { run: RunId::new(run), seq: EventSeq::new(seq), workflow: WorkflowDigest::from_bytes([0u8; 32]) }`.
**Test fix:** No change. The helper is correct and is preserved verbatim across all 5 proptest files.

## Boundary 3: `temp_journal` Helper → Filesystem

**Type:** I/O boundary (filesystem-backed Fjall journal).
**Signature:** `fn temp_journal() -> (tempfile::TempDir, FjallJournal)`.
**Returns:** `(TempDir, FjallJournal)` where `TempDir` is auto-cleaned on drop.
**Failure modes:** `tempdir()` may fail (rare; CI runner out of space); `FjallJournal::open` may fail if the temp path is invalid. Both currently use `.expect(...)`. These are setup, not assertion; preserved as-is.
**Test fix:** No change.

## Boundary 4: `JournalWriteBatch::append_event` (Production)

**Type:** Imperative-shell boundary; takes `&mut self` and `&JournalEvent`; returns `Result<(), JournalError>`.
**Source:** `crates/vb_storage/src/batch/append_event.rs:42-67`.
**Contract:** `run == event.run_id()`, `seq == event.seq()` (typed equality, no field mutation, no synthesis).
**Side effect on `DuplicateEvent`:** `self.aborted = true`.
**Test fix:** No change to production. The test fix is on the **consumption side**: the test assertion must reflect the contract.

## Boundary 5: Test Assertion (THE FIX)

**Type:** Pure-function boundary (no I/O).
**Inputs:** `result: Result<(), JournalError>` returned by `b2.append_event(&event)`.
**Required transformation:**

| Before (weak) | After (strong) |
|--------------|---------------|
| `let is_dup = matches!(result, Err(JournalError::DuplicateEvent { .. })); prop_assert!(is_dup);` | `prop_assert!(matches!(result, Err(JournalError::DuplicateEvent { run: r, seq: s }) if r == RunId::new(run) && s == EventSeq::new(seq)));` |

**Forbidden transformation:**

- `assert!(result.is_err())` (variant-blind).
- `matches!(result, Err(JournalError::DuplicateEvent { run: _, seq: _ }))` (still wildcard).
- `assert_eq!(format!("{result:?}"), "...")` (stringly).

## Boundary 6: `RunId::new` / `EventSeq::new` Smart Constructors

**Type:** Pure constructor, no validation (the underlying `u64` is total).
**Inputs:** `u64` from the proptest strategy.
**Returns:** Newtype wrapper preserving equality.
**Test fix:** Used to re-bag the proptest input for comparison. `RunId::new(run)` is `const fn`; same for `EventSeq::new(seq)`.

## Boundary 7: `PartialEq` on `RunId` / `EventSeq`

**Type:** Pure derived trait.
**Contract:** `RunId::new(a) == RunId::new(b)` iff `a == b`; `EventSeq::new(a) == EventSeq::new(b)` iff `a == b`. Both are `#[repr(transparent)]` newtypes, so `==` is structural.
**Test fix:** The guard `r == RunId::new(run)` is a typed equality check on `RunId == RunId`, not a `u64 == u64` cast.

## Boundary 8: Diagnostic Code Path (Out of Scope)

**Type:** Side channel for observability.
**Source:** `crates/vb_storage/src/error/codes.rs:104,197` maps `DuplicateEvent { .. }` → `DUPLICATE_EVENT_CODE = "DUPLICATE_EVENT"`.
**Test fix:** Out of scope. Future enhancement could pin the diagnostic code in addition to the typed tuple.

## Boundary 9: Kani Proof Harness (Existing, Out of Scope)

**Type:** Formal verification artifact.
**Source:** `crates/vb_storage/src/kani_vb_vzcuf_ps004.rs:48-59` already models `DuplicateEvent { run: r, seq: s }` with `r == run && s == seq` guards.
**Test fix:** No change. The test fix strengthens the runtime↔Kani binding without requiring any new harness.

## Pure-Core / Imperative-Shell / Async-Shell / Storage Boundaries

| Boundary class | Surface | Test involvement |
|----------------|---------|------------------|
| Pure core | `make_event`, `RunId::new`, `EventSeq::new`, `PartialEq` comparisons | Used by test body. No I/O. |
| Imperative shell | `JournalWriteBatch::new`, `append_event`, `commit`, `is_aborted` | Called by test body. Out of scope for change. |
| Async shell | (none) | Not used by any of the 5 tests. |
| Storage boundary | `temp_journal()` (filesystem-backed `FjallJournal`) | Used in setup. Ephemeral. Out of scope. |
| Network boundary | (none) | Not used. |
| Time boundary | (none) | Not used. |
| FFI boundary | (none) | Not used. |
| `unsafe` boundary | (none) | `forbid(unsafe_code)` in all crates. |
| Parser/codec boundary | `encode_record` (used by other tests in PS_001/PS_003/PS_008/PS_009 but NOT by the duplicate tests) | Out of scope for THIS bead. |

## Boundaries NOT Modified by This Bead

- Production `JournalWriteBatch::append_event` (imperative shell).
- Production `JournalError` enum and its variants (type declaration).
- Diagnostic code mapping (`codes.rs`).
- Kani harness `kani_vb_vzcuf_ps004.rs`.
- `Cargo.toml` (no dependency change).
- Helper functions `make_event` and `temp_journal` (preserved verbatim).
- Proptest strategy signatures (preserved verbatim).

## Boundaries Modified by This Bead

- Test-side assertion body of 5 proptest functions in 4 files (`proptest_vb_vzcuf_PS_001/003/004/008/009.rs`).
- The 6 weak `matches!` patterns are replaced by the field-bound guard pattern.

## Cross-Cutting Boundary Concerns

| Concern | Status |
|---------|--------|
| `forbid(unsafe_code)` | Preserved. The test fix adds no `unsafe`. |
| `no panic` in production | Preserved. The `panic!` in `let-else` is test-only (canonical reference at `tests.rs:1363`). |
| `no unwrap/expect` on negative-path result | Preserved. The strong assertion does NOT unwrap `result`; it pattern-binds it. |
| Forbidden arithmetic | Preserved. No arithmetic in the assertion. |
| Bounded state | Preserved. Proptest input space is bounded; `make_event` only constructs. |
| Determinism | Preserved. Proptest with `tempdir()` is deterministic across reruns. |