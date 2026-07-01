# Contract — vb-pg2wq

**Bead:** vb-pg2wq — Tests: make duplicate-event test assert one exact contract (P1 bug)
**Lane:** Rust-local + test-only assertion repair
**Verifier scope:** `proptest` only; no new Kani/Verus/Flux/Loom required.

## Canonical Contract (Single Clause)

For each of the 5 proptest functions listed in `§5 Per-File Change Specification`, the test assertion on `result: Result<(), JournalError>` returned by `JournalWriteBatch::append_event(&event)` MUST be rewritten from the weak form

```rust
let is_dup = matches!(result, Err(JournalError::DuplicateEvent { .. }));
prop_assert!(is_dup);
```

(or the PS_004 variant `let duplicate_event = matches!(append_result, Err(JournalError::DuplicateEvent { .. })); prop_assert!(duplicate_event);`)

to the strong form

```rust
prop_assert!(matches!(
    result,
    Err(JournalError::DuplicateEvent { run: r, seq: s })
        if r == RunId::new(run) && s == EventSeq::new(seq)
));
```

where `run` and `seq` are the proptest input bindings already present in the function signature.

**Reference strong pattern:** `crates/vb_storage/src/tests.rs:1344-1367` (`fn duplicate_event_returns_exact_run_and_seq`). The proptest analog mirrors the field-bound `matches!` guard idiom; the unit-test `let-else` exhaustive panic pattern is preserved for any non-proptest duplicate-event test (none in scope for this bead).

**Production contract being pinned:** `crates/vb_storage/src/batch/append_event.rs:61-67` — `self.journal.events.contains_key(key)?` branch returns `Err(JournalError::DuplicateEvent { run: event.run_id(), seq: event.seq() })` and sets `self.aborted = true`.

## EARS Obligations

### Obligation 1 — Exact-Tuple Pin

> **WHEN** the proptest executes the cross-batch duplicate scenario (`b1.commit()` followed by `b2.append_event(&event)`),
> **SHALL** the test assertion bind both `run: RunId` and `seq: EventSeq` from the `Err` payload and assert typed equality against the proptest inputs `run`/`seq` re-bagged via `RunId::new(...)` / `EventSeq::new(...)`.

**Coverage:** All 6 weak occurrences in 5 functions across 4 files (see `§5`).

### Obligation 2 — Variant Discriminant

> **WHEN** `b2.append_event(&event)` returns any `Err(JournalError::Variant)` other than `DuplicateEvent`,
> **SHALL** the test assertion fail (panic via `let-else` or `prop_assert!(false)` via `matches!` guard returning `false`).

**Coverage:** All 6 occurrences. Sibling variants that MUST be rejected: `DuplicateStagedKey`, `QueueFull`, `KeyCapacity`, `BatchAborted`, `InvalidEvent`, `Encode`, `Fjall`, `JournalBatchBytesExceeded`, `PayloadTooLarge`, `WriteLockPoisoned`, `QueueShutdown`, `QueueCapacity`, `SequenceOverflow`, etc. The exhaustive `let-else` (for unit tests) and the named-variant `matches!` guard (for proptest) cover all of them.

### Obligation 3 — `Ok(())` Rejection

> **WHEN** `b2.append_event(&event)` returns `Ok(())` in the cross-batch duplicate scenario,
> **SHALL** the test assertion fail (silent-overwrite regression).

**Coverage:** All 6 occurrences. The exhaustive `let-else` and the `matches!` guard both reject `Ok(())`.

### Obligation 4 — Preserve All Other Assertions

> **WHEN** the proptest function contains assertions beyond the duplicate-event check (e.g., `prop_assert_eq!(b2.len(), 0)`, `prop_assert!(b2.is_aborted())`, `prop_assert_eq!(journal.events_for_run(...).len(), 1)`, `prop_assert!(matches!(commit_result, Err(JournalError::BatchAborted)))`),
> **SHALL** those assertions be preserved verbatim.

**Coverage:** PS_004 (`ps004_no_persist`, `ps004_empty_commit_after_rej`) has 3-4 secondary assertions each that MUST be preserved.

### Obligation 5 — Preserve Proptest Strategy

> **WHEN** the proptest function signature declares `run in 1u64..1000u64, seq in 0u64..100u64` (or the PS_004-no-persist variant `run in 1u64..1000u64`),
> **SHALL** the signature be preserved verbatim.

**Coverage:** All 5 functions.

### Obligation 6 — No Production Change

> **WHILE** this bead is in flight,
> **SHALL NOT** any production source file under `crates/vb_storage/src/` be modified. The fix is test-only.

**Coverage:** `crates/vb_storage/src/batch/append_event.rs:42-67`, `crates/vb_storage/src/error/mod.rs:30-31`, and all other production sources remain unchanged.

### Obligation 7 — No Cargo.toml Change

> **WHILE** this bead is in flight,
> **SHALL NOT** any `Cargo.toml` file be modified.

**Coverage:** All `Cargo.toml` files in the workspace.

### Obligation 8 — No Forbidden Constructs

