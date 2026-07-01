# Domain Model — vb-pg2wq

**Bead:** vb-pg2wq — Tests: make duplicate-event test assert one exact contract (P1 bug)
**Lane:** Rust-local + test-only assertion repair
**Source-of-truth:** `crates/vb_storage/src/batch/append_event.rs:42-67`
**Canonical strong pattern:** `crates/vb_storage/src/tests.rs:1344-1367`

## Ubiquitous Language

| Term | Definition | Source |
|------|------------|--------|
| **Journal event** | A typed record written durably to the Fjall journal; in this bead, only `JournalEvent::RunAccepted { run, seq, workflow }` is exercised. | `crates/vb_storage/src/events.rs` |
| **`RunId`** | Opaque newtype over `u64` (`vb_core::ids`) identifying a workflow run. Smart constructor: `RunId::new(value)`; accessor: `RunId::get()` returns `u64`. | `crates/vb_core/src/ids/mod.rs:80` |
| **`EventSeq`** | Opaque monotonic newtype over `u64` (`vb_storage::types`) identifying the per-run event sequence. Smart constructor: `EventSeq::new(value)`; accessor: `EventSeq::get()` returns `u64`. | `crates/vb_storage/src/types.rs:73-94` |
| **`JournalWriteBatch`** | Mutable batch wrapper around an `OwnedWriteBatch`; observes an `&FjallJournal` for duplicate detection across already-committed events. | `crates/vb_storage/src/batch/types.rs` |
| **`JournalError::DuplicateEvent { run, seq }`** | Typed error returned when a cross-batch duplicate is detected against the durable keyspace; the `run`/`seq` are the exact `(event.run_id(), event.seq())` of the rejected event. | `crates/vb_storage/src/error/mod.rs:30-31` |
| **`JournalError::DuplicateStagedKey { run, seq }`** | Sibling variant for in-batch (not-yet-committed) duplicates. Same payload fields, different variant. **Not** in scope for this bead (none of the 5 weak tests exercise same-batch duplicate). | `crates/vb_storage/src/error/mod.rs:32-33` |
| **Weak assertion** | `matches!(result, Err(JournalError::DuplicateEvent { .. }))` followed by `prop_assert!(is_dup)`. Accepts ANY `(run, seq)` tuple; does not pin the typed contract. | 6 occurrences in `crates/vb_storage/tests/proptest_vb_vzcuf_PS_001/003/004/008/009.rs` |
| **Strong assertion (canonical)** | `let Err(JournalError::DuplicateEvent { run, seq }) = result else { panic!("expected DuplicateEvent, got {:?}", result) }; assert_eq!(run, RunId::new(EXPECTED)); assert_eq!(seq, EventSeq::new(EXPECTED));`. Pins BOTH variant AND payload fields. | `crates/vb_storage/src/tests.rs:1362-1366` |
| **Cross-batch duplicate** | Event `e` is committed in batch `b1`; the same `e` is later passed to `b2.append_event(&e)`. Production code path: `append_event.rs:61-67` (the durable `contains_key` branch). | All 5 weak tests |
| **Same-batch duplicate** | Two `append_event` calls inside one batch with identical `(run, seq)`. Production code path: `append_event.rs:55-60` (the `staged_event_keys` HashSet branch). Returns `DuplicateStagedKey`, NOT `DuplicateEvent`. | Out of scope for this bead |
| **Proptest input space** | `run ∈ 1u64..1000u64`, `seq ∈ 0u64..100u64` (where applicable). Tightly bounded; safe under `checked_add`. | Each proptest signature |

## Bounded Context

The domain of this bead is the **proptest assertion contract** for the cross-batch `DuplicateEvent` branch of `JournalWriteBatch::append_event`. The production code already implements the exact `(run, seq)` return tuple (lines 42-67 of `append_event.rs`); this bead only hardens the test surface to reflect that production contract.

**In scope:**

- 6 weak-assertion occurrences in `crates/vb_storage/tests/` (proptest lanes only)
- 5 proptest functions: `ps001_duplicate_rejected`, `ps003_dup_fields`, `ps004_no_persist`, `ps004_empty_commit_after_rej`, `ps008_dup_before_queue`, `ps009_dup_rejected`
- 4 source files: `proptest_vb_vzcuf_PS_001.rs`, `proptest_vb_vzcuf_PS_003.rs`, `proptest_vb_vzcuf_PS_004.rs`, `proptest_vb_vzcuf_PS_008.rs`, `proptest_vb_vzcuf_PS_009.rs`

**Out of scope (adjacent findings, follow-up candidates):**

