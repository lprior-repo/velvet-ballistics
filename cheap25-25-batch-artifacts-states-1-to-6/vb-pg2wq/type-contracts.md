# Type Contracts — vb-pg2wq

**Bead:** vb-pg2wq — Tests: make duplicate-event test assert one exact contract (P1 bug)
**Lane:** Rust-local + test-only assertion repair

## Contract Type 1: Strong Field-Bound `matches!` Guard

The replacement assertion for each of the 6 weak occurrences uses a `matches!` macro with a **field-bound guard** that pins the exact `run`/`seq` payload.

### Weak form (FORBIDDEN — current state)

```rust
let is_dup = matches!(result, Err(JournalError::DuplicateEvent { .. }));
prop_assert!(is_dup);
```

This accepts ANY `DuplicateEvent` tuple regardless of field values. A regression that returns `DuplicateEvent { run: RunId::new(0), seq: EventSeq::new(0) }` (or any other tuple) still passes.

### Strong form (REQUIRED — replacement)

```rust
prop_assert!(matches!(
    result,
    Err(JournalError::DuplicateEvent { run: r, seq: s })
        if r == RunId::new(run) && s == EventSeq::new(seq)
));
```

The guard binds `r: RunId` and `s: EventSeq` and asserts equality against the proptest inputs `run: u64`/`seq: u64` re-bagged via the smart constructors. Any field mutation or wrong tuple fails.

### Equality contract

- `RunId::PartialEq` is derived (`#[derive(PartialEq, Eq)]` via `numeric_id!` macro at `crates/vb_core/src/ids/mod.rs:24-55`). `RunId::new(a) == RunId::new(b)` iff `a == b`.
- `EventSeq::PartialEq` is hand-derived (`#[derive(PartialEq, Eq)]` at `crates/vb_storage/src/types.rs:69-71`). `EventSeq::new(a) == EventSeq::new(b)` iff `a == b`.
- The comparison `r == RunId::new(run)` is a typed equality check; no `as u64` cast is required because both sides are newtype-wrapped `u64`.

## Contract Type 2: `let-else` Exhaustiveness Bind (for deterministic unit tests)

For non-proptest test functions (e.g., the canonical reference at `tests.rs:1344-1367`), the pattern is:

```rust
let Err(JournalError::DuplicateEvent { run, seq }) = result else {
    panic!("expected DuplicateEvent, got {:?}", result);
};
assert_eq!(run, RunId::new(EXPECTED_RUN));
assert_eq!(seq, EventSeq::new(EXPECTED_SEQ));
```

### Exhaustiveness contract

The `let-else` pattern is exhaustive over `result: Result<(), JournalError>`:

- `Ok(())` → falls through `else` branch → `panic!`.
- `Err(JournalError::QueueFull)` → falls through `else` → `panic!`.
- `Err(JournalError::BatchAborted)` → falls through `else` → `panic!`.
- `Err(JournalError::DuplicateStagedKey { .. })` → falls through `else` → `panic!` (sibling variant; same payload fields but different variant; the cross-batch scenario MUST NOT trigger it).
- `Err(JournalError::DuplicateEvent { run, seq })` → binds `run: RunId, seq: EventSeq`, then `assert_eq!` checks both.

A regression that returns ANY error variant other than `DuplicateEvent` fails. A regression that returns `DuplicateEvent` with the wrong tuple fails. A regression that returns `Ok(())` fails.

### Panic location

The `panic!("expected DuplicateEvent, got {:?}", result)` is the **only** `panic!` permitted in the test-fix. Per the master contract (`velvet-ballistics-MASTER.md`), `panic` is forbidden in production Rust; test code is allowed to `panic!` for failure signaling, and the canonical reference uses it at exactly this location.

## Contract Type 3: `prop_assert!` Macro Surface (for proptest lanes)

All 5 weak tests run inside `proptest! { ... }` blocks; the assertion macro must be `prop_assert!` (not `assert!`) so proptest reports shrunk counterexamples on failure.

### Required macro per assertion type

| Assertion target | Required macro |
|------------------|----------------|
| Boolean (`matches!(...) if guard`) | `prop_assert!(...)` |
| Equality (`a == b`) | `prop_assert_eq!(a, b)` |
| `Result::is_ok()` / `is_err()` | `prop_assert!(result.is_ok())` / `prop_assert!(result.is_err())` (preserved as-is) |
| `len()` numeric check | `prop_assert_eq!(len, expected)` (preserved as-is in PS_004) |
| `is_aborted()` boolean | `prop_assert!(batch.is_aborted())` (preserved as-is in PS_004) |

The weak occurrences use `prop_assert!(is_dup)` where `is_dup: bool`; the replacement MUST be the field-bound `matches!` guard directly inside `prop_assert!`. The intermediate `let is_dup = ...;` binding is removed.

## Contract Type 4: Proptest Input Space (preserved)

The proptest strategy `run in 1u64..1000u64, seq in 0u64..100u64` MUST be preserved verbatim. Changing the input space is a scope expansion; only the assertion body changes.

### Why the bounded space matters

