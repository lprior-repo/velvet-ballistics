
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260509131811-smco6wmz
// Title: runtime/storage: Per-primitive durability proof matrix
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260509131811-smco6wmz.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260509131811-smco6wmz"
  title: "runtime/storage: Per-primitive durability proof matrix"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "Journal events and shard commands are discoverable in vb_storage, vb_runtime, and vb_core",
      "Master-doc primitive list is treated as authoritative",
      "No primitive is marked proven based on documentation alone",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "Each primitive has a row naming event type, storage partition, ack point, replay assertion, and test evidence",
      "Missing evidence is converted into follow-up beads or failing tests",
      "The matrix is wired into the release durability checker or verifier gate",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "A primitive cannot be checked off without at least one real test or executable verifier assertion",
      "Ack-after-persist ordering is explicit for every mutation path",
      "Replay from journal produces the same externally visible state as the original execution",
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
      "Matrix row for run acceptance links RunAccepted persistence to replayed active state",
      "Matrix row for timer firing links TimerFired persistence to deterministic resume state",
    ]

    // Required error path tests
    required_error_tests: [
      "Gate fails when a primitive row omits replay evidence",
      "Gate fails when a primitive row claims ack-before-persist or has no event mapping",
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