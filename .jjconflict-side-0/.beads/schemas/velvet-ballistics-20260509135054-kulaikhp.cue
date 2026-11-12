
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260509135054-kulaikhp
// Title: makepad: Implement Splash AppShell contract
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260509135054-kulaikhp.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260509135054-kulaikhp"
  title: "makepad: Implement Splash AppShell contract"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "Makepad skill patterns and existing vb_ui_makepad shell code have been read",
      "UI model artifacts for shell state are understood",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "AppShell boot path exists and renders shell chrome",
      "Route changes are represented as typed UI actions",
      "Rendering uses model artifacts and testable widget state",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "UI actions do not mutate durable runtime state directly",
      "AppShell can render with fixture data",
      "Makepad-specific code remains outside runtime core crates",
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
      "AppShell fixture renders sidebar topbar and routed content",
      "route action switches from overview to workflow graph fixture",
    ]

    // Required error path tests
    required_error_tests: [
      "invalid route falls back to structured empty/error screen",
      "missing fixture data renders a diagnostic state instead of panicking",
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