- `1u64..1000u64` excludes `0` to avoid colliding with sentinel values and to keep the test orthogonal to the `RunId::ZERO` constant.
- `0u64..100u64` includes `0` because `EventSeq::ZERO` is a legal initial sequence.
- Both ranges are well below `u64::MAX`, so no overflow risk in `make_event` (which only constructs a `JournalEvent::RunAccepted`, no arithmetic).

### Required import set (already present)

```rust
use proptest::prelude::*;
use vb_core::{RunId, WorkflowDigest};
use vb_storage::EventSeq;
use vb_storage::batch::JournalWriteBatch;
use vb_storage::error::JournalError;
use vb_storage::events::JournalEvent;
use vb_storage::journal::FjallJournal;
```

No new imports are required for the test fix. `RunId::new`, `RunId::get` (unused, equality suffices), `EventSeq::new` are already accessible via `vb_core::RunId` and `vb_storage::EventSeq`.

## Contract Type 5: Forbidden Test-Only Patterns

After this bead lands, the following patterns are FORBIDDEN inside the 5 target proptest functions:

| Pattern | Reason forbidden |
|---------|------------------|
| `matches!(result, Err(JournalError::DuplicateEvent { .. }))` | Wildcard discards `run`/`seq`. |
| `matches!(append_result, Err(JournalError::DuplicateEvent { .. }))` (PS_004 variant) | Same wildcard issue. |
| `matches!(dup_result, Err(JournalError::DuplicateEvent { .. }))` (other files) | Same. |
| `prop_assert!(matches!(result, Err(_)))` | Variant-blind; accepts any error including `BatchAborted`. |
| `assert_ne!(result, Ok(()))` | Equivalent to `is_err()`; does not check the error variant or payload. |
| `assert!(result.is_err())` | Variant-blind; accepts any `Err` variant. |

The replacement MUST use the field-bound guard pattern (`Contract Type 1`).

## Type-System Boundary

### Test-side types

- `result: Result<(), JournalError>` — return type of `JournalWriteBatch::append_event(&mut self, event: &JournalEvent) -> Result<(), JournalError>`.
- `RunId`, `EventSeq` — newtype wrappers, used as typed equality targets.
- `JournalEvent::RunAccepted { run, seq, workflow }` — the only event variant exercised.

### Production-side types (referenced, not modified)

- `JournalWriteBatch<'j>` — `crates/vb_storage/src/batch/types.rs`.
- `JournalError` enum — `crates/vb_storage/src/error/mod.rs:21-...` (16+ variants; the test must distinguish `DuplicateEvent` from `DuplicateStagedKey`).
- `FjallJournal` — `crates/vb_storage/src/journal/`.

### Type-level hazard: same payload, different variant

`DuplicateEvent { run: RunId, seq: EventSeq }` and `DuplicateStagedKey { run: RunId, seq: EventSeq }` have **identical payload fields**. A regression that returns `DuplicateStagedKey` instead of `DuplicateEvent` in the cross-batch scenario must NOT pass. The `let-else` exhaustive pattern handles this because `DuplicateStagedKey { .. }` does not match the `DuplicateEvent { run, seq }` arm.

The field-bound `matches!` guard inside `prop_assert!` ALSO handles this because `matches!` does not match a different variant even with identical fields.

## Pre/Post-Conditions (Test Surface)

### Pre-condition (per test execution)

- `b1 = JournalWriteBatch::new(&journal)` — fresh batch.
- `event = JournalEvent::RunAccepted { run: RunId::new(run), seq: EventSeq::new(seq), workflow: WorkflowDigest::from_bytes([0u8; 32]) }` — fixed event.
- `b1.append_event(&event)` MUST return `Ok(())`.
- `b1.commit()` MUST return `Ok(())`.

### Post-condition (per test execution)

- `b2 = JournalWriteBatch::new(&journal)` — fresh batch observing same journal.
- `b2.append_event(&event)` MUST return `Err(JournalError::DuplicateEvent { run: RunId::new(run), seq: EventSeq::new(seq) })`.
- For PS_004 only: additionally `b2.is_aborted() == true` and `b2.commit() == Err(BatchAborted)` and `journal.events_for_run(RunId::new(run)).len() == 1`.

## Type-Contract Summary

| # | Contract | Required | Forbidden |
|---|----------|----------|-----------|
| 1 | Strong field-bound `matches!` guard | `prop_assert!(matches!(result, Err(JournalError::DuplicateEvent { run: r, seq: s }) if r == RunId::new(run) && s == EventSeq::new(seq)))` | `matches!(_, DuplicateEvent { .. })` |
| 2 | `let-else` exhaustiveness (canonical pattern) | `let Err(JournalError::DuplicateEvent { run, seq }) = result else { panic!(...) }; assert_eq!(run, RunId::new(E)); assert_eq!(seq, EventSeq::new(E));` | `let Err(JournalError::DuplicateEvent { .. }) = result else { ... };` |
| 3 | Proptest macro discipline | `prop_assert!(...)` and `prop_assert_eq!(...)` | Bare `assert!` inside `proptest!` blocks |
| 4 | Input space preservation | `run in 1u64..1000u64, seq in 0u64..100u64` | Any change to proptest strategy |
| 5 | Forbidden patterns | (none — all required patterns are constructive) | Wildcard `..` in duplicate-event match arm; variant-blind `is_err()`; wrong-variant tolerance |