
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260503135422-khzxkk1f
// Title: Codegen: Emit CollectStart/Page/Next/Finish
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260503135422-khzxkk1f.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260503135422-khzxkk1f"
  title: "Codegen: Emit CollectStart/Page/Next/Finish"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "CompiledNodeKind::CollectStart { source, limit, page_size, body, done, output, time_limit_ms } fields are available.",
      "CompiledNodeKind::CollectPage { collector_slot, body, done } fields are available.",
      "CompiledNodeKind::CollectNext { collector_slot, body, done } fields are available.",
      "CompiledNodeKind::CollectFinish { collector_slot, output, next } fields are available.",
      "A CollectStates side table is accessible to the generated code for pagination state.",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "Generated Rust for Collect compiles without errors.",
      "Generated Collect produces identical pagination results to the interpreter for the same inputs.",
      "The emit_unsupported_node_step match arms for CollectStart/Page/Next/Finish are removed.",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "page_size must be nonzero (enforced at CollectStart).",
      "page_size must not exceed limit (enforced at CollectStart).",
      "item_count must not exceed limit (enforced at CollectStart).",
      "Pagination state must be cleaned up on CollectFinish or when jumping to done.",
      "Time limit enforcement must check elapsed time against time_limit_ms on each CollectNext.",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(4)
    error_path_tests: [...string] & list.MinItems(4)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "Generated CollectStart with a 5-item source, page_size=2, limit=100 writes first page [0,1] and jumps to body.",
      "Generated CollectNext advances to page [2,3] and then [4], then jumps to done.",
      "Generated CollectFinish writes the final collected value to output.",
      "Equivalence test: 5-item list, page_size=2 produces same pages from interpreter and generated code.",
    ]

    // Required error path tests
    required_error_tests: [
      "Generated CollectStart with non-list source returns TypeMismatch.",
      "Generated CollectStart with page_size=0 returns InvalidCompiledWorkflow.",
      "Generated CollectStart with item_count > limit returns CollectItemLimitExceeded.",
      "Generated CollectNext without pagination state returns InvalidCompiledWorkflow.",
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