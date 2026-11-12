
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260509135054-qsjzhsea
// Title: cli: Add runtime ai context packet command
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260509135054-qsjzhsea.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260509135054-qsjzhsea"
  title: "cli: Add runtime ai context packet command"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "Run inspection and artifact schema surfaces are known",
      "Redaction model is understood",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "AI context command emits structured and text output",
      "Packet includes citations to run artifacts and diagnostic sources",
      "Secrets are redacted with explicit redaction_status",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "AI context is read-only",
      "Packet size is bounded",
      "Every recommendation maps to a real CLI command or artifact",
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
      "ai context for a valid run includes state diagnostics and next commands",
      "json output includes citations and redaction status",
    ]

    // Required error path tests
    required_error_tests: [
      "unknown run returns structured not-found diagnostic",
      "tainted payload is redacted in context packet",
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