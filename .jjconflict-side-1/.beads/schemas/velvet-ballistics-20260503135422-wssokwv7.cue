
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260503135422-wssokwv7
// Title: Codegen: Emit RepeatStart/Attempt/Check/Finish
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260503135422-wssokwv7.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260503135422-wssokwv7"
  title: "Codegen: Emit RepeatStart/Attempt/Check/Finish"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "CompiledNodeKind::RepeatStart { max_attempts, body, done, output } fields are available.",
      "CompiledNodeKind::RepeatAttempt { attempt_slot, body, done } fields are available.",
      "CompiledNodeKind::RepeatCheck { attempt_slot, done, next } fields are available.",
      "CompiledNodeKind::RepeatFinish { result, output, next } fields are available.",
      "The packed state encoding uses REPEAT_SHIFT = 32 with u16 fields.",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "Generated Rust for Repeat compiles without errors.",
      "Generated Repeat produces identical attempt counting to the interpreter.",
      "The emit_unsupported_node_step match arms for RepeatStart/Attempt/Check/Finish are removed.",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "max_attempts must be nonzero (enforced at RepeatStart).",
      "current_attempt must not exceed max_attempts.",
      "Packed state encoding: (max_attempts as i64) << 32 | (current_attempt as i64).",
      "Reserved bits (16-31) must be zero.",
      "saturating_add on current_attempt prevents overflow at u16::MAX.",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(5)
    error_path_tests: [...string] & list.MinItems(4)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "Generated RepeatStart with max_attempts=5 writes packed state (5, 0) and jumps to body.",
      "Generated RepeatCheck increments counter from 0 to 1 and routes to body when 1 < 5.",
      "Generated RepeatCheck at boundary (attempt=4, max=5) increments to 5 and routes to done.",
      "Generated RepeatFinish writes result to output and continues.",
      "Equivalence test: 3-attempt loop with condition produces same behavior from interpreter and generated code.",
    ]

    // Required error path tests
    required_error_tests: [
      "Generated RepeatStart with max_attempts=0 returns InternalInvariantViolation.",
      "Generated RepeatAttempt with non-I64 attempt slot returns TypeMismatch.",
      "Generated RepeatCheck with corrupted packed state returns InternalInvariantViolation.",
      "Generated RepeatFinish with output=None returns MissingOutputSlot.",
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