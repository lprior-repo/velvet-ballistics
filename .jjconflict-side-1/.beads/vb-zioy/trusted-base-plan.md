# Trusted Base Plan: vb-zioy

## Overview

This bead introduces minimal trusted surface. The fix is a parameter-plumbing change with no model reductions, stub boundaries, or axiomatic assumptions.

## Trusted Base Entries

### T1: Caller Passes Valid Source Index

- **Obligation IDs**: PO-003, PO-004, PO-005
- **Location**: `lower_canonical_*` call sites in `mod_compile_lowering/part_02.rs`, `part_03.rs`, `part_04.rs`
- **Marker**: `// CALLER-CONTRACT: diagnostic_step must be the original source AST index`
- **Trusted Kind**: `caller_obligation`
- **Reason**: The type system cannot distinguish `usize` source indices from `usize` synthetic indices. We trust that each caller passes its `index: usize` parameter (available in every lowering function signature) rather than a computed offset.
- **Scope**: All 5 call sites of `emit_single_body_set`
- **Impact**: If a caller passes a wrong index, diagnostics will still be incorrect despite `emit_single_body_set` using the parameter faithfully.
- **Compensating Evidence**: 
  - Code review of all call sites
  - Grep verification: `grep -n 'emit_single_body_set' crates/vb_compile/src/mod_compile_lowering/*.rs`
  - Integration tests for each scoped primitive verify the reported step matches the expected source index
- **Behavior Affecting**: Yes
- **Owner**: proof-to-implementation bridge (State 5)
- **Expiry**: Permanent (until `SourceStepIdx` newtype is introduced in a future bead)

### T2: Existing proptest Strategies Are Sound

- **Obligation IDs**: PO-001, PO-002
- **Location**: `crates/vb_compile/src/proptest_body_dispatcher.rs`, `proptest_error_parity.rs`
- **Marker**: `// STRATEGY-TRUST: non_set_body_strategy covers all non-Set variants`
- **Trusted Kind**: `strategy_coverage`
- **Reason**: The proptest strategies manually enumerate non-Set StepPrimitive variants. We trust that this enumeration is exhaustive.
- **Scope**: PO-001 (StepFieldShape) and PO-002 (UnsupportedStepPrimitive)
- **Impact**: Missing a variant in the strategy would leave a gap in coverage.
- **Compensating Evidence**: 
  - Review against `vb_yaml::ast::StepPrimitive` enum definition
  - Existing `proptest_error_parity.rs` already enumerates variants
- **Behavior Affecting**: No
- **Owner**: proof-writer (State 6)
- **Expiry**: Permanent

## No Other Trusted Base

- No `assume`, `axiom`, `admit`, `external_body`, `trusted`, `ignore`, or disabled checks.
- No unsafe code to Miri-trust.
- No model abstractions or reductions.
