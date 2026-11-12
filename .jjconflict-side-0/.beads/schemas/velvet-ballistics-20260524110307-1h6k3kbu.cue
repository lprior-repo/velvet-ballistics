
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260524110307-1h6k3kbu
// Title: compiler: Lower nested do primitive bodies
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260524110307-1h6k3kbu.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260524110307-1h6k3kbu"
  title: "compiler: Lower nested do primitive bodies"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "Read to-fix/01-compiler-yaml-ir-defects.md and the referenced master sections before editing",
      "Inspect the current public compile path and affected validation/lowering modules before selecting files",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "lower nested do body steps to numeric final IR through try_from_parts",
      "A bead-local evidence artifact records commands, logs, commit SHA, changed files, and residual risks",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "No production path may introduce unwrap, expect, panic, unsafe, unchecked indexing, or runtime YAML/string lookup in final IR",
      "All public failure paths return typed symbolic diagnostics instead of panicking",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(2)
    error_path_tests: [...string] & list.MinItems(2)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "Nested do inside a valid body compiles",
      "Nested do action reference is numeric before runtime",
    ]

    // Required error path tests
    required_error_tests: [
      "Unknown nested action returns symbolic diagnostic",
      "Invalid nested do input reference returns symbolic diagnostic",
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
//     timestamp: "2026-05-24T11:03:07Z"
//   }
// }