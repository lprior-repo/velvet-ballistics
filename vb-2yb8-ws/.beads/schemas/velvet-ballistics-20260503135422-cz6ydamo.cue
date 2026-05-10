
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260503135422-cz6ydamo
// Title: CLI: graph command
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260503135422-cz6ydamo.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260503135422-cz6ydamo"
  title: "CLI: graph command"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "The workflow file at the given path is readable and valid YAML.",
      "The vb_compile crate can produce a CompiledWorkflow with node and edge information.",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "A WorkflowGraph is emitted containing all steps as nodes and all control flow transitions as edges.",
      "Every node's step_idx corresponds to its position in the compiled step list.",
      "Edge kinds correctly classify sequential, branch, merge, and error-routing transitions.",
      "Exit code is 0 on success, non-zero on failure.",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Graph MUST be derived from compiled IR, not raw YAML.",
      "Every compiled step MUST appear as exactly one node.",
      "Edge set MUST be complete: no step transition is omitted.",
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
      "Linear workflow with 3 sequential steps produces 3 nodes and 2 sequential edges.",
      "Workflow with a branch step produces nodes with branch edge kinds.",
      "Workflow with error routing produces edges with error-routing kind.",
    ]

    // Required error path tests
    required_error_tests: [
      "Invalid YAML produces compilation error with non-zero exit code.",
      "Workflow with unresolvable references produces descriptive error.",
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