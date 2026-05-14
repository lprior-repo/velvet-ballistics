
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260503135422-n2nxpqma
// Title: CLI: Define stable exit code constants
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260503135422-n2nxpqma.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260503135422-n2nxpqma"
  title: "CLI: Define stable exit code constants"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "CoreError enum in crates/vb_core/src/errors.rs has 30+ variants with diagnostic codes",
      "ActionError enum in crates/vb_core/src/action.rs has 9 variants",
      "main.rs currently uses ExitCode::SUCCESS, ExitCode::FAILURE, and ExitCode::from(2)",
      "setup_exit_code function at line 724 returns ExitCode::from(2) for setup errors",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "ExitCode enum defined in exit_code.rs with named variants for codes 0-8",
      "From<CoreError> for ExitCode maps each CoreError variant to the appropriate domain code",
      "From<ActionError> for ExitCode maps ActionError variants to action policy or runtime codes",
      "All cmd_* functions return typed exit codes instead of raw ExitCode::FAILURE",
      "HELP string documents exit codes",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "No unsafe, unwrap, expect, panic, todo, unimplemented, or dbg",
      "Exit code assignments are stable and documented",
      "Zero tolerance for source lint",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(9)
    error_path_tests: [...string] & list.MinItems(4)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "cmd_validate returns exit code 0 for valid workflow",
      "cmd_validate returns exit code 1 for YAML parse failure",
      "cmd_validate returns exit code 3 for compilation failure",
      "cmd_compile returns exit code 0 on successful compilation",
      "cmd_compile returns exit code 3 on compilation error",
      "cmd_run returns exit code 4 on runtime tick failure",
      "cmd_inspect returns exit code 5 on journal open failure",
      "cmd_ipc_serve returns exit code 6 on socket bind failure",
      "cmd_replay returns exit code 5 on journal read failure",
    ]

    // Required error path tests
    required_error_tests: [
      "Unknown command returns exit code 1 (validation analog)",
      "Missing required argument returns exit code 1",
      "Malformed input-bin decode returns exit code 4 (runtime)",
      "IR deserialization failure in run-compiled returns exit code 3",
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