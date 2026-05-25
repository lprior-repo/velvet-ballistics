# Proof Strategy - vb-xi2f.37

## Bead
**ID**: vb-xi2f.37
**Title**: P0: accept canonical reduce primitive name
**Scope**: reduce primitive in YAML parse/validate/compile pipeline

## Risk Classification
| Risk | Assessment | Rationale |
|------|------------|-----------|
| Parsing regression | Medium | Adding new primitive to match arms could miss cases |
| Type consistency | Low | Enum variant addition is straightforward |
| Unknown field bypass | Medium | If "reduce" not added to rejection list, could be silently ignored |
| No panic on parse | Low | Kani can verify bounded input space |
| Canonical name mapping | Low | Simple string mapping, unit test sufficient |

## Verifier Selection
| Risk | Verifier | Rationale |
|------|----------|-----------|
| Parse no panic | Kani | Bounded model checking for step parsing |
| Unknown field rejection | Kani | Cover "reduce" is not rejected |
| Reduce variant correctness | Verus | Rust-local type invariants |
| Unit test coverage | cargo test | Standard test lane |
| Fuzz coverage | cargo-fuzz | Random step generation |

## TLA+ Lane: NOT APPLICABLE
- **Rationale**: This is a purely local parsing change, no temporal or state-machine behavior
- **Evidence**: No cross-step temporal properties, no queue/state machine involved

## Verus Lane: REQUIRED
- **Artifact**: vb_yaml/src/ast/types.rs
- **Focus**: StepPrimitive::Reduce variant is well-formed
- **Command**: `verus vb_yaml/src/ast/types.rs`

## Kani Lane: REQUIRED
- **Artifact**: vb_yaml/src/ast/parse_steps.rs
- **Harness**: `parse_step_primitive_reduce_nopanic`
- **Command**: `cargo kani --harness parse_step_primitive_reduce_nopanic`
- **Assumptions**: Bounded YAML node depth

## Miri Lane: NOT APPLICABLE
- **Rationale**: No unsafe code in vb_yaml AST parsing
- **Evidence**: #![forbid(unsafe_code)] in types.rs and parse_steps.rs

## Loom Lane: NOT APPLICABLE
- **Rationale**: No concurrency primitives involved
- **Evidence**: Single-threaded YAML parsing only

## Flux Lane: NOT APPLICABLE
- **Rationale**: No refinement types needed; enum variant is simple
- **Evidence**: StepPrimitive is a plain enum, not a refined type

## Proptest Lane: REQUIRED
- **Artifact**: vb_yaml/src/profile_tests.rs or dedicated property test
- **Focus**: is_primitive returns correct values for all strings
- **Command**: `cargo test --test profile_reduce_primitive`

## Fuzz Lane: CONDITIONAL
- **Artifact**: fuzz/ directory
- **Focus**: Reduce step parsing with malformed input
- **Command**: `cargo fuzz run parse_steps`

## Proof Obligations

### PO-vb-xi2f-001: Reduce recognized by is_primitive
- **Requirement**: CC-001
- **Verifier**: Kani
- **Artifact**: vb_yaml/src/ast/parse_steps.rs
- **Command**: `cargo kani --harness is_primitive_reduce`
- **Expected Evidence**: Kani reports no witness
- **Assumptions**: String input bounded to ASCII
- **Required**: Yes
- **Mode**: verify-deep

### PO-vb-xi2f-002: Reduce not rejected by unknown fields
- **Requirement**: CC-002
- **Verifier**: Kani
- **Artifact**: vb_yaml/src/ast/parse_steps.rs
- **Command**: `cargo kani --harness reject_unknown_reduce`
- **Expected Evidence**: Kani reports no witness for rejection path
- **Assumptions**: Bounded field list
- **Required**: Yes
- **Mode**: verify-deep

### PO-vb-xi2f-003: Reduce variant exists and parses
- **Requirement**: CC-003, CC-001
- **Verifier**: Verus
- **Artifact**: vb_yaml/src/ast/types.rs
- **Command**: `verus vb_yaml/src/ast/types.rs`
- **Expected Evidence**: Verus verified with 0 errors
- **Assumptions**: None
- **Required**: Yes
- **Mode**: verify-proof

### PO-vb-xi2f-004: Unit test coverage
- **Requirement**: CC-001, CC-002, CC-004
- **Verifier**: cargo test
- **Artifact**: vb_yaml/src/profile_tests.rs
- **Command**: `cargo test -p vb_yaml reduce`
- **Expected Evidence**: All tests pass
- **Assumptions**: None
- **Required**: Yes
- **Mode**: test

### PO-vb-xi2f-005: Fuzz parse_steps with reduce
- **Requirement**: CC-001
- **Verifier**: cargo-fuzz
- **Artifact**: fuzz/src/parse_steps.rs
- **Command**: `cargo fuzz run parse_steps`
- **Expected Evidence**: No crashes after 10M iterations
- **Assumptions**: Corpus includes reduce steps
- **Required**: No (waived - corpus would need reduce additions)
- **Mode**: fuzz
- **Waiver Reason**: Fuzz corpus update is separate work

## Waiver Requests
- Fuzz lane: corpus update required for reduce coverage; separate bead recommended

## Strategy Summary
- Primary: Kani for bounded parsing verification
- Secondary: Verus for type invariants
- Support: Unit tests for canonical name mapping
- Excluded: TLA+, Miri, Loom, Flux (not applicable by evidence)
