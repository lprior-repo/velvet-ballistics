# Hazard Analysis — vb-pg2wq

**Bead:** vb-pg2wq — Tests: make duplicate-event test assert one exact contract (P1 bug)
**Lane:** Rust-local + test-only assertion repair

## Hazard Index

| # | Hazard Class | Severity (this bead) | Mitigation |
|---|--------------|----------------------|------------|
| H1 | Audit-regression-resistance (THE bead) | **CRITICAL — the whole point** | Strong field-bound `matches!` guard; mirrors canonical pattern at `tests.rs:1344-1367`. |
| H2 | Variant confusion (`DuplicateEvent` vs `DuplicateStagedKey`) | High | `let-else` exhaustive pattern; `matches!` guard binds the variant name. |
| H3 | Test-quality (class-of-bug across PS_00x series) | Medium-High | Fix applied to ALL 6 occurrences in one bead to prevent regression. |
| H4 | Production binding drift | Low | No production change; test fix strengthens runtime↔proof alignment. |
| H5 | Forbidden-construct introduction | Low | Test fix uses `prop_assert!`/`prop_assert_eq!`/`panic!` only; no `unwrap`/`expect` on `result`. |
| H6 | Temporal hazard | None | Single-threaded proptest; no async. |
| H7 | Concurrency hazard | None | Sequential `&mut self` operations; no shared state. |
| H8 | Unsafe/provenance hazard | None | `forbid(unsafe_code)`; no raw pointers. |
| H9 | Parser/codec hazard | None | `make_event` is fixed; no parser change. |
| H10 | Hostile input (fuzz/proptest) | Mitigated | Proptest is the hostile-input lane; strategy preserved verbatim. |
| H11 | Performance hazard | None | Test fix is microsecond-scale. |
| H12 | Public-API hazard | None | Uses already-public batch API; no new exports. |
| H13 | Dependency hazard | None | No `Cargo.toml` change. |
| H14 | Migration hazard | None | No schema change. |
| H15 | Release/API hazard | None | Test-only change. |
| H16 | Storage/persistence hazard | Low | `tempfile::tempdir()`; existing fsync at commit. |
| H17 | Bounded-state hazard | None | Proptest input space `1..1000 × 0..100` is well below `u64::MAX`. |

---

## H1: Audit-Regression-Resistance (THE HAZARD)

**Source:** Bead description verbatim:
> "A test for duplicate-event handling has fuzzy or weak assertions (e.g., assert_ne!, asserts on len only). Replace with an exact contract assertion that pins the exact behavior on duplicate detection."

**Current state (broken):**

```rust
let is_dup = matches!(result, Err(JournalError::DuplicateEvent { .. }));
prop_assert!(is_dup);
```

The `..` wildcard accepts ANY `(run, seq)` tuple. A regression that returns `DuplicateEvent { run: RunId::new(0), seq: EventSeq::new(0) }` would pass. A regression that returns `DuplicateEvent { run: <sentinel>, seq: <sentinel> }` would pass. Only a regression that changes the **variant** would fail.

**Required state (fixed):**

```rust
prop_assert!(matches!(
    result,
    Err(JournalError::DuplicateEvent { run: r, seq: s })
        if r == RunId::new(run) && s == EventSeq::new(seq)
));
```

The guard binds `r: RunId` and `s: EventSeq` and asserts typed equality against the proptest inputs. Any field mutation fails.

**Severity:** CRITICAL. This is the entire reason for the bead.

**Probability (without fix):** HIGH. The weak pattern is endemic — 6 occurrences in 5 functions across 4 files, plus 6 more in adjacent `src/batch/t_*` files (out of scope but the same pattern).

**Residual risk after fix:** LOW. Field-bound guard is type-checked at compile time; proptest strategy varies `run`/`seq` across the full input space.

---

## H2: Variant Confusion (`DuplicateEvent` vs `DuplicateStagedKey`)

**Source:** `crates/vb_storage/src/error/mod.rs:30-33`.

Both variants carry IDENTICAL payload fields (`run: RunId, seq: EventSeq`). The variants differ ONLY in their semantic meaning:
- `DuplicateEvent`: cross-batch duplicate (durable keyspace).
- `DuplicateStagedKey`: same-batch duplicate (in-memory `staged_event_keys` set).

