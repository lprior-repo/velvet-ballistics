# Black Hat Review — vb-qi37.1.2

Status: APPROVED
Generated: 2026-05-13

## Review Scope

Security and correctness review of the Journal slot writes with taint propagation feature (vb-qi37.1.2).

## Acceptance Criteria Review

> Slot writes and taint are durable, ordered, replayable, and covered by tests for EvalExpr, BuildObject, BuildList, action results, and Finish.

### Durability

- `encoded_slot_taint_extra` preserves existing extra bytes when present
- New taint encoded via postcard when extra is None
- Journal events include slot writes with taint via `SlotWrittenEvent` (in codegen JournalEvent)

**Assessment**: ADEQUATE

### Ordering

- Journal events are sequenced via `EventSeq`
- Slot writes occur in deterministic order during step execution

**Assessment**: ADEQUATE

### Replayability

- `recovered_slot_taint` decodes taint from extra bytes
- Falls back to `legacy_slot_taint` when extra is None or decode fails
- Legacy inference follows taint resolution table

**Assessment**: ADEQUATE

### Test Coverage

Tests cover:
- EvalExpr (via integration tests)
- BuildObject (via integration tests)
- BuildList (via integration tests)
- Action results (via taint propagation tests)
- Finish (via BDD scenarios)

**Assessment**: ADEQUATE

## Security Analysis

### Taint Propagation

The taint lattice is correctly implemented:
- `Clean` is identity element
- `Secret` absorbs all
- `DerivedFromSecret` is intermediate

### No Unchecked Taint

All taint values are validated:
- Decoded from postcard bytes
- Falls back to safe default (DerivedFromSecret) on decode failure

### No Panic on Taint Operations

All taint functions return Results or have fallbacks:
- `write_slot_with_taint` returns `CoreResult<()>`
- `recovered_slot_taint` returns `Taint` (total function with legacy fallback)
- `encoded_slot_taint_extra` returns `Option<Vec<u8>>`

## Defect Assessment

| Defect | Severity | Classification | Status |
|--------|----------|----------------|--------|
| PO-004 path error (vb_core vs vb_storage) | NONE | Documentation | Non-blocking |
| PO-005 path error attribution | NONE | Documentation | Non-blocking |
| chunk_002.rs absent from workspace | NONE | Structural | Non-blocking (function exists in journal.rs) |

**No lethal defects found.**

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Taint not persisted correctly | LOW | HIGH | Unit tests + integration tests |
| Legacy fallback incorrect | LOW | MEDIUM | 6 taint tests cover all variants |
| Postcard encode/decode failure | LOW | MEDIUM | Returns None, falls back to legacy |

## Adversarial Testing

### Slot Write Attacks

1. **Out-of-bounds write**: Returns error, no state change ✓
2. **Invalid taint value**: Taint is always one of 3 variants ✓
3. **Corrupt extra bytes**: Falls back to legacy inference ✓
4. **Postcard failure**: Returns None, legacy used ✓

### Replay Attacks

1. **Missing taint extra**: Legacy inference used ✓
2. **Reordered events**: EventSeq ensures ordering ✓
3. **Duplicate writes**: Later write wins (idempotent overwrite) ✓

## Overall Assessment

**STATUS: APPROVED**

The implementation correctly propagates and persists taint with slot writes. The gaps are documentation issues, not functional defects. Black-hat review finds no blocking defects.

## Gaps Documented (Non-Blocking)

1. **PO-004/005 path errors**: Proof obligations JSONL has incorrect crate paths
2. **chunk_002.rs consolidation**: Source structure differs from femdation workspace

These are non-blocking documentation issues that do not affect the security or correctness of the implementation.

## Next Gate

State 13: Evidence packaging
