
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260503135423-avzx72iz
// Title: Core: Feature flags for generated/bench/volatile
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260503135423-avzx72iz.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260503135423-avzx72iz"
  title: "Core: Feature flags for generated/bench/volatile"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "crates/vb_core/Cargo.toml currently has [features] with default = [] only.",
      "No downstream crate currently uses non-default features from vb_core.",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "vb_core/Cargo.toml has generated, bench, and volatile features defined.",
      "cargo check -p vb_core succeeds with no features, each feature individually, and all features.",
      "cargo check --workspace --all-features succeeds.",
      "The 'default' feature set is unchanged (empty).",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Features must not add new required dependencies.",
      "Features must be purely additive (no feature-gated removal of existing functionality).",
      "Feature names must be lowercase with hyphens.",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(5)
    error_path_tests: [...string] & list.MinItems(2)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "cargo check -p vb_core --features generated succeeds.",
      "cargo check -p vb_core --features bench succeeds.",
      "cargo check -p vb_core --features volatile succeeds.",
      "cargo check -p vb_core --all-features succeeds.",
      "cargo check -p vb_core (no features) succeeds and produces same output as before.",
    ]

    // Required error path tests
    required_error_tests: [
      "cargo check --workspace --all-features succeeds (no feature combination breaks compilation).",
      "Core: Feature flags for generated/bench/volatile reports a clear diagnostic when invoked with invalid arguments",
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
//     timestamp: "2026-05-03T13:54:23Z"
//   }
// }