**Regression scenario:** A production code refactor that moves the early-return boundary from line 61-67 (`journal.events.contains_key`) to line 55-60 (`staged_event_keys.contains`) would cause the cross-batch test scenario to return `DuplicateStagedKey` instead of `DuplicateEvent`.

**Mitigation:** The strong pattern binds the **variant name** explicitly:
```rust
let Err(JournalError::DuplicateEvent { run, seq }) = result else { panic!(...); };
```
or
```rust
matches!(result, Err(JournalError::DuplicateEvent { run: r, seq: s }) if ...)
```

A `DuplicateStagedKey` value does NOT match the `DuplicateEvent` arm. The strong pattern catches this regression; the weak `..` pattern does NOT (since the wildcards would still match the same payload shape if the variant were swapped, and the `let is_dup = matches!(result, Err(JournalError::DuplicateEvent { .. }))` would fail at the `matches!` step because the variant name is different — actually the weak pattern DOES catch variant confusion because `matches!` checks the variant name. So H2 is mitigated by BOTH weak and strong patterns; this is not the discriminant of the bead. The discriminant is **field mutation**, which is H1.).

**Severity (re-classified):** LOW for H2 specifically (the weak pattern already catches variant confusion). The weak pattern's blind spot is field mutation (H1).

---

## H3: Test-Quality Class-of-Bug

**Source:** Bead finding: the weak pattern is endemic across the PS_00x proptest series.

**Scope:** 6 occurrences in 5 functions across 4 files. All 6 are fixed in this bead to prevent:
- The same pattern recurring in future test additions.
- One occurrence being "fixed" while siblings remain weak (audit drift).
- Adjacent `src/batch/t_*` files continuing the pattern (out of scope but flagged for follow-up).

**Mitigation:** All 6 occurrences are fixed in the same bead; the canonical reference at `tests.rs:1344-1367` is the model; the next-handoff section of `contract.md` enumerates per-file changes.

**Residual risk:** Adjacent `src/batch/t_*` files (7 weak occurrences across 5 files) are out of scope. Follow-up bead candidates flagged in `codebase-map.md`.

---

## H4: Production Binding Drift

**Source:** No production change is in scope. The test fix is on the consumption side.

**Risk:** A future production change that alters the `DuplicateEvent` payload contract (e.g., adds a third field) would cause the strong pattern to fail at compile time. This is **desirable**: compile-time failure prevents silent drift.

**Mitigation:** The strong pattern is type-driven; if production adds a field to `DuplicateEvent`, the `matches!` pattern will require an explicit binding or wildcard for the new field. The test code will need a coordinated update, which is the right failure mode.

**Residual risk:** LOW. The Kani harness `kani_vb_vzcuf_ps004.rs:48-59` already models `DuplicateEvent { run: r, seq: s }`; if production changes the payload, the Kani harness will also need an update, providing a second check.

---

## H5: Forbidden-Construct Introduction

**Source:** Master contract forbids `unsafe`, `unwrap`, `expect`, `panic` (production only), `todo`, `unimplemented`, `dbg`, unchecked indexing/slicing/casts/arithmetic, runtime YAML/JSON/HTTP.

**Test-side allowance:** `panic!` is allowed in test code for failure signaling. `expect` is used in setup (positive-path). `unwrap` is not used in the new assertions.

**Audit of new code:**

| Construct | Used? | Compliant? |
|-----------|-------|------------|
| `unsafe` | No | ✅ |
| `unwrap()` | No | ✅ |
| `expect(...)` | No (in the new assertion body) | ✅ — pre-existing `.expect("first")` / `.expect("commit")` in setup are preserved (positive-path setup, allowed) |
| `panic!(...)` | No (in proptest lanes); Yes (in canonical reference at `tests.rs:1363`, not modified) | ✅ — `panic!` is allowed in test code per master contract |
| `todo!()` / `unimplemented!()` | No | ✅ |
| `dbg!()` | No | ✅ |
| Unchecked indexing | No | ✅ |
| Unchecked casts | No | ✅ — `r == RunId::new(run)` is a typed `RunId == RunId` comparison, not a `u64 as RunId` cast |
| Unchecked arithmetic | No | ✅ — `make_event` only constructs |
| Runtime YAML/JSON/HTTP | No | ✅ |

**Severity:** LOW. Audit clean.

---

## H6: Temporal Hazard

**Source:** None. All 5 proptest functions are synchronous, single-threaded, no `tokio`, no wall-clock dependence.

**Severity:** NONE.

---

## H7: Concurrency Hazard

**Source:** `JournalWriteBatch::append_event` takes `&mut self`; the 5 tests are sequential; no shared state across proptest iterations.

**Severity:** NONE.

---

## H8: Unsafe/Provenance Hazard

**Source:** `forbid(unsafe_code)` in all relevant crates. No raw pointers in test code.

**Severity:** NONE.

---

## H9: Parser/Codec Hazard

**Source:** `encode_record` is used by other tests in the same files but NOT by the duplicate-event tests (which only call `append_event` and `commit`).

**Severity:** NONE for this bead.

---

## H10: Hostile Input (Fuzz/Proptest)

**Source:** Proptest is the hostile-input lane for this bead.

**Strategy:** `run in 1u64..1000u64`, `seq in 0u64..100u64` (where applicable). The strategy is preserved verbatim — only the assertion body changes.

**Coverage:** 1000 × 100 = 100,000 inputs per proptest function (with shrinking). The strong assertion must hold for ALL of them.

**Regression scenario:** A production change that returns `DuplicateEvent` correctly for `run=1, seq=0` but incorrectly for `run=999, seq=99` (e.g., off-by-one in some key construction) would be caught by the strong assertion.

**Severity:** LOW (the fix is the mitigation). Without the fix, hostile input could expose the wrong tuple silently.

---

## H11: Performance Hazard

**Source:** Test fix is microsecond-scale (`matches!` guard with two `PartialEq` checks on newtype `u64` wrappers).

**Severity:** NONE.

---

## H12: Public-API Hazard

**Source:** No new exports. The strong pattern uses `RunId::new`, `EventSeq::new`, `JournalError::DuplicateEvent`, `JournalWriteBatch::append_event`, `JournalWriteBatch::commit`, `JournalWriteBatch::is_aborted`, `JournalWriteBatch::len`, `JournalWriteBatch::is_empty`, `FjallJournal::events_for_run`, `prop_assert!`, `prop_assert_eq!`, `matches!`. All are pre-existing public APIs or macros.

**Severity:** NONE.

---

## H13: Dependency Hazard

**Source:** No `Cargo.toml` change.

**Severity:** NONE.

---

## H14: Migration Hazard

**Source:** No schema change. `JournalEvent::RunAccepted` payload unchanged. `JournalError::DuplicateEvent` payload unchanged.

**Severity:** NONE.

---

## H15: Release/API Hazard

**Source:** Test-only change. No release artifact affected.

**Severity:** NONE.

---

## H16: Storage/Persistence Hazard

**Source:** `tempfile::tempdir()` for ephemerality; `FjallJournal` fsyncs at `commit()`. No external storage.

**Severity:** LOW. `tempdir()` may fail under extreme disk pressure; `.expect(...)` in setup would panic. Pre-existing behavior; preserved.

---

## H17: Bounded-State Hazard

**Source:** Proptest input space `1u64..1000u64 × 0u64..100u64` is well below `u64::MAX`. No overflow risk in `make_event`.

**Severity:** NONE.

---

## Aggregate Hazard Summary

| Hazard class | Net severity (this bead) |
|--------------|-------------------------|
| Audit-regression-resistance | **RESOLVED by this bead** (H1) |
| Variant confusion | **Pre-mitigated** (H2 — caught by both weak and strong patterns) |
| Test-quality class-of-bug | **RESOLVED by this bead** (H3) |
| Production binding drift | **Strengthened** (H4 — type-driven failure on production change) |
| Forbidden-construct introduction | **None introduced** (H5) |
| All other classes | **None** |

**Residual risk after bead lands:** LOW. The dominant residual is the adjacent `src/batch/t_*` files (out of scope) and the `journal_side_index_contracts.rs:495-531` borderline case (already partially mitigated by `is_aborted`/`len` assertions).

---

## Open Hazard Questions

None. The hazard profile is fully determined by the production code at `crates/vb_storage/src/batch/append_event.rs:42-67`, the variant declaration at `crates/vb_storage/src/error/mod.rs:30-33`, the canonical reference at `crates/vb_storage/src/tests.rs:1344-1367`, and the proptest strategies in the 4 target files.