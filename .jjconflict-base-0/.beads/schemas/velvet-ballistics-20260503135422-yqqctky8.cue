
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260503135422-yqqctky8
// Title: CLI: Add --emit yaml and --emit postcard output formats
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260503135422-yqqctky8.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260503135422-yqqctky8"
  title: "CLI: Add --emit yaml and --emit postcard output formats"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "OutputFormat enum exists in crates/velvet_ballistics/src/main.rs with Text, Json, Jsonl variants",
      "parse_output_format function currently checks --jsonl and --json flags",
      "json_out and json_error helpers produce structured JSON output",
      "postcard crate is already a dependency for IR serialization",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "EmitFormat enum replaces OutputFormat with Text, Yaml, Postcard variants",
      "emit_yaml_value helper writes YAML manually without a YAML crate dependency",
      "All reporting commands (validate, compile, run, run-compiled, inspect, events, replay, bench-run, doctor) accept --emit flag",
      "schema_version and kind fields appear in every YAML output document",
      "Existing --json/--jsonl behavior is preserved",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "No unsafe, unwrap, expect, panic, todo, unimplemented, or dbg in new code",
      "No new serde dependency added for YAML serialization",
      "Source lint tolerance is zero",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(7)
    error_path_tests: [...string] & list.MinItems(3)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "validate --emit yaml produces YAML with schema_version and kind headers",
      "compile --emit yaml --emit ir --out /tmp/out.vbir produces YAML status output",
      "events --emit yaml --db <path> produces YAML event list",
      "inspect --emit postcard --db <path> produces valid postcard binary",
      "replay --emit postcard --db <path> produces valid postcard binary",
      "--json flag still produces JSON identical to current output",
      "--jsonl flag still produces JSONL identical to current output",
    ]

    // Required error path tests
    required_error_tests: [
      "--emit invalid-format reports error with valid format list",
      "YAML output for a failed compilation includes error details in kind: error document",
      "Postcard output for empty event list is valid postcard empty sequence",
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