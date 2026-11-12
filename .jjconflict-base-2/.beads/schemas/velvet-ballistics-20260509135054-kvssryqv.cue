
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260509135054-kvssryqv
// Title: quality: Enforce CLI UI typed artifact parity matrix
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260509135054-kvssryqv.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260509135054-kvssryqv"
  title: "quality: Enforce CLI UI typed artifact parity matrix"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "CLI output schemas and UI model schemas are discoverable",
      "Current artifact list is extracted from master doc",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "Parity matrix exists as data or tests",
      "Gate fails on missing side of the matrix",
      "Evidence names the artifact and missing surface",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Schema version and kind agree across CLI and UI",
      "Redaction semantics are identical across surfaces",
      "Parity checks are deterministic and source-controlled",
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
      "verification certificate artifact appears in CLI and UI parity matrix",
      "run incident artifact round-trips through UI model and CLI JSON",
    ]

    // Required error path tests
    required_error_tests: [
      "missing UI consumer for a CLI artifact fails the gate",
      "schema version mismatch fails with actionable diagnostic",
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
//     timestamp: "2026-05-09T13:50:54Z"
//   }
// }