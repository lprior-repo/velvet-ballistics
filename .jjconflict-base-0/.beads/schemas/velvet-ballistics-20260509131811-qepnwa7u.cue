
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260509131811-qepnwa7u
// Title: verifier/storage: Durability gate for accepted artifacts
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260509131811-qepnwa7u.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260509131811-qepnwa7u"
  title: "verifier/storage: Durability gate for accepted artifacts"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "Artifact storage and digest verification paths exist in vb_storage",
      "Verifier or validation crate can surface release gate diagnostics",
      "Accepted artifact definition comes from master doc, not ad hoc naming",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "Verifier reports each required artifact evidence item by name",
      "Digest mismatch or missing record fails with typed diagnostics",
      "Passing gate records enough evidence for CI and operator review",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Accepted artifact digest equals stored payload digest",
      "Generated Rust evidence and compiled IR evidence refer to the same workflow source identity",
      "Durability gate does not mutate artifacts while checking them",
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
      "Complete accepted artifact evidence set passes the durability gate",
      "Gate report lists source, IR, generated Rust, schema, and replay evidence as present",
    ]

    // Required error path tests
    required_error_tests: [
      "Missing generated Rust evidence fails the gate with an exact diagnostic",
      "Digest mismatch between artifact record and payload fails without marking accepted",
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