- `crates/vb_storage/src/batch/t_append_event.rs` (`batch_append_event_rejects_duplicate_event`)
- `crates/vb_storage/src/batch/t_byte_accounting_part{2,3,4}.rs` (6 weak matches across 4 files)
- `crates/workspace_tests/tests/journal_side_index_contracts.rs` (borderline — has additional `is_aborted`/`len` assertions that narrow the contract)
- `crates/vb_storage/src/tests.rs:837-851` (`duplicate_event_append_is_rejected`)
- All `verification/**`, `fuzz/**`, `benches/**`, `xtask/**`, production source, `Cargo.toml`

## Value Objects (already provided by upstream)

| Value Object | Constructor | Accessor | Equality |
|--------------|-------------|----------|----------|
| `RunId` | `RunId::new(value: u64) -> RunId` (const) | `.get() -> u64` (const) | `PartialEq`, `Eq`, `Ord` |
| `EventSeq` | `EventSeq::new(value: u64) -> EventSeq` (const) | `.get() -> u64` (const) | `PartialEq`, `Eq`, `Ord` |

Both are `#[repr(transparent)] newtype(u64)` wrappers, so `RunId::new(run) == RunId::new(run)` is structurally identical to the proptest input `run` rebagged. The strong-assertion pattern `assert_eq!(run, RunId::new(EXPECTED))` is a typed equality check; a regression that returns `DuplicateEvent { run: RunId::new(0), seq: EventSeq::new(0) }` (or any other tuple) will fail this assertion.

## Aggregate (test-side)

The **proptest cross-batch duplicate scenario** is a small two-batch aggregate:

```
make_event(run, seq) ──> b1.append_event(&event)  ──> b1.commit() ──> b2.append_event(&event) ──> Err(DuplicateEvent { run, seq })
```

The contract asserts that the `Err` carries the exact `(run, seq)` of the input event. No additional state is mutated (no inner batch insert; `b2.aborted == true` is a secondary invariant in PS_004).

## Forbidden States (Test-Side)

| State | Why forbidden |
|-------|---------------|
| `Err(DuplicateEvent { run: RunId::new(0), seq: EventSeq::new(0) })` when proptest input was `run=42, seq=7` | A regression that hardcodes or synthesizes the wrong tuple must NOT pass. The whole point of this bead. |
| `Err(DuplicateEvent { run: RunId::new(WRONG_RUN), seq: EventSeq::new(EXPECTED_SEQ) })` | Field mutation must be detected. |
| `Err(DuplicateStagedKey { .. })` in the cross-batch scenario | Variant confusion between same-batch (`DuplicateStagedKey`) and cross-batch (`DuplicateEvent`) must NOT be masked by `..` wildcard. |
| `Ok(())` from `b2.append_event(&event)` after `b1.commit()` | Silent overwrite regression; previously a hard panic for the cross-batch guard. |
| `Err(QueueFull)` / `Err(BatchAborted)` / `Err(KeyCapacity)` etc. | Sibling-variant regressions; previously masked by `..`. |

## Test Contract Surface

Each of the 6 weak occurrences must be rewritten to mirror the canonical pattern from `vb_storage/src/tests.rs:1344-1367`. The exact pattern is encoded in `contract.md` and the per-file test-fix plan is described in the next-handoff section of `contract.md`.

## Invariants (Test-Side)

1. **Exact tuple binding:** Every proptest cross-batch duplicate test must assert `result` equals `Err(DuplicateEvent { run: RunId::new(run), seq: EventSeq::new(seq) })` for the proptest inputs `run`/`seq`.
2. **Variant binding:** The test must distinguish `DuplicateEvent` from `DuplicateStagedKey` even though both carry identical payload fields; the `let-else` pattern binds the variant by exhaustiveness.
3. **No wildcards:** `..` is forbidden in the duplicate-event match arm. The pattern must name `run` and `seq` to enable field assertion.
4. **No `.unwrap()` / `.expect()` for negative-path results:** `result` is `Err(...)` in the duplicate path; `panic!` in the `let-else` branch is the only allowed panic (and is required to fail loud on regression). The two `.expect("first")` / `.expect("commit")` calls in the setup are positive-path setup; they are pre-existing and remain.

## Open Domain Questions

None. The contract surface is fully determined by:

1. The production code at `crates/vb_storage/src/batch/append_event.rs:42-67` (lines 61-67 are the `DuplicateEvent` branch).
2. The variant declaration at `crates/vb_storage/src/error/mod.rs:30-31`.
3. The canonical strong pattern at `crates/vb_storage/src/tests.rs:1344-1367`.
4. The proptest input space `run ∈ 1u64..1000u64, seq ∈ 0u64..100u64` already present in each test signature.

No domain language decision is outstanding.