bead_id: vb-qi37.7.3
bead_title: "validate: symbol/reference bounds and resource contract errors"
phase: 5
updated_at: 2026-05-09T01:10:00Z

# Contract Specification — Validation Error Coverage: Symbol Bounds & Resource Contracts

## Context
- **Feature**: Validate that compiled workflow IR correctly enforces symbol/reference bounds
  and resource contract limits, producing typed `WorkflowError` and `ValidationError`
  variants at the correct abstraction boundaries.
- **Domain terms**: `validate_symbol`, `validate_accessor_paths`, `validate_resource_contract`,
  `WorkflowError::{SymbolOutOfBounds, ResourceContractExceeded, ResourceContractTooLarge}`,
  `ValidationError::AccessorSymbolOutOfBounds`, `ValidationPipeline::validate`
- **Assumptions**:
  - `WorkflowParts` is well-structured (nodes, expressions, accessors, constants, symbols)
  - `ResourceContract` fields are set before validation calls
  - The vb_validate pipeline gate functions operate on `WorkflowParts` with a `symbols_count` field

## Error Contract

### `validate_symbol` → `WorkflowError::SymbolOutOfBounds`

The internal function `validate_symbol(symbol, symbols_count)` in `vb_core::workflow::mod`
is the single source of truth for symbol out-of-bounds detection:

```rust
fn validate_symbol(symbol: SymbolId, symbols_count: u32) -> Result<(), WorkflowError> {
    if symbol.get() < symbols_count {
        Ok(())
    } else {
        Err(WorkflowError::SymbolOutOfBounds { symbol })
    }
}
```

Callers: `validate_accessor_paths`, `validate_constants_symbols`, `validate_build_object_symbols`.

### `validate_gate_08_accessor_path_segments` → `ValidationError::AccessorSymbolOutOfBounds`

The vb_validate gate 8 function checks accessor path segments and maps symbol bounds
violations to `ValidationError::AccessorSymbolOutOfBounds`:

```rust
PathSegment::Field(sym_id) => {
    if sym_id.get() >= parts.symbols_count {
        return Err(ValidationError::AccessorSymbolOutOfBounds {
            accessor_index: acc_index,
            segment_index: seg_index,
            symbol: sym_id.get(),
            symbols_count: parts.symbols_count,
        });
    }
}
```

### `validate_resource_contract` → `WorkflowError::{ResourceContractTooLarge, ResourceContractExceeded}`

Two distinct error paths exist in the resource contract validation:

- `ResourceContractTooLarge`: declared limit in the contract itself exceeds protocol hard limit
- `ResourceContractExceeded`: actual compiled artifact exceeds the declared contract limit

```rust
fn validate_contract_limit(
    resource: &'static str,
    actual: usize,
    declared: usize,
    hard_limit: usize,
) -> Result<(), WorkflowError> {
    if declared > hard_limit {
        return Err(WorkflowError::ResourceContractTooLarge { resource });
    }
    if actual > declared {
        Err(WorkflowError::ResourceContractExceeded { resource })
    } else {
        Ok(())
    }
}
```

## Pipeline `validate` → `ValidationError`

The vb_validate `ValidationPipeline::validate(parts)` runs all gates and returns
`ValidationResult<()>`. Each gate may produce a distinct `ValidationError` variant.
The pipeline short-circuits on the first error.

Key `ValidationError` variants produced by the pipeline:

| Gate | Error Variant |
|------|---------------|
| Gate 7 | `ExpressionStackExceeded`, `ExpressionStackMismatch` |
| Gate 8 | `AccessorSymbolOutOfBounds`, `AccessorPathTooDeep`, `AccessorSlotOutOfRange` |
| Gate 9 | `SlotReferenceOutOfRange` |
| Gate 10 | `NodeKindConstraintViolation` |
| Gate 11 | `LoopBodyStepOutOfRange` |
| Gate 13 | `SlotDependencyCycle` |
| Gate 14 | `SlotTypeInconsistency` |
| Gate 15 | `NonDeterministicPath` |

## Contract Boundaries

1. **Symbol bounds** are enforced at the vb_core workflow layer via `validate_symbol`.
   The vb_validate gate 8 function re-enforces the same invariant and surfaces it as
   `ValidationError::AccessorSymbolOutOfBounds`.
2. **Resource contract** errors are produced only by `validate_resource_contract` in vb_core.
   The vb_validate pipeline does not independently produce `WorkflowError` variants;
   it operates only on `WorkflowParts` and produces `ValidationError`.
3. The two error systems (`WorkflowError` in vb_core, `ValidationError` in vb_validate)
   are strictly separated. No function in vb_validate returns `WorkflowError`.

## Non-goals
- This bead does not cover runtime slot out-of-bounds errors (those occur in `RunFrame`).
- This bead does not cover budget policy errors (`WorkflowError::BudgetPolicyExceeded`).
- This bead does not cover action contract validation (Gate 12).
