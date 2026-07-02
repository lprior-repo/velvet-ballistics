
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260503135422-qmpylgwe
// Title: CLI: incident command -- black box report with failure code, side-effect certainty, and repair hints
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260503135422-qmpylgwe.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260503135422-qmpylgwe"
  title: "CLI: incident command -- black box report with failure code, side-effect certainty, and repair hints"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "FjallJournal is open and accessible at the configured data path",
      "RunId corresponds to a run that has at least one journal event",
      "ActionContract registry is populated with contracts for all actions referenced in the run",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "Report contains the terminal ActionFailureCode or ReplayError variant",
      "Report contains a SideEffect certainty classification for each action step in the run",
      "Report contains at least one repair hint when a failure is detected",
      "Report includes the run_id, failure step index, and failure code",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "The incident command never mutates journal data",
      "Replay is read-only and cannot alter slot state outside the diagnostic context",
      "Side-effect certainty is derived solely from ActionContract.side_effect and ActionContract.retry_safety",
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
      "Given a run that failed with ActionFailureCode::Timeout at step 3, when incident is invoked, report shows Timeout, side-effect Writes, and a hint about increasing timeout_ms",
      "Given a run that stalled on a NonDeterministicStep (Do node), report shows the blocking step and suspended action id",
    ]

    // Required error path tests
    required_error_tests: [
      "Given a nonexistent run_id, incident returns an error with diagnostic code RUN_NOT_FOUND",
      "Given a run with zero journal events, incident returns an error indicating no recovery data",
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