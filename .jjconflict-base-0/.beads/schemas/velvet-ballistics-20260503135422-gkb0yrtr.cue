
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260503135422-gkb0yrtr
// Title: Codegen: Emit ForEachStart/Next/Join
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260503135422-gkb0yrtr.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260503135422-gkb0yrtr"
  title: "Codegen: Emit ForEachStart/Next/Join"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "CompiledNodeKind::ForEachStart { input, item_slot, limit, body, done } fields are available.",
      "CompiledNodeKind::ForEachNext { iterator_slot, body, done } fields are available.",
      "CompiledNodeKind::ForEachJoin { materialized, output, next } fields are available.",
      "The generated code has access to read_slot, write_slot, SlotValue, StepOutcome, DriveError.",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "Generated Rust for ForEach loops compiles without errors.",
      "Generated ForEach produces identical slot state to the interpreter runtime for the same input.",
      "The emit_unsupported_node_step match arms for ForEachStart/Next/Join are removed.",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Limit enforcement must match runtime: item_count > limit returns Err(DriveError::IterationLimitExceeded).",
      "Empty input list must route to done without binding item_slot.",
      "Tail list must be materialized as a fresh SlotValue::List handle.",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(4)
    error_path_tests: [...string] & list.MinItems(3)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "Generated ForEachStart with a 3-item list and limit=10 jumps to body with first item bound.",
      "Generated ForEachNext on a 2-item tail binds the next item and writes the 1-item tail back.",
      "Generated ForEachJoin writes the materialized list to output and continues to next.",
      "Equivalence test: interpreter and generated code produce identical slot states for a 5-item list.",
    ]

    // Required error path tests
    required_error_tests: [
      "Generated ForEachStart with a non-list input returns TypeMismatch.",
      "Generated ForEachStart with item_count exceeding limit returns IterationLimitExceeded.",
      "Generated ForEachNext with a non-list iterator returns TypeMismatch.",
    ]
  }

  // Code completion
  code_complete: {
    implementation_exists: string  // Path to implementation file
    tests_exist: string  // Path to test file
    ci_passing: bool & true
    no_unwrap_calls: bool & true  // Rust/functional constraint
    no_panics: bool & true  // Rust constraint
  }

  // Completion criteria
  completion: {
    all_sections_complete: bool & true
    documentation_updated: bool
    beads_closed: bool
    timestamp: string  // ISO8601 completion timestamp
  }
}

// Example implementation proof - create this file to validate completion:
//
// implementation.cue:
// package validation
//
// implementation: #BeadImplementation & {
//   contracts_verified: {
//     preconditions_checked: true
//     postconditions_verified: true
//     invariants_maintained: true
//     precondition_checks: [/* documented checks */]
//     postcondition_checks: [/* documented verifications */]
//     invariant_checks: [/* documented invariants */]
//   }
//   tests_passing: {
//     all_tests_pass: true
//     happy_path_tests: ["test_version_flag_works", "test_version_format", "test_exit_code_zero"]
//     error_path_tests: ["test_invalid_flag_errors", "test_no_flags_normal_behavior"]
//   }
//   code_complete: {
//     implementation_exists: "src/main.rs"
//     tests_exist: "tests/cli_test.rs"
//     ci_passing: true
//     no_unwrap_calls: true
//     no_panics: true
//   }
//   completion: {
//     all_sections_complete: true
//     documentation_updated: true
//     beads_closed: false
//     timestamp: "2026-05-03T13:54:22Z"
//   }
// }