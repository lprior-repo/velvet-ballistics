
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260503135422-gkhqie0n
// Title: Codegen: Emit TogetherStart/Join
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260503135422-gkhqie0n.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260503135422-gkhqie0n"
  title: "Codegen: Emit TogetherStart/Join"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "CompiledNodeKind::TogetherStart { branches, join, output } fields are available.",
      "CompiledNodeKind::TogetherBranch { branch, entry, join, accumulator, output } fields are available.",
      "CompiledNodeKind::TogetherJoin { branch_count, accumulator, output, next } fields are available.",
      "The generated code has access to list manipulation helpers (insert_list, append).",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "Generated Rust for Together compiles without errors.",
      "Generated Together produces identical accumulator state to the interpreter for the same branch inputs.",
      "The emit_unsupported_node_step match arms for TogetherStart/Branch/Join are removed.",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Branch count must fit in u16 (max 65535 branches).",
      "Empty branches list must return InvalidCompiledWorkflow error.",
      "Null and List values in the last branch result must not be appended at join time (matching runtime semantics).",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(5)
    error_path_tests: [...string] & list.MinItems(3)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "Generated TogetherStart with 2 branches creates empty accumulator and jumps to first branch.",
      "Generated TogetherBranch with branch=0 skips append and jumps to entry.",
      "Generated TogetherBranch with branch=1 appends previous result to accumulator.",
      "Generated TogetherJoin with 2 branches produces merged list [result_a, result_b].",
      "Equivalence test: 3-branch together produces same output from interpreter and generated code.",
    ]

    // Required error path tests
    required_error_tests: [
      "Generated TogetherStart with zero branches returns InvalidCompiledWorkflow.",
      "Generated TogetherBranch with corrupted accumulator (not a list) returns TypeMismatch.",
      "Generated TogetherStart with output=None returns MissingOutputSlot.",
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