> **WHEN** the test assertion is rewritten,
> **SHALL NOT** the new code introduce `unsafe`, `unwrap`, `expect` (on the negative-path `result`), `todo`, `unimplemented`, `dbg`, unchecked indexing, unchecked slicing, unchecked casts, unchecked arithmetic, runtime YAML, runtime JSON, or runtime HTTP.

**Coverage:** The new assertion uses only `prop_assert!` with `matches!(...) if guard` and `RunId::new(...)` / `EventSeq::new(...)` smart constructors. No forbidden constructs.

### Obligation 9 — Preserve Helpers

> **WHEN** the proptest file imports or defines helpers (`make_event`, `temp_journal`),
> **SHALL** those helpers be preserved verbatim.

**Coverage:** All 4 target files.

---

## Per-File Change Specification (Next-Handoff Plan)

### File 1: `crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs`

**Function:** `ps001_duplicate_rejected` (lines 69-79)

**Current weak assertion (lines 77-78):**
```rust
let is_dup = matches!(result, Err(JournalError::DuplicateEvent { .. }));
prop_assert!(is_dup);
```

**Replacement:**
```rust
prop_assert!(matches!(
    result,
    Err(JournalError::DuplicateEvent { run: r, seq: s })
        if r == RunId::new(run) && s == EventSeq::new(seq)
));
```

**Imports:** `RunId` already imported via `use vb_core::{RunId, WorkflowDigest};`. `EventSeq` already imported via `use vb_storage::EventSeq;`. No new imports.

**Secondary assertions:** None — `ps001_duplicate_rejected` only asserts the duplicate-event tuple. Preserved (no change).

### File 2: `crates/vb_storage/tests/proptest_vb_vzcuf_PS_003.rs`

**Function:** `ps003_dup_fields` (lines 55-65)

**Current weak assertion (lines 63-64):**
```rust
let is_dup = matches!(result, Err(JournalError::DuplicateEvent { .. }));
prop_assert!(is_dup);
```

**Replacement:**
```rust
prop_assert!(matches!(
    result,
    Err(JournalError::DuplicateEvent { run: r, seq: s })
        if r == RunId::new(run) && s == EventSeq::new(seq)
));
```

**Imports:** Same as File 1; no new imports.

