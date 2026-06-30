
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260517150454-5jzaki7c
// Title: bdd: Observability and evidence packaging acceptance scenarios
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260517150454-5jzaki7c.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260517150454-5jzaki7c"
  title: "bdd: Observability and evidence packaging acceptance scenarios"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "Existing repository builds",
      "Scenario fixtures are isolated per test",
      "No private implementation API is used as the primary behavior surface",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "Each scenario has explicit Given, When, Then assertions",
      "Runner output identifies exact failing scenario and observable mismatch",
      "Evidence artifacts can be traced to the related bead and master-doc section",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Scenarios are deterministic and independently runnable",
      "Assertions check exact values or exact typed errors",
      "No mock replaces Fjall, IPC, CLI, or direct API internals when real local dependencies are available",
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
      "evidence_bundle_contains_every_phase_command_and_artifact_hash",
      "trace_events_link_to_run_step_and_correlation_id",
    ]

    // Required error path tests
    required_error_tests: [
      "incident_report_redacts_secret_tainted_details",
      "evidence_packaging_fails_when_required_raw_log_missing",
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
//     timestamp: "2026-05-17T15:04:54Z"
//   }
// }