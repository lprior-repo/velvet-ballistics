
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260509134851-twyv8ojt
// Title: runtime/storage: Persist execution attempt numbers and reject stale completions
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260509134851-twyv8ojt.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260509134851-twyv8ojt"
  title: "runtime/storage: Persist execution attempt numbers and reject stale completions"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "Run admission and action completion paths have been located in vb_runtime and vb_storage",
      "A typed stale-attempt error variant or equivalent diagnostic path exists or is added",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "RunAccepted or equivalent durable admission record carries an attempt number",
      "Action tickets and completion/failure journal events carry the same attempt number",
      "Stale completions are rejected before journal or index mutation",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "A run has exactly one latest accepted attempt at a time",
      "Older attempts cannot win after a newer attempt is admitted",
      "Attempt checks happen before durable mutation",
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
      "admitting a run persists attempt one before ack",
      "retrying a run increments and persists the latest attempt before new tickets are issued",
    ]

    // Required error path tests
    required_error_tests: [
      "completion for attempt one is rejected after attempt two is admitted",
      "stale failure event cannot overwrite the latest attempt terminal state",
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
//     timestamp: "2026-05-09T13:48:51Z"
//   }
// }