**Secondary assertions:** None. The function name (`ps003_dup_fields`) is currently a lie (it doesn't assert any fields); the fix delivers on the name's promise.

### File 3: `crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs`

**Function A:** `ps004_no_persist` (lines 39-54)

**Current weak assertion (lines 47-48):**
```rust
let duplicate_event = matches!(append_result, Err(JournalError::DuplicateEvent { .. }));
prop_assert!(duplicate_event);
```

**Replacement:**
```rust
prop_assert!(matches!(
    append_result,
    Err(JournalError::DuplicateEvent { run: r, seq: s })
        if r == RunId::new(run) && s == EventSeq::new(seq)
));
```

(Note: `seq` is fixed at 0 in this function's setup `let event = make_event(run, 0);` — so the strong assertion pins `seq = EventSeq::new(0)`.)

**Secondary assertions (preserved verbatim):**
- Line 49: `prop_assert!(b2.is_aborted());`
- Line 51: `prop_assert!(matches!(commit_result, Err(JournalError::BatchAborted)));`
- Line 53: `prop_assert_eq!(events.len(), 1);`

**Imports:** `JournalError` already imported via `use vb_storage::{EventSeq, JournalError};`. `RunId` already imported via `use vb_core::{RunId, WorkflowDigest};`. No new imports.

**Function B:** `ps004_empty_commit_after_rej` (lines 84-98)

**Current weak assertion (lines 93-94):**
```rust
let duplicate_event = matches!(append_result, Err(JournalError::DuplicateEvent { .. }));
prop_assert!(duplicate_event);
```

**Replacement:**
```rust
prop_assert!(matches!(
    append_result,
    Err(JournalError::DuplicateEvent { run: r, seq: s })
        if r == RunId::new(run) && s == EventSeq::new(seq)
));
```

**Secondary assertions (preserved verbatim):**
- Line 95: `prop_assert!(b2.is_aborted());`
- Line 97: `prop_assert!(matches!(commit_result, Err(JournalError::BatchAborted)));`

**Imports:** Same as Function A; no new imports.

### File 4: `crates/vb_storage/tests/proptest_vb_vzcuf_PS_008.rs`

**Function:** `ps008_dup_before_queue` (lines 27-36)

**Current weak assertion (line 35):**
```rust
let is_dup = matches!(result, Err(JournalError::DuplicateEvent { .. })); prop_assert!(is_dup);
```

**Replacement:**
```rust
prop_assert!(matches!(
    result,
    Err(JournalError::DuplicateEvent { run: r, seq: s })
        if r == RunId::new(run) && s == EventSeq::new(seq)
));
```

**Imports:** Same as File 1; no new imports.

**Secondary assertions:** None.

### File 5: `crates/vb_storage/tests/proptest_vb_vzcuf_PS_009.rs`

**Function:** `ps009_dup_rejected` (lines 27-37)

**Current weak assertion (lines 35-36):**
```rust
let is_dup = matches!(result, Err(JournalError::DuplicateEvent { .. }));
prop_assert!(is_dup);
```

**Replacement:**
```rust
prop_assert!(matches!(
    result,
    Err(JournalError::DuplicateEvent { run: r, seq: s })
        if r == RunId::new(run) && s == EventSeq::new(seq)
));
```

**Imports:** Same as File 1; no new imports.

**Secondary assertions:** None.

---

## Verification Plan (Per-File Cargo Test Commands)

```
cargo test -p vb_storage --test proptest_vb_vzcuf_PS_001 ps001_duplicate_rejected --no-fail-fast
cargo test -p vb_storage --test proptest_vb_vzcuf_PS_003 ps003_dup_fields --no-fail-fast
cargo test -p vb_storage --test proptest_vb_vzcuf_PS_004 ps004_no_persist --no-fail-fast
cargo test -p vb_storage --test proptest_vb_vzcuf_PS_004 ps004_empty_commit_after_rej --no-fail-fast
cargo test -p vb_storage --test proptest_vb_vzcuf_PS_008 ps008_dup_before_queue --no-fail-fast
cargo test -p vb_storage --test proptest_vb_vzcuf_PS_009 ps009_dup_rejected --no-fail-fast
```

Each command should pass with the strong assertion (the production contract already holds, and the test now verifies it field-by-field). A failure would indicate either:
- A production regression (production code returns wrong tuple) — fix production.
- A test-setup bug — fix the test.

**Negative test:** To prove the strong assertion catches regressions, the test-writer may temporarily mutate production to return `DuplicateEvent { run: RunId::new(0), seq: EventSeq::new(0) }` and observe the test fail. Then revert the production mutation. (This is a manual verification, not an automated test in the test suite.)

---

## Adjacent (Out-of-Scope) Follow-Up Candidates

These weak `..` patterns exist in the codebase but are NOT modified by this bead:

| File | Function | Lines | Status |
|------|----------|-------|--------|
| `crates/vb_storage/src/batch/t_append_event.rs` | `batch_append_event_rejects_duplicate_event` | 20-43 | Out of scope; follow-up bead candidate |
| `crates/vb_storage/src/batch/t_byte_accounting_part2.rs` | `rejected_duplicate_event_not_staged_in_batch` | 84-106 | Out of scope; follow-up bead candidate |
| `crates/vb_storage/src/batch/t_byte_accounting_part3.rs` | `duplicate_detection_fires_before_count_check` | 5-20 | Out of scope; follow-up bead candidate |
| `crates/vb_storage/src/batch/t_byte_accounting_part3.rs` | `duplicate_and_queue_full_conflict_duplicate_wins` | 55-70 | Out of scope; follow-up bead candidate |
| `crates/vb_storage/src/batch/t_byte_accounting_part4.rs` | `cross_batch_duplicate_is_rejected_with_duplicate_event` | 5-20 | Out of scope; follow-up bead candidate |
| `crates/vb_storage/src/batch/t_byte_accounting_part4.rs` | `duplicate_event_aborts_batch` | 22-36 | Out of scope; follow-up bead candidate |
| `crates/vb_storage/src/batch/t_byte_accounting_part4.rs` | `e2e_aborted_batch_commit_returns_typed_batch_aborted_error` | 76-104 | Out of scope; follow-up bead candidate |
| `crates/vb_storage/src/batch/t_byte_accounting_part4.rs` | `append_strict_batch_atomicity_rolls_back_on_duplicate` | 106-129 | Out of scope; follow-up bead candidate |
| `crates/vb_storage/src/tests.rs` | `duplicate_event_append_is_rejected` | 837-851 | Out of scope (not in proptest lane) |
| `crates/workspace_tests/tests/journal_side_index_contracts.rs` | `two_in_flight_same_run_seq` | 495-531 | Borderline; has additional `is_aborted`/`len`/`event-count` assertions that partially mitigate; out of scope |

These are flagged for follow-up beads but not modified here.

---

## Contract Acceptance Criteria

A reviewer may approve this bead's contract artifacts iff:

1. All 9 contract artifacts are present (`domain-model.md`, `type-contracts.md`, `workflow-model.md`, `error-taxonomy.md`, `boundary-map.md`, `hazard-analysis.md`, `contract.md`, `proof-seeds.jsonl`, `traceability-matrix.jsonl`).
2. `proof-seeds.jsonl` is valid `proof-seed/v1` and passes `jq -c .` per line.
3. `traceability-matrix.jsonl` is valid JSONL and passes `jq -c .` per line.
4. The canonical contract clause (top of this document) is unambiguous and maps to all 6 weak occurrences.
5. The per-file change specification enumerates the exact replacement for each of the 6 weak occurrences.
6. The verifier plan is bounded to `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_00X <fn_name>` (no new harnesses required).
7. The Kani binding is acknowledged as already-present (no new Kani needed).
8. The production-binding gate is acknowledged: no production change means no Verus mirror needed for this bead (the test fix strengthens an already-bound contract).

The contract is NOT proof-complete; proof-planner will follow.