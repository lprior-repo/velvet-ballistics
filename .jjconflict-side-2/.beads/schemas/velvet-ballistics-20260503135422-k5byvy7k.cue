
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260503135422-k5byvy7k
// Title: CLI: cancel command
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260503135422-k5byvy7k.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260503135422-k5byvy7k"
  title: "CLI: cancel command"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "The storage database at --db path exists and is accessible.",
      "The run identified by run-id exists in storage.",
      "The run is in a non-terminal state (running or suspended).",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "A RunCancelled event is appended to the event journal for the run.",
      "The run state transitions to Cancelled.",
      "No further step execution occurs for the cancelled run.",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Event journal is append-only; no existing events are modified.",
      "Each run has at most one RunCancelled event.",
      "The run-id is never mutated.",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(3)
    error_path_tests: [...string] & list.MinItems(4)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "Cancel a running run and verify RunCancelled event is written.",
      "Cancel a suspended run and verify it transitions to Cancelled state.",
      "Cancel command prints human-readable confirmation with run-id.",
    ]

    // Required error path tests
    required_error_tests: [
      "Cancel a non-existent run-id and verify a clear error message.",
      "Cancel with an invalid --db path and verify storage error is reported.",
      "Cancel an already-completed run and verify rejection with appropriate message.",
      "Cancel an already-cancelled run and verify idempotency or rejection.",
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