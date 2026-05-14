
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260503135422-g9ehfmji
// Title: Storage: Process lock with flock -- exclusive lock on ipc-serve startup with typed error including PID
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260503135422-g9ehfmji.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260503135422-g9ehfmji"
  title: "Storage: Process lock with flock -- exclusive lock on ipc-serve startup with typed error including PID"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "The data directory exists and is writable",
      "No other process holds an exclusive flock on the lock file",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "On success, the current process holds an exclusive flock on the lock file",
      "On failure, the error message includes the PID of the holding process (if detectable)",
      "The lock file descriptor is held open for the lifetime of the process",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Only one process can hold the exclusive lock at a time",
      "The lock is released automatically on process exit (crash, kill, normal shutdown)",
      "No unsafe code is used in the lock implementation",
      "flock is used (not fcntl) for automatic crash release semantics",
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
      "Given no existing lock, ipc-serve acquires the lock and starts successfully",
      "Given a process holding the lock that exits, a subsequent startup acquires the lock",
    ]

    // Required error path tests
    required_error_tests: [
      "Given another process holding the exclusive lock, startup fails with an error containing the holding PID",
      "Given a read-only data directory where the lock file cannot be created, startup fails with a filesystem error",
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