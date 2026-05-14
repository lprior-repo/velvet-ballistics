
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260509135054-xpf2r3os
// Title: cli: Implement verify hero command and VerificationReport certificates
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260509135054-xpf2r3os.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260509135054-xpf2r3os"
  title: "cli: Implement verify hero command and VerificationReport certificates"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "Existing verifier/admission/storage gates are discoverable",
      "CLI structured output contract is understood",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "verify emits VerificationReport in text and machine-readable modes",
      "strict profile fails closed on missing durability or replay evidence",
      "repair hints cite concrete failing gates and bead evidence where available",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Stable exit codes do not drift across output formats",
      "Human text and structured output describe the same failure set",
      "No raw panic or stack trace reaches the operator",
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
      "verify succeeds for a fixture with valid source IR generated code digest and replay evidence",
      "verify --format json emits a parseable VerificationReport with certificate identifiers",
    ]

    // Required error path tests
    required_error_tests: [
      "verify fails closed when durability evidence is missing",
      "verify returns the documented nonzero exit code when replay evidence fails",
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