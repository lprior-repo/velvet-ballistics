# Proof Writer Report — vb-mfks

## Status Summary
**State**: 5 (Proof Execution)
**Lanes Executed**: Kani (primary)
**Obligations Touched**: 17

## Files Modified

### vb_runtime/kani_trace_ring.rs
| Harness | Violation | Fix |
|---------|-----------|-----|
| `arbitrary_trace_event` | Hardcoded StepIdx::new(0), SlotIdx::new(0) | Replaced with kani::any() |
| `verify_drain_for_run_correctness` | Fixed 4-event hardcoded array | Replaced with kani::any() for event contents; explicit run_ids preserved for test correctness |

### vb_runtime/kani_admission_store.rs
| Harness | Violation | Fix |
|---------|-----------|-----|
| `storage_artifact_store_send` | Vacuous kani::assert(true) | Removed; Send bound is compile-time enforced |
| `storage_artifact_store_sync` | Vacuous kani::assert(true) | Removed; Sync bound is compile-time enforced |

### vb_validate/kani_gate_08_structural.rs
| Harness | Violation | Fix |
|---------|-----------|-----|
| H9 `kani_gate_08_empty_nodes_valid_accessors_pass` | Hardcoded WorkflowParts | kani::any() + assume(nodes.is_empty()) + validity assumptions |
| H10 `kani_gate_08_expressions_with_accessor_refs` | Hardcoded ExprIdx, AccessorIdx, SlotIdx, SymbolId | kani::any() for all ID types + bounded validity assumptions |
| H11 `kani_gate_08_mixed_accessor_paths` | Hardcoded SlotIdx, SymbolId, u32 indices | kani::any() + bounded validity assumptions |
| H13 `kani_gate_08_constants_with_symbols` | Hardcoded ConstIdx, SlotIdx, SymbolId | kani::any() for all ID types + bounded validity assumptions |
| H14 `kani_gate_08_many_accessors_varied_depths` | Hardcoded SlotIdx, SymbolId, u32 indices | kani::any() + bounded validity assumptions |

**Note**: H12 (`kani_gate_08_all_node_kinds_no_panic`) was NOT modified - it already uses `kani::any()` correctly (not a violation per proof-plan-review).

### vb_runtime/kani_capability_harnesses.rs
| Harness | Violation | Fix |
|---------|-----------|-----|
| `AdmissionCaseStore::load_accepted_artifact` (case 4) | Hardcoded "network" | arbitrary_capability_name() helper |
| `check_capability_grants_exact_match` | Hardcoded "action" | arbitrary_capability_name() |
| `check_capability_action_match_name_grants` | Hardcoded "network" | arbitrary_capability_name() |
| `check_capability_action_match_name_denies` | Hardcoded "secrets"/"network" | arbitrary_capability_name() x2 + assume(names differ) |
| `check_capability_action_mismatch_name_grants` | Hardcoded "network" | arbitrary_capability_name() |
| `check_capability_action_mismatch_name_denies` | Hardcoded "secrets"/"network" | arbitrary_capability_name() x2 |
| `check_capability_hierarchical_rejects_subpath` | Hardcoded "network.api"/"network" | arbitrary_capability_name() + format!() |
| `check_capability_partial_segment_rejected` | Hardcoded "network"/"net" | arbitrary_capability_name() + prefix extraction |

**Note**: `check_capability_harness` was NOT modified - it already uses `kani::any()` for action and name (not a violation).

## Commands Run

| Command | Result |
|---------|--------|
| `cargo check -p vb_runtime -p vb_validate` | **PASS** - all crates compile |
| `cargo kani -p vb_runtime --harness verify_trace_ring_bounds` | **BLOCKED** - pre-existing vb_storage errors (43 missing kani::Arbitrary impls in kani_recovery_hydrate.rs) |

## Trusted Boundaries & Assumptions

1. **StepIdx/SlotIdx validity**: These are identifier types (u16 wrappers), not array indices. Any u16 value is valid for use in TraceEvent.

2. **Accessor validity assumptions**: In structural harnesses, `kani::assume()` ensures root < slot_count and field symbols < symbols_count. Gate 8 only validates accessors.

3. **Index sentinel filtering**: `kani::assume(idx != u32::MAX)` prevents the MAX sentinel value that Gate 8 treats as invalid.

4. **Capability name generation**: `arbitrary_capability_name()` uses `String::from_utf8_lossy` which always succeeds (replaces invalid UTF-8 with U+FFFD). Empty strings are replaced with "cap".

5. **Send/Sync compile-time enforcement**: These bounds are enforced by the Rust compiler. The harnesses verify `compiled_ir_exists` doesn't panic.

## Blockers

### BLOCKED_TOOLING: Kani proof execution
**Discovery command**: `cargo kani -p vb_runtime --harness verify_trace_ring_bounds`

**Issue**: vb_storage/src/kani_recovery_hydrate.rs has 43 compilation errors - missing `kani::Arbitrary` implementations for `EventSeq`, `CapabilitySet`, `RuntimePolicy`, `chrono::DateTime<Utc>`, `FjallJournal`, `Vec<JournalEvent>`.

**Impact**: Cannot execute Kani proofs on vb_runtime/vb_validate until vb_storage Arbitrary implementations are added.

**Recommendation**: Add `kani::Arbitrary` implementations to vb_storage types, or exclude kani_recovery_hydrate.rs from the Kani build.

## Obligations Completed
17 harness violations addressed (25 specific changes across 4 files):
- 2 vacuous assertions removed (admission_store)
- 3 hardcoded index violations fixed (trace_ring)
- 5 hardcoded WorkflowParts violations fixed (structural)
- 7 hardcoded capability name violations fixed (capability)

## Verification Status
- **Compilation**: PASS (cargo check)
- **Kani Smoke**: BLOCKED (pre-existing vb_storage issues)
- **Formal Proof**: PENDING (requires unblocking tooling)
