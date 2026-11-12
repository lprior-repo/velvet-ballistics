
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260503135422-oe0dgyee
// Title: Core: Execution attempt tracking with StaleAttempt rejection -- monotonic counter, journal tagging, and recovery filter
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260503135422-oe0dgyee.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260503135422-oe0dgyee"
  title: "Core: Execution attempt tracking with StaleAttempt rejection -- monotonic counter, journal tagging, and recovery filter"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "RunState.action_attempts is allocated with one entry per workflow step",
      "RetryPolicy for the action is available for max_attempts bounding",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "ActionTicket.attempt is monotonically increasing per step within a run",
      "ActionJournalEvent variants include the attempt number",
      "Stale completions (attempt < current) are rejected without modifying run state",
      "The attempt counter never exceeds the retry policy's max_attempts",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Attempt counters are per-step, not global per-run",
      "The counter starts at 1 (1-indexed, matching ActionTicket.attempt convention)",
      "Replay of journal events preserves the attempt sequence",
      "StaleAttempt rejection is deterministic and does not depend on timing",
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
      "Given a Do-node that suspends 3 times with retries, the attempt counter goes 1, 2, 3 and each ticket carries the correct attempt",
      "Given a completion with attempt=2 when current is 2, the completion is accepted",
    ]

    // Required error path tests
    required_error_tests: [
      "Given a completion with attempt=1 when current is 3, the completion is rejected as StaleAttempt",
      "Given an attempt counter at max_attempts, a subsequent suspension is rejected as exhausted",
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