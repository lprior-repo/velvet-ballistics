# Defects Document — vb-e4mt

## Bead: Resource Bounds and Budget Enforcement
**State**: 12 (black-hat review complete)
**Date**: 2026-05-19

---

## Defect Log

| ID | Severity | Type | File | Line | Description | Fix Required |
|----|----------|------|------|------|-------------|--------------|
| DEFECT-MINOR-1 | MINOR | Policy Violation | `budget.rs` | 1414 | `unwrap_or` in `branch_count_to_u16` violates zero-unwrap policy | Yes (before merge) |
| DEFECT-MINOR-2 | MINOR | Documentation | `contract.md` | POST-004 | Contract says `fits_within` returns `bool`; implementation returns `Result<(), AggregateBudgetError>` | Documentation fix only |

---

## DEFECT-MINOR-1: Unwrap in `branch_count_to_u16`

**Location**: `crates/vb_core/src/budget.rs:1412-1416`

**Code**:
```rust
fn branch_count_to_u16(count: usize) -> Result<u16, WorkflowError> {
    u16::try_from(count).map_err(|_| WorkflowError::StepCountOverflow {
        actual: u64::try_from(count).unwrap_or(u64::MAX),
    })
}
```

**Problem**: The `unwrap_or` on line 1414 violates the zero-unwrap policy. While it is **architecturally unreachable** on any Rust-supported platform (where `usize` is at most 64 bits and `u64::try_from(usize)` cannot fail), the pattern is forbidden by policy.

**Fix**: Replace `unwrap_or(u64::MAX)` with a const-assert or remove the inner `u64::try_from` entirely since the outer `u16::try_from` failing implies `count > u16::MAX < u64::MAX`:

```rust
fn branch_count_to_u16(count: usize) -> Result<u16, WorkflowError> {
    u16::try_from(count).map_err(|_| {
        // usize cannot exceed u64::MAX on any supported platform
        WorkflowError::StepCountOverflow {
            actual: count as u64,
        }
    })
}
```

**Risk**: None. The `as u64` cast from `usize` to `u64` is always safe on supported platforms.

**Charged to**: vb-e4mt implementation.

---

## DEFECT-MINOR-2: Contract/Code Mismatch on `fits_within` Return Type

**Location**: `contract.md` POST-004 vs `budget.rs:570`

**Contract says**:
> `AggregateResourceUsage::fits_within` returns `true` iff all dimensions of `self` are <= corresponding dimensions of `capacity`.

**Implementation**:
```rust
pub fn fits_within(
    &self,
    capacity: &AggregateResourceCapacity,
) -> Result<(), AggregateBudgetError> {
    // ... checks ...
    Ok(())
}
```

**Problem**: The contract describes boolean semantics (`true`/`false`) but the implementation returns `Result<(), AggregateBudgetError>` where `Ok(())` is semantically equivalent to `true` and `Err(_)` equivalent to `false`. The behavior is correct; the contract language is imprecise.

**Fix**: Update `contract.md` POST-004 to:
> `AggregateResourceUsage::fits_within` returns `Ok(())` iff all dimensions of `self` are <= corresponding dimensions of `capacity`; returns `AggregateBudgetError::CapacityExceeded` otherwise.

**Risk**: None. This is a documentation fix only.

**Charged to**: vb-e4mt contract maintenance (not implementation).

---

## Non-Defects (Information Only)

### KANI-BUDGET-001 Timeout — Harness Architecture Issue

**Not charged to production code.** The timeout occurs because:
1. `kani::any::<WorkflowParts>()` generates arbitrarily large `CompiledNode` slices
2. `compute_fanout_and_depth` recurses through all nodes
3. `#[kani::unwind(6)]` limits exploration but causes state explosion on branching workflows

**Production code is panic-free** (cycle detection, `checked_add`, explicit error returns). The harness architecture is the bottleneck.

### OQ-001: GAP-1 BudgetError Missing Fields

**Not charged to vb-e4mt.** The missing `primitive`, `node_index`, `structural_path` fields per vb_qi37_2_4 BLOCK_LOCAL spec is an open question (OQ-001) from the contract phase. Resolution requires cross-bead coordination with vb_qi37_2_4. Not a vb-e4mt implementation defect.

### fmt DEFERRED_GLOBAL

**Pre-existing issue, not charged to this bead.**
