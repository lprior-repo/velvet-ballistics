# Architectural Drift Report: `vb_storage/src/codec/tests.rs`

## File Summary

| Metric | Value |
|--------|-------|
| **File Path** | `crates/vb_storage/src/codec/tests.rs` |
| **Total Lines** | 2557 |
| **Test Count** | 96 |
| **Location** | `crates/vb_storage/src/codec/` |
| **Status** | **REFACTOR REQUIRED** |

## Size Analysis

| Threshold | Actual | Ratio |
|-----------|--------|-------|
| 300 lines (max) | 2557 lines | 8.5x over limit |

## Findings

### 1. Line Count Violation
- **Issue**: File is **2557 lines** — massively exceeds the 300-line threshold
- **Severity**: CRITICAL
- **Category**: `tests.rs` files should be compact; this one is 8.5x the limit

### 2. Test Density
- **Tests**: 96 tests across 2557 lines (~26.6 lines/test)
- **Observation**: Many tests follow an identical encode/decode roundtrip pattern
- **Opportunity**: Tests can be deduplicated via parameterized test helpers

### 3. Architectural Drift
- **Domain**: Binary codec tests for `vb_storage` journal/wal encoding
- **Cohesion**: Single file tests multiple record types (JournalEvent, WorkflowSource, CompiledIr, Blob, RunHeader, Snapshot)
- **Violation**: High coupling — all codec behavior in one test module
- **DDD Concern**: Primitive obsession not present; NewTypes (RunId, SlotIdx, StepIdx, EventSeq, etc.) used correctly

## Recommendations

### Immediate Actions (Required)

1. **Split by Record Kind Family** — Create sub-modules:
   - `tests/journal_events.rs` — tests for JournalEvent roundtrips (RunAccepted, StepStarted, SlotWritten, etc.)
   - `tests/workflow_records.rs` — tests for WorkflowSource, CompiledIr records
   - `tests/blob_record.rs` — tests for BlobRecord
   - `tests/snapshot_record.rs` — tests for RunSnapshot
   - `tests/header_tests.rs` — tests for header encode/decode
   - `tests/error_path.rs` — consolidated error/rejection tests

2. **Target Sizes After Split**:
   - `journal_events.rs`: ~500-600 lines (25-30 tests)
   - `workflow_records.rs`: ~300-350 lines (15-20 tests)
   - `blob_record.rs`: ~200-250 lines (8-10 tests)
   - `snapshot_record.rs`: ~200-250 lines (8-10 tests)
   - `header_tests.rs`: ~300-350 lines (15-20 tests)
   - `error_path.rs`: ~400-450 lines (15-20 tests)

3. **Update `mod.rs`** — Export new test modules

### Test Pattern Harmonization
Many roundtrip tests follow this exact pattern:
```rust
#[test]
fn encode_decode_roundtrip_<variant>() -> Result<(), JournalError> {
    let event = JournalEvent::<variant> { ... };
    let bytes = encode_record(...)?;
    let (_, decoded) = decode_record::<JournalEvent>(...)?;
    assert_eq!(decoded, event);
    Ok(())
}
```
Consider extracting a shared helper:
```rust
fn roundtrip_event<E: Encode + Decode + Eq>(event: &E, magic: u32, kind: RecordKind, max: u32) -> Result<(), JournalError>
```

## Status

```
STATUS: REFACTOR REQUIRED
```

The file MUST be split into sub-modules. Current size (2557 lines) is architecturally unacceptable per the <300 line rule.
