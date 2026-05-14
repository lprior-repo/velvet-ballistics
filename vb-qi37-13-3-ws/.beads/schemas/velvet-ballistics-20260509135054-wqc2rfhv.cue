
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260509135054-wqc2rfhv
// Title: ui-model: Enforce artifact schema bounds and CLI parity
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260509135054-wqc2rfhv.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260509135054-wqc2rfhv"
  title: "ui-model: Enforce artifact schema bounds and CLI parity"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "vb_ui_model source has been read",
      "Master artifact schema requirements are extracted",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "All exported UI artifacts carry universal metadata fields",
      "Unbounded Vec fields are replaced or guarded by bounded typed collections where required",
      "CLI/UI parity tests prove equivalent artifacts",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "UI model types remain cold-path only",
      "Makepad crates do not depend on runtime core internals",
      "Redaction status is explicit rather than inferred from absent data",
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
      "workflow summary artifact serializes with all universal schema fields",
      "CLI emitted artifact and UI model artifact compare equal after canonicalization",
    ]

    // Required error path tests
    required_error_tests: [
      "oversized event table artifact is rejected or clipped with explicit truncation metadata",
      "redacted value cannot be serialized as raw secret text",
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