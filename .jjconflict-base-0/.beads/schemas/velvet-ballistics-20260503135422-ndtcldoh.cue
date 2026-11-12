
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260503135422-ndtcldoh
// Title: CLI: diff command
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260503135422-ndtcldoh.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260503135422-ndtcldoh"
  title: "CLI: diff command"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "Both workflow file paths are readable.",
      "The vb_compile crate can compile each file independently.",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "A structured diff report is emitted showing added/removed/modified steps, resource contract deltas, and secret changes.",
      "Exit code is 0 if workflows are semantically identical, 1 if differences exist, 2 on error.",
      "No workflow steps are executed.",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Diff MUST compare compiled IR, never raw YAML text.",
      "Deterministic: same inputs always produce the same diff output.",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(3)
    error_path_tests: [...string] & list.MinItems(3)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "Two identical workflow files produce exit code 0 and an empty diff.",
      "A workflow with an added step versus the original produces exit code 1 and shows the addition.",
      "A workflow with a modified step action produces exit code 1 and shows the modification.",
    ]

    // Required error path tests
    required_error_tests: [
      "One invalid file produces exit code 2 with compilation error details.",
      "Both files invalid produces exit code 2 with both sets of errors.",
      "Missing file produces exit code 2 with file-not-found message.",
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