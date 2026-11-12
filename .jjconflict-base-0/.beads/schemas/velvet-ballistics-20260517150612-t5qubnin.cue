
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260517150612-t5qubnin
// Title: bdd: Executable behavior catalog and scenario matrix
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260517150612-t5qubnin.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260517150612-t5qubnin"
  title: "bdd: Executable behavior catalog and scenario matrix"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "Master document behavior has been read",
      "Existing tests and fixtures have been inventoried",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "Scenario group has executable tests",
      "Each test has scenario id and evidence output",
      "Missing implementation gaps are linked to existing beads",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "No private helper is the primary behavior surface",
      "Every error scenario asserts exact diagnostic or typed error",
      "No mutable state is shared between scenarios",
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
      "catalog_lists_every_master_doc_behavior_by_scenario_id",
      "catalog_maps_existing_tests_to_covered_scenarios",
    ]

    // Required error path tests
    required_error_tests: [
      "catalog_gate_fails_when_behavior_has_no_scenario",
      "catalog_gate_fails_when_scenario_has_no_test_target",
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
//     timestamp: "2026-05-17T15:06:12Z"
//   }
// }