
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260509131811-hzg1s8kf
// Title: storage/runtime: Single-server database lock enforcement
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260509131811-hzg1s8kf.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260509131811-hzg1s8kf"
  title: "storage/runtime: Single-server database lock enforcement"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "Existing journal open path is in crates/vb_storage/src/journal.rs",
      "Process lock behavior is represented by crates/vb_storage/src/process_lock.rs or equivalent storage module",
      "Tests use isolated temporary directories and do not depend on global machine state",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "First open succeeds and records lock ownership evidence",
      "Second same-path open returns a typed lock-held error",
      "Failed open leaves the target directory free of new storage artifacts",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "At most one live FjallJournal owns a database path per process boundary",
      "Lock acquisition completes before Fjall keyspace creation",
      "Lock release occurs when the owning journal is dropped",
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
      "First journal open creates a process lock file containing the holder pid",
      "Journal drop releases ownership and a later open succeeds",
    ]

    // Required error path tests
    required_error_tests: [
      "Second journal open on the same path returns ProcessLockHeld or equivalent typed error",
      "Forced lock failure does not create keyspaces, blobs, or journal partitions",
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