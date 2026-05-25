# Trusted Base Plan - vb-xi2f.37

## Trust Markers

### TM-001: No Unsafe Code
- **Location**: vb_yaml/src/ast/types.rs, vb_yaml/src/ast/parse_steps.rs
- **Marker**: `#![forbid(unsafe_code)]`
- **Implication**: Miri lane not required for UB detection

### TM-002: Parsing is Local
- **Location**: vb_yaml/src/ast/parse_steps.rs
- **Implication**: No cross-step temporal behavior; TLA+ not required

### TM-003: Single-Threaded Parsing
- **Location**: vb_yaml parsing is synchronous
- **Implication**: No concurrency primitives; Loom not required

## Trusted Boundaries

### TB-001: YamlError Enum
- **Trusted**: All YamlError variants are exhaustively handled
- **Surface**: parse_steps.rs lines 44-83
- **Invariant**: Unknown YamlError cannot leak

### TB-002: StepPrimitive Enum
- **Trusted**: All StepPrimitive variants handled in match
- **Surface**: parse_step_primitive() match statement
- **Invariant**: Non-exhaustive enum prevents undefined variants

## Known Assumptions

### A-001: Bounded String Input
- **Assumption**: Primitive names are ASCII lowercase strings
- **Bound**: Max 20 characters
- **Source**: is_primitive() string comparison

### A-002: Bounded Field List
- **Assumption**: reject_unknown_step_fields has finite field list
- **Bound**: 20 items in array
- **Source**: parse_steps.rs line 108

### A-003: YAML Node Depth
- **Assumption**: YAML document depth is bounded by parser limits
- **Bound**: Configured in vb_yaml/limits.rs
- **Source**: limits.rs constants

## Model Reductions

### MR-001: Parse Only
- **Reduction**: Only parse/validate pipeline, not runtime
- **Justification**: reduce is a compile-layer name acceptance only
- **Risk**: None - runtime unchanged

### MR-002: No State Machine
- **Reduction**: No temporal properties modeled
- **Justification**: Local string matching only
- **Risk**: None - stateless transformation

## Stub Declarations

None - all proof obligations have concrete implementation paths.
