# ADR 008 (v1): Expression Engine

## Status

Accepted as architecture baseline. Implementation completion requires evidence.

## Decision

Expressions compile to bounded bytecode before runtime execution. Runtime expression evaluation uses numeric slot operands, constant pools, accessors, finite numeric values, and checked stack operations.

Boolean `and` and `or` do not short-circuit.

## Invariants

- Max expression nesting depth is 64.
- Max helper arity is 8.
- Max tokens are 256.
- Max source bytes are 4096.
- Runtime stack depth is 64.
- Integer overflow and division by zero return typed errors.
- `FiniteF64` rejects NaN and infinities.

## Known Gaps

The master identifies mixed I64/F64 parity and helper parity gaps. ADR acceptance does not close those gaps.

## Master Anchors

- Section 27: Mandatory Function Surface: `vb_expr`
- Section 38: Property Tests
- Section 46: Expression Grammar, Type System, and Helper Signatures
