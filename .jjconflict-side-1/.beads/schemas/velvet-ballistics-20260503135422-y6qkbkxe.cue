
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260503135422-y6qkbkxe
// Title: CLI: ai context command -- compact redacted packet with suggested next commands
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260503135422-y6qkbkxe.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260503135422-y6qkbkxe"
  title: "CLI: ai context command -- compact redacted packet with suggested next commands"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "RunId corresponds to a run with at least one journal event",
      "TraceRing contains events for the run (may be empty if already drained)",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "All SlotValues with Taint::Secret or Taint::DerivedFromSecret are replaced with [REDACTED]",
      "Packet includes workflow digest, run_id, event count, and failure code if terminal",
      "Packet includes at least one suggested next command",
      "Packet is valid UTF-8 and compact enough to fit in a typical AI context window",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "No raw secret value is ever included in the output",
      "Taint metadata is preserved (redacted value shows the taint level but not the data)",
      "Suggested commands reference only CLI commands that exist",
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
      "Given a run with a Secret-tainted slot, ai-context output contains [REDACTED] in place of the value",
      "Given a failed run, ai-context suggests 'incident <run_id>' as a next command",
    ]

    // Required error path tests
    required_error_tests: [
      "Given an invalid run_id, ai-context exits with a RUN_NOT_FOUND error",
      "CLI: ai context command -- compact redacted packet with suggested next commands reports a clear diagnostic when invoked with invalid arguments",
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