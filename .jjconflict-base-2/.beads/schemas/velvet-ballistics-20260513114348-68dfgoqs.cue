
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260513114348-68dfgoqs
// Title: engine: validate durable YAML runtime acceptance graph
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260513114348-68dfgoqs.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260513114348-68dfgoqs"
  title: "engine: validate durable YAML runtime acceptance graph"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "Bead graph contains existing implementation beads for YAML semantics, admission, recovery, boundedness, CLI, IPC, and quality evidence.",
      "Master document remains authoritative for runtime no-YAML/no-JSON/no-HTTP and Fjall/Postcard durability constraints.",
      "Existing unrelated Rust worktree changes are not modified by dependency-graph planning.",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "vb-engine-yaml depends on all non-UI non-codegen engine blockers needed for durable YAML runtime acceptance.",
      "Previously false-closed parent beads are reopened when they still have open implementation children.",
      "Dependency graph has no cycles and bd ready exposes the next implementable beads.",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Runtime acceptance never depends on raw YAML interpretation during execution.",
      "Every durable acceptance path includes typed failure propagation, fail-closed recovery, and direct command evidence.",
      "Codegen/UI beads remain outside the no-codegen/no-UI acceptance root unless the user changes scope.",
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
      "bd show vb-engine-yaml lists YAML semantics, recovery, admission, boundedness, CLI, IPC, and quality evidence dependencies.",
      "bd dep cycles returns no dependency cycles.",
      "bd ready exposes implementable next work after false-closed parents are reopened.",
    ]

    // Required error path tests
    required_error_tests: [
      "A closed parent with open required children is treated as dependency drift and reopened.",
      "A dependency path that pulls UI/codegen/maxperf into vb-engine-yaml is rejected or split out of scope.",
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
//     timestamp: "2026-05-13T11:43:48Z"
//   }
// }