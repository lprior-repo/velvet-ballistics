
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260503135422-ddconglm
// Title: Storage: VerificationWarning type for admission gates
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260503135422-ddconglm.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260503135422-ddconglm"
  title: "Storage: VerificationWarning type for admission gates"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "VerificationProof and AcceptedArtifact types already exist in crates/vb_storage/src/admission.rs.",
      "The gate constants (ADMISSION_GATE_COUNT = 2) and gate indices are stable.",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "AcceptedArtifact has a new warnings: Vec<VerificationWarning> field.",
      "Existing submit_artifact and admit_compiled_artifact calls compile without behavioral change (warnings Vec is empty).",
      "VerificationWarning is serializable via postcard/serde.",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "warnings.len() must never exceed ADMISSION_GATE_COUNT.",
      "Each warning.gate must be a valid gate index (< ADMISSION_GATE_COUNT).",
      "warning.code must be nonzero (0 reserved for no-warning sentinel).",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(3)
    error_path_tests: [...string] & list.MinItems(2)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "VerificationWarning::new(1, \"soft issue\", 0) produces a warning with the correct fields.",
      "AcceptedArtifact produced by Journaled policy with no soft issues has empty warnings.",
      "Serialization round-trip: postcard serialize then deserialize preserves all fields.",
    ]

    // Required error path tests
    required_error_tests: [
      "AcceptedArtifact produced by Relaxed policy has zero-length warnings.",
      "VerificationWarning with gate >= ADMISSION_GATE_COUNT does not panic (caller responsibility).",
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
//     timestamp: "2026-05-03T13:54:22Z"
//   }
// }