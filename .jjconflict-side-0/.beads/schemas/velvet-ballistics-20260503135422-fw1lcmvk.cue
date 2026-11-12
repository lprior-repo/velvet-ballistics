
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260503135422-fw1lcmvk
// Title: CLI: retry command
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260503135422-fw1lcmvk.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260503135422-fw1lcmvk"
  title: "CLI: retry command"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "The storage database at --db path exists and is accessible.",
      "The run identified by run-id exists in storage.",
      "The step identified by step-id exists within the run.",
      "The step is in a Failed state.",
      "The attempt count has not exceeded the configured maximum.",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "The step's ActionTicket attempt field is incremented.",
      "A retry event is recorded in the journal.",
      "The step is re-enqueued for execution with the new attempt number.",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Attempt counter monotonically increases.",
      "Original step inputs are preserved across retries.",
      "Previous attempt results remain in the journal for audit.",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(2)
    error_path_tests: [...string] & list.MinItems(5)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "Retry a failed step and verify it re-executes with incremented attempt counter.",
      "Retry command prints confirmation with run-id, step-id, and attempt number.",
    ]

    // Required error path tests
    required_error_tests: [
      "Retry a non-existent run-id and verify error message.",
      "Retry a non-existent step-id within a valid run and verify error.",
      "Retry a step that is in Completed state and verify rejection.",
      "Retry a step that is in Running state and verify rejection.",
      "Retry with invalid --db path and verify storage error.",
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