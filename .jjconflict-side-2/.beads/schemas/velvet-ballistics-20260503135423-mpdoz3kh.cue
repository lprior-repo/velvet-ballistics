
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260503135423-mpdoz3kh
// Title: Build: PGO and maxperf justfile targets
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260503135423-mpdoz3kh.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260503135423-mpdoz3kh"
  title: "Build: PGO and maxperf justfile targets"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "justfile already has pgo-instrument-build and pgo-optimized-build targets.",
      "justfile already has maxperf and maxperf-native targets.",
      "Cargo.toml has [profile.maxperf] with inherits = 'release', lto = 'fat', codegen-units = 1, debug = false.",
      "The nightly toolchain is configured in the justfile as nightly-2026-04-28.",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "'just pgo' runs the complete PGO pipeline end-to-end.",
      "'just maxperf-release' produces a binary with maxperf profile.",
      "Existing justfile targets (maxperf, maxperf-native) continue to work.",
      "moon ci is unaffected by the new targets.",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "PGO profile data must be stored under target/pgo/.",
      "The maxperf profile must not enable debug assertions or overflow checks.",
      "The nightly toolchain variable must be used consistently across all targets.",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(4)
    error_path_tests: [...string] & list.MinItems(2)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "'just pgo-instrument-build' compiles successfully with profile generation flags.",
      "'just pgo-optimized-build' compiles successfully with merged profile data.",
      "'just maxperf-release' compiles successfully with fat LTO.",
      "'just maxperf-native' compiles with target-cpu=native.",
    ]

    // Required error path tests
    required_error_tests: [
      "PGO pipeline fails gracefully if no profile data is generated (missing workload run).",
      "maxperf build reports clear error if nightly toolchain is not installed.",
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