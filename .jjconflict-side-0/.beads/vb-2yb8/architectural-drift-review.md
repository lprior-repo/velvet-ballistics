# Architectural Drift Review — vb-2yb8

## Review Date: 2026-05-09
## Reviewer: GoMasterOrchestrator

## File Size Audit

| File | Lines | Hot/Cold | Status |
|------|-------|----------|--------|
| `crates/vb_runtime/src/durability_matrix.rs` | 359 | Cold (verification) | OK — includes tests |
| `crates/vb_runtime/tests/durability_matrix_integration.rs` | 511 | Test | OK |
| Production portion of durability_matrix.rs | ~280 | Cold | OK |

## DDD Principles

### Bounded Context
- `durability_matrix` lives in `vb_runtime` which is the correct bounded context
- It references `vb_storage::RecordKind` via cross-crate dependency (acceptable)

### Value Objects
- `DurabilityRow` is immutable, no identity — correct value object ✓
- `StoragePartition` and `AckPoint` are enums with no behavior — correct value objects ✓

### Domain Language
- Uses canonical primitive names from MASTER.md ✓
- Uses `RecordKind` from storage domain ✓
- Uses `CompiledNodeKind` names from core domain ✓

## Scott Wlaschin DDD Check

1. **Make illegal states unrepresentable:**
   - `AckPoint::BeforeJournalAppend` exists but gate test rejects it ✓
   - Could be stronger: use type system to only allow `AfterJournalAppend` at compile time
   - **Recommendation:** Remove `BeforeJournalAppend` variant entirely; it's a forbidden state

2. **Parse, don't validate:**
   - The matrix is declarative (parsed/constructed statically) rather than validated dynamically ✓
   - `verify_matrix` is validation, not parsing — acceptable for this use case

3. **Types as documentation:**
   - `RecordKind`, `StoragePartition`, `AckPoint` are self-documenting ✓

## Existing Pattern Compliance

- Module follows `vb_runtime` conventions (forbid unsafe, deny unused_must_use) ✓
- Test file follows integration test conventions (separate `tests/` directory) ✓
- Uses `VolatileRuntimeJournal` like existing tests ✓
- Uses `Shard::new_with_journal` like existing tests ✓

## Refactoring Opportunities

1. **Remove `BeforeJournalAppend`:** This variant represents an illegal state. Since all handlers must ack after persist, the variant serves no purpose except in gate tests. Consider removing it and the associated verifier.

2. **Extract test helpers:** The integration tests duplicate workflow fixtures from `src/shard/tests.rs`. Consider sharing fixtures via a test-util module.

3. **Add meta-rows:** `ErrorHandler` and `RetryCheck` are runtime constructs that emit journal events. They should have matrix rows.

## Decision

No code changes required for landing. Minor refactoring opportunities documented for follow-up beads.

STATUS: APPROVED
