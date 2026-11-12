
package validation

import "list"

// Validation schema for bead: velvet-ballistics-20260503135422-jlpeurjj
// Title: CLI: verify command with profiles and certificates
//
// This schema validates that implementation is complete.
// Use: cue vet velvet-ballistics-20260503135422-jlpeurjj.cue implementation.cue

#BeadImplementation: {
  bead_id: "velvet-ballistics-20260503135422-jlpeurjj"
  title: "CLI: verify command with profiles and certificates"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "vb_compile::compile_workflow produces compilation errors as CompileErrorList with Vec<CompileError>",
      "vb_validate gates are organized in crates/vb_validate/src/ (gates.rs, schema.rs, references.rs, control_flow.rs, type_check.rs, taint_prop.rs, etc.)",
      "vb_core::budget::WholeWorkflowBudget::compute produces a budget from compiled nodes",
      "vb_core::budget::BoundednessPolicy::validate checks budget against policy limits",
      "vb_core::action::verify_idempotency checks action idempotency contracts",
      "vb_core::action::validate_action_dispatch validates action dispatch legality",
      "ResourceContract in vb_core::workflow defines resource limits per workflow",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "VerificationReport struct contains certificates for structural, boundedness, resources, taint, actions, and durability domains",
      "Each certificate has: domain name, status (pass/fail/skipped), findings list",
      "Each finding has: severity, code, message, optional location",
      "--profile flag accepts quick, standard, exhaustive with documented gate coverage per profile",
      "VerificationReport is serializable to JSON, YAML (manual), and Postcard",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "No unsafe, unwrap, expect, panic, todo, unimplemented, or dbg",
      "No YAML, JSON, or HTTP in runtime core (YAML writing is CLI-layer only)",
      "Zero tolerance for source lint",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(6)
    error_path_tests: [...string] & list.MinItems(4)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "verify --profile quick on a minimal valid workflow produces a passing VerificationReport with structural and budget certificates",
      "verify --profile standard on a valid workflow produces all standard certificates with pass status",
      "verify --profile exhaustive on a valid workflow produces all certificates including durability simulation",
      "verify on a workflow with budget violation produces boundedness certificate with fail status and findings",
      "verify on a workflow with type errors produces structural certificate with type-check findings",
      "verify --emit yaml produces YAML output with schema_version and kind headers",
    ]

    // Required error path tests
    required_error_tests: [
      "verify on a file that does not exist returns storage error exit code",
      "verify on invalid UTF-8 returns validation error exit code",
      "verify on YAML that fails parsing returns structural certificate with parse error findings",
      "verify on workflow that fails compilation returns compilation certificate with all compile errors as findings",
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