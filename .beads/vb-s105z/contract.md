# Constant Pool Lookup Contract

## Component
Compiler constant pool management for branch instruction operands.

## Defect
Lookup returns the first pool entry regardless of value match and accepts malformed requests without parsing.

## Required Behavior
Parse the requested operand as a signed 64-bit integer before any pool lookup. When constant reuse is enabled, return an existing pool index only if its stored value equals the parsed integer. Otherwise, append the parsed value as a new constant entry. Convert the resulting index to u16 with checked arithmetic, propagating typed errors on overflow or parse failure.

## Invariants
The pool never returns an index for a value different from the requested one. Malformed or out-of-range inputs are rejected even when the pool contains entries. Distinct integer values always occupy distinct pool slots. Equal values share a single slot when reuse is enabled.

## Test Requirements
Verify two distinct values produce separate pool entries. Verify equal values reuse the same entry when reuse is enabled. Verify malformed input is rejected with a non-empty pool. Verify out-of-range values are rejected. Verify numeric boundary conditions. Verify the compiled branch instruction IR contains the correct operand values matching the source.

## Workflow
Rust contract specification, implementation, test suite, code review, test review. No proof planning.

## IR Digest Note
Numeric IR digest computation excludes authoring-only labels. Raw source identity tracking remains separate from the digest.
