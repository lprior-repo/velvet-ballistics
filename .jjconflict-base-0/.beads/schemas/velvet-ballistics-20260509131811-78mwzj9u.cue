
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260509131811-78mwzj9u
// Title: runtime: Shard scheduler bounded ownership evidence
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260509131811-78mwzj9u.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260509131811-78mwzj9u"
  title: "runtime: Shard scheduler bounded ownership evidence"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "ShardConfig validation defines queue, run, timer, and step-budget limits",
      "Shard::status or equivalent status snapshot is available for evidence",
      "Tests can construct shards with small capacities",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "Queue capacity rejection is deterministic and typed",
      "Timer ownership is visible in pending_timer_count/status",
      "Shutdown drain processes at most the configured bound and leaves no orphaned ownership state",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "A run is owned by exactly one shard at a time",
      "Pending timer count never exceeds configured timer capacity",
      "One tick processes no more command or step work than its budget allows",
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
      "Shard tick processes queued commands in FIFO order up to budget",
      "Status snapshot reports active run count, pending timer count, queue length, and health",
    ]

    // Required error path tests
    required_error_tests: [
      "Submit beyond command queue capacity returns a typed capacity error",
      "Timer insertion beyond timer capacity rejects without leaking a run or timer",
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
//     timestamp: "2026-05-09T13:18:11Z"
//   }
// }