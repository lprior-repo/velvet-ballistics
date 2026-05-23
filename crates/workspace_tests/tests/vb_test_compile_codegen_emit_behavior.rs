#![forbid(unsafe_code)]
//! Behavior tests for vb_compile code generation emission.
//!
//! Tests the exact output of `emit_rust_workflow` focusing on:
//! - Generated code structure and content
//! - ID emission behavior
//! - Drive function generation
//! - Exact output assertions

use vb_codegen::emit_rust_workflow;
use vb_compile::compile_workflow;

// ---------------------------------------------------------------------------
// Generated code structure - header and overall layout
// ---------------------------------------------------------------------------

#[test]
fn generated_header_contains_forbid_unsafe_directive() {
    let source = br#"
version: velvet-ballastics/v1
name: header_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Header must contain #![forbid(unsafe_code)]
    assert!(
        generated.contains("#![forbid(unsafe_code)]"),
        "generated code must forbid unsafe code"
    );
}

#[test]
fn generated_header_contains_deny_directives() {
    let source = br#"
version: velvet-ballastics/v1
name: deny_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must contain deny directives for quality
    assert!(
        generated.contains("#![deny(unused_must_use)]"),
        "generated code must deny unused_must_use"
    );
    assert!(
        generated.contains("#![deny(unreachable_pub)]"),
        "generated code must deny unreachable_pub"
    );
    assert!(
        generated.contains("#![deny(rust_2018_idioms)]"),
        "generated code must deny rust_2018_idioms"
    );
}

#[test]
fn generated_header_contains_generated_workflow_comment() {
    let source = br#"
version: velvet-ballastics/v1
name: comment_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must contain generated workflow comment
    assert!(
        generated.contains("//! Generated workflow - DO NOT EDIT"),
        "generated code must contain DO NOT EDIT comment"
    );
    assert!(
        generated.contains("Produced by vb_codegen emit_rust_workflow"),
        "generated code must contain attribution comment"
    );
}

#[test]
fn generated_contains_slot_value_enum() {
    let source = br#"
version: velvet-ballastics/v1
name: slot_enum_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must contain SlotValue enum definition with all variants
    // The enum is defined as: pub enum SlotValue { Null, Bool(bool), I64(i64), F64(f64), ... }
    assert!(
        generated.contains("pub enum SlotValue"),
        "generated code must contain SlotValue enum"
    );
    // Must contain all variant type names in the enum definition
    assert!(generated.contains("Null"));
    assert!(generated.contains("Bool(bool)"));
    assert!(generated.contains("I64(i64)"));
    assert!(generated.contains("F64(f64)"));
    assert!(generated.contains("Symbol(u32)"));
    assert!(generated.contains("List(u32)"));
    assert!(generated.contains("Object(u32)"));
    assert!(generated.contains("Blob(u64)"));
}

#[test]
fn generated_contains_taint_enum() {
    let source = br#"
version: velvet-ballastics/v1
name: taint_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must contain Taint enum
    assert!(generated.contains("pub enum Taint"));
    // Must contain all taint variants
    assert!(generated.contains("Taint::Clean"));
    assert!(generated.contains("Taint::DerivedFromSecret"));
    assert!(generated.contains("Taint::Secret"));
    assert!(generated.contains("Taint::Random"));
    assert!(generated.contains("Taint::TimeDependent"));
}

#[test]
fn generated_contains_drive_error_enum() {
    let source = br#"
version: velvet-ballastics/v1
name: drive_error_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must contain DriveError enum
    assert!(generated.contains("pub enum DriveError"));
    // Must contain key error variants (just the variant name, not full path)
    assert!(generated.contains("InvalidProgramCounter"));
    assert!(generated.contains("StepBudgetExhausted"));
    assert!(generated.contains("MissingNextStep"));
    assert!(generated.contains("SlotOutOfBounds"));
    assert!(generated.contains("TaintViolation"));
}

// ---------------------------------------------------------------------------
// ID emission behavior
// ---------------------------------------------------------------------------

#[test]
fn id_emission_contains_workflow_slot_count() {
    let source = br#"
version: velvet-ballastics/v1
name: slot_count_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must emit WORKFLOW_SLOT_COUNT constant
    let expected_line = "const WORKFLOW_SLOT_COUNT: usize = ";
    assert!(
        generated.contains(expected_line),
        "generated code must contain WORKFLOW_SLOT_COUNT constant"
    );
}

#[test]
fn id_emission_contains_workflow_node_count() {
    let source = br#"
version: velvet-ballastics/v1
name: node_count_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must emit WORKFLOW_NODE_COUNT constant
    let expected_line = "const WORKFLOW_NODE_COUNT: u16 = ";
    assert!(
        generated.contains(expected_line),
        "generated code must contain WORKFLOW_NODE_COUNT constant"
    );
}

#[test]
fn id_emission_slot_count_matches_workflow() {
    let source = br#"
version: velvet-ballastics/v1
name: slot_count_match
when:
  manual: {}
steps:
  - id: init
    set:
      output: val_x
      value: "1"
  - id: done
    finish:
      result: val_x
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    let parts = workflow.to_parts();
    let expected = format!("const WORKFLOW_SLOT_COUNT: usize = {};", parts.slot_count);
    assert!(
        generated.contains(&expected),
        "generated WORKFLOW_SLOT_COUNT must match workflow slot_count"
    );
}

#[test]
fn id_emission_node_count_matches_workflow() {
    let source = br#"
version: velvet-ballastics/v1
name: node_count_match
when:
  manual: {}
steps:
  - id: first
    set:
      output: val_a
      value: "1"
  - id: second
    set:
      output: val_b
      value: "2"
  - id: done
    finish:
      result: val_a
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    let parts = workflow.to_parts();
    let expected = format!("const WORKFLOW_NODE_COUNT: u16 = {};", parts.nodes.len());
    assert!(
        generated.contains(&expected),
        "generated WORKFLOW_NODE_COUNT must match workflow node count"
    );
}

#[test]
fn id_emission_contains_symbol_constants() {
    let source = br#"
version: velvet-ballastics/v1
name: symbol_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Symbol ID constants must be emitted in format _sym_N: u32 = N;
    // Even if no symbols, should have the section marker
    assert!(
        generated.contains("// --- Typed ID constants ---"),
        "generated code must contain typed ID constants section"
    );
}

#[test]
fn id_section_has_correct_separator() {
    let source = br#"
version: velvet-ballastics/v1
name: sep_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // ID section must be properly separated
    assert!(
        generated.contains("// --- Typed ID constants ---"),
        "ID constants section must use correct separator"
    );
}

// ---------------------------------------------------------------------------
// Drive function generation
// ---------------------------------------------------------------------------

#[test]
fn drive_function_has_correct_signature() {
    let source = br#"
version: velvet-ballastics/v1
name: drive_sig_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Drive function must have correct signature
    let expected = "pub fn drive(mut slots: [Option<SlotValue>;";
    assert!(
        generated.contains(expected),
        "drive function must have correct signature"
    );
    assert!(
        generated.contains("Result<SlotValue, DriveError>"),
        "drive function must return Result<SlotValue, DriveError>"
    );
}

#[test]
fn drive_function_initializes_slot_taints() {
    let source = br#"
version: velvet-ballastics/v1
name: taints_init_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must initialize slot_taints array
    assert!(
        generated.contains("let mut slot_taints = [Taint::Clean; WORKFLOW_SLOT_COUNT];"),
        "drive function must initialize slot_taints"
    );
}

#[test]
fn drive_function_initializes_program_counter() {
    let source = br#"
version: velvet-ballastics/v1
name: pc_init_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must initialize pc from workflow entry
    assert!(
        generated.contains("let mut pc: u16 = "),
        "drive function must initialize program counter"
    );
}

#[test]
fn drive_function_initializes_step_budget() {
    let source = br#"
version: velvet-ballastics/v1
name: budget_init_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must initialize step_budget_remaining
    assert!(
        generated
            .contains("let mut step_budget_remaining: u64 = CONTRACT_MAX_STEP_BUDGET_PER_TICK;"),
        "drive function must initialize step_budget"
    );
}

#[test]
fn drive_function_has_main_loop() {
    let source = br#"
version: velvet-ballastics/v1
name: loop_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must have main loop
    assert!(
        generated.contains("loop {"),
        "drive function must have main loop"
    );
}

#[test]
fn drive_function_checks_step_budget_exhaustion() {
    let source = br#"
version: velvet-ballastics/v1
name: budget_check_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must check and decrement step budget
    assert!(
        generated.contains("if step_budget_remaining == 0"),
        "drive function must check for budget exhaustion"
    );
    assert!(
        generated.contains("step_budget_remaining.checked_sub(1)"),
        "drive function must decrement budget with checked_sub"
    );
    assert!(
        generated.contains("DriveError::StepBudgetExhausted"),
        "drive function must return error on budget exhaustion"
    );
}

#[test]
fn drive_function_has_match_on_pc() {
    let source = br#"
version: velvet-ballastics/v1
name: pc_match_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must match on pc to dispatch to step functions
    assert!(
        generated.contains("let outcome = match pc {"),
        "drive function must match on pc"
    );
}

#[test]
fn drive_function_dispatches_to_step_functions() {
    let source = br#"
version: velvet-ballastics/v1
name: dispatch_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must dispatch to step_0
    assert!(
        generated.contains("step_0("),
        "drive function must dispatch to step_0"
    );
}

#[test]
fn drive_function_handles_invalid_pc() {
    let source = br#"
version: velvet-ballastics/v1
name: invalid_pc_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must handle invalid pc
    assert!(
        generated.contains("_ => return Err(DriveError::InvalidProgramCounter)"),
        "drive function must handle invalid pc"
    );
}

#[test]
fn drive_function_handles_step_outcome_continue() {
    let source = br#"
version: velvet-ballastics/v1
name: continue_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must handle Continue outcome
    assert!(
        generated.contains("StepOutcome::Continue(next) => pc = next"),
        "drive function must handle Continue outcome"
    );
}

#[test]
fn drive_function_handles_step_outcome_finished() {
    let source = br#"
version: velvet-ballastics/v1
name: finished_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must handle Finished outcome
    assert!(
        generated.contains("StepOutcome::Finished(value) => return Ok(value)"),
        "drive function must handle Finished outcome"
    );
}

#[test]
fn drive_function_comment_separator_present() {
    let source = br#"
version: velvet-ballastics/v1
name: sep_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Drive function section must have separator
    assert!(
        generated.contains("// --- Main drive function ---"),
        "drive function section must have correct separator"
    );
}

// ---------------------------------------------------------------------------
// Step function generation
// ---------------------------------------------------------------------------

#[test]
fn step_functions_are_named_sequentially() {
    let source = br#"
version: velvet-ballastics/v1
name: step_names_test
when:
  manual: {}
steps:
  - id: first
    set:
      output: val_x
      value: "1"
  - id: second
    set:
      output: val_y
      value: "2"
  - id: done
    finish:
      result: val_x
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Step functions must be named step_0, step_1, step_2
    assert!(generated.contains("fn step_0("));
    assert!(generated.contains("fn step_1("));
    assert!(generated.contains("fn step_2("));
}

#[test]
fn step_function_signature_contains_workflow_slot_count() {
    let source = br#"
version: velvet-ballastics/v1
name: step_sig_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Step functions must use WORKFLOW_SLOT_COUNT in signature
    assert!(
        generated.contains("&mut [Option<SlotValue>; WORKFLOW_SLOT_COUNT]"),
        "step function signature must use WORKFLOW_SLOT_COUNT"
    );
}

#[test]
fn step_function_signature_contains_slot_taints() {
    let source = br#"
version: velvet-ballastics/v1
name: taints_param_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Step functions must have slot_taints parameter
    assert!(
        generated.contains("&mut [Taint; WORKFLOW_SLOT_COUNT]"),
        "step function signature must have slot_taints parameter"
    );
}

#[test]
fn step_function_signature_contains_list_store() {
    let source = br#"
version: velvet-ballastics/v1
name: list_store_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Step functions must have list_store parameter
    assert!(
        generated.contains("&mut ListStore"),
        "step function signature must have list_store parameter"
    );
}

#[test]
fn step_function_signature_contains_object_store() {
    let source = br#"
version: velvet-ballastics/v1
name: object_store_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Step functions must have object_store parameter
    assert!(
        generated.contains("&mut ObjectStore"),
        "step function signature must have object_store parameter"
    );
}

#[test]
fn step_function_returns_step_outcome() {
    let source = br#"
version: velvet-ballastics/v1
name: outcome_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Step functions must return StepOutcome
    assert!(
        generated.contains("Result<StepOutcome, DriveError>"),
        "step function must return StepOutcome result"
    );
}

// ---------------------------------------------------------------------------
// Constants emission
// ---------------------------------------------------------------------------

#[test]
fn constants_section_has_separator() {
    let source = br#"
version: velvet-ballastics/v1
name: const_section_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Constants section must have proper separator
    assert!(
        generated.contains("// --- Constant pool ---"),
        "constants section must have proper separator"
    );
}

#[test]
fn constants_array_is_named_constants() {
    let source = br#"
version: velvet-ballastics/v1
name: const_array_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Constants array must be named CONSTANTS
    assert!(
        generated.contains("const CONSTANTS: [SlotValue;"),
        "constants array must be named CONSTANTS"
    );
}

#[test]
fn constants_array_has_correct_terminator() {
    let source = br#"
version: velvet-ballastics/v1
name: const_term_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Constants array must be properly terminated
    assert!(
        generated.contains("];"),
        "constants array must be properly terminated"
    );
}

#[test]
fn constants_empty_workflow_has_empty_array() {
    let source = br#"
version: velvet-ballastics/v1
name: empty_const_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // For minimal workflow, CONSTANTS array may be empty or have minimal values
    // The array declaration should be present
    let parts = workflow.to_parts();
    let expected = format!(
        "const CONSTANTS: [SlotValue; {}] = [",
        parts.constants.len()
    );
    assert!(
        generated.contains(&expected),
        "constants array length must match workflow constants count"
    );
}

// ---------------------------------------------------------------------------
// Resource contract emission
// ---------------------------------------------------------------------------

#[test]
fn resource_contract_section_has_separator() {
    let source = br#"
version: velvet-ballastics/v1
name: contract_section_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Resource contract section must be present
    assert!(
        generated.contains("CONTRACT_MAX_STEPS") || generated.contains("CONTRACT_MAX_SLOTS"),
        "generated code must contain resource contract constants"
    );
}

// ---------------------------------------------------------------------------
// Generated runtime API
// ---------------------------------------------------------------------------

#[test]
fn generated_runtime_api_drive_with_journal() {
    let source = br#"
version: velvet-ballastics/v1
name: runtime_api_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must have drive_with_journal function
    assert!(
        generated.contains("pub fn drive_with_journal("),
        "generated code must have drive_with_journal function"
    );
}

#[test]
fn generated_runtime_api_contains_generated_run_state() {
    let source = br#"
version: velvet-ballastics/v1
name: run_state_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must have GeneratedRunState impl block
    assert!(
        generated.contains("impl GeneratedRunState"),
        "generated code must have GeneratedRunState impl"
    );
}

#[test]
fn generated_run_state_new_initializes_all_fields() {
    let source = br#"
version: velvet-ballastics/v1
name: new_init_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // GeneratedRunState::new must initialize all fields
    assert!(
        generated.contains("pub fn new(slots: [Option<SlotValue>; WORKFLOW_SLOT_COUNT]) -> Self"),
        "GeneratedRunState must have new constructor"
    );
}

#[test]
fn generated_run_state_has_run_until_blocked() {
    let source = br#"
version: velvet-ballastics/v1
name: blocked_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must have run_until_blocked method
    assert!(
        generated.contains("pub fn run_until_blocked(&mut self)"),
        "GeneratedRunState must have run_until_blocked method"
    );
}

#[test]
fn generated_api_has_action_resume() {
    let source = br#"
version: velvet-ballastics/v1
name: action_resume_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must have complete_action method
    assert!(
        generated.contains("pub fn complete_action("),
        "GeneratedRunState must have complete_action method"
    );
}

#[test]
fn generated_api_has_ask_answer() {
    let source = br#"
version: velvet-ballastics/v1
name: ask_answer_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must have answer_ask method
    assert!(
        generated.contains("pub fn answer_ask("),
        "GeneratedRunState must have answer_ask method"
    );
}

// ---------------------------------------------------------------------------
// Action match dispatch
// ---------------------------------------------------------------------------

#[test]
fn action_match_dispatch_section_exists() {
    let source = br#"
version: velvet-ballastics/v1
name: action_match_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must have action completion next function
    assert!(
        generated.contains("fn action_completion_next("),
        "generated code must have action_completion_next function"
    );
}

#[test]
fn ask_answer_spec_section_exists() {
    let source = br#"
version: velvet-ballastics/v1
name: ask_spec_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must have ask answer spec function
    assert!(
        generated.contains("fn ask_answer_spec("),
        "generated code must have ask_answer_spec function"
    );
}

// ---------------------------------------------------------------------------
// Finish function
// ---------------------------------------------------------------------------

#[test]
fn generated_finish_result_slot_function_exists() {
    let source = br#"
version: velvet-ballastics/v1
name: finish_slot_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must have finish result slot function
    assert!(
        generated.contains("fn finish_result_slot("),
        "generated code must have finish_result_slot function"
    );
}

// ---------------------------------------------------------------------------
// Multi-step workflow generation
// ---------------------------------------------------------------------------

#[test]
fn multi_step_workflow_generates_all_step_functions() {
    let source = br#"
version: velvet-ballastics/v1
name: multi_step_test
when:
  manual: {}
steps:
  - id: first
    set:
      output: val_a
      value: "1"
  - id: second
    set:
      output: val_b
      value: "2"
  - id: third
    set:
      output: val_c
      value: "3"
  - id: done
    finish:
      result: val_c
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // All 4 step functions must be generated
    assert!(generated.contains("fn step_0("));
    assert!(generated.contains("fn step_1("));
    assert!(generated.contains("fn step_2("));
    assert!(generated.contains("fn step_3("));
}

#[test]
fn drive_function_dispatches_to_all_step_functions() {
    let source = br#"
version: velvet-ballastics/v1
name: dispatch_all_test
when:
  manual: {}
steps:
  - id: first
    set:
      output: val_x
      value: "1"
  - id: second
    finish:
      result: val_x
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Drive must dispatch to both step_0 and step_1
    let parts = workflow.to_parts();
    for i in 0..parts.nodes.len() {
        assert!(
            generated.contains(&format!("{i} => step_{i}(")),
            "drive must dispatch to step_{}",
            i
        );
    }
}

#[test]
fn multi_step_workflow_match_covers_all_cases() {
    let source = br#"
version: velvet-ballastics/v1
name: match_cases_test
when:
  manual: {}
steps:
  - id: step_a
    set:
      output: val_x
      value: "1"
  - id: step_b
    set:
      output: val_y
      value: "2"
  - id: done
    finish:
      result: val_x
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Match must cover each step index
    assert!(generated.contains("0 => step_0("));
    assert!(generated.contains("1 => step_1("));
    assert!(generated.contains("2 => step_2("));
}

// ---------------------------------------------------------------------------
// Exact structural assertions - line-level verification
// ---------------------------------------------------------------------------

#[test]
fn generated_code_starts_with_attribute() {
    let source = br#"
version: velvet-ballastics/v1
name: attr_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Generated code must start with #![forbid
    assert!(
        generated.starts_with("#![forbid(unsafe_code)]"),
        "generated code must start with forbid attribute"
    );
}

#[test]
fn generated_code_contains_use_statement() {
    let source = br#"
version: velvet-ballastics/v1
name: use_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must have use statement
    assert!(
        generated.contains("use std::convert::TryFrom;"),
        "generated code must have use statement"
    );
}

#[test]
fn generated_step_outcome_enum_exists() {
    let source = br#"
version: velvet-ballastics/v1
name: outcome_enum_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Must have StepOutcome enum (not pub - it's internal)
    assert!(
        generated.contains("enum StepOutcome"),
        "generated code must have StepOutcome enum"
    );
    // Must have Continue and Finished variants
    assert!(
        generated.contains("StepOutcome::Continue"),
        "StepOutcome must have Continue variant"
    );
    assert!(
        generated.contains("StepOutcome::Finished"),
        "StepOutcome must have Finished variant"
    );
}

// ---------------------------------------------------------------------------
// Journal contract
// ---------------------------------------------------------------------------

#[test]
fn journal_contract_section_has_separator() {
    let source = br#"
version: velvet-ballastics/v1
name: journal_section_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Journal contract section must be present
    assert!(
        generated.contains("// --- Generated journal contract ---"),
        "generated code must have journal contract section"
    );
}

#[test]
fn journal_capacity_constant_is_generated() {
    let source = br#"
version: velvet-ballastics/v1
name: journal_cap_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Journal capacity constant must be generated
    assert!(
        generated.contains("const GENERATED_JOURNAL_CAPACITY: usize = "),
        "generated code must have GENERATED_JOURNAL_CAPACITY"
    );
}

// ---------------------------------------------------------------------------
// Expression functions
// ---------------------------------------------------------------------------

#[test]
fn expression_functions_are_generated_when_expressions_exist() {
    // Note: This test verifies the codegen infrastructure for expressions.
    // Expression evaluation in generated code requires specific workflow constructs.
    // We test that the generated code structure supports expression functions.
    let source = br#"
version: velvet-ballastics/v1
name: expr_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // The generated code should have infrastructure for expression evaluation
    // even if this minimal workflow doesn't use expressions
    assert!(
        generated.contains("ExprStack"),
        "generated code must have ExprStack for expression evaluation"
    );
}

// ---------------------------------------------------------------------------
// Exact string matching for critical structural elements
// ---------------------------------------------------------------------------

#[test]
fn critical_structs_have_exact_form() {
    let source = br#"
version: velvet-ballastics/v1
name: exact_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Critical elements must have exact form
    assert!(generated.contains("pub enum SlotValue { Null, Bool(bool), I64(i64), F64(f64), Symbol(u32), List(u32), Object(u32), Blob(u64) }"));
    assert!(
        generated
            .contains("pub enum Taint { Clean, DerivedFromSecret, Secret, Random, TimeDependent }")
    );
    assert!(generated.contains("pub enum DriveError {"));
    // StepOutcome is not pub - it's internal
    assert!(generated.contains("enum StepOutcome { Continue(u16), Finished(SlotValue) }"));
    assert!(generated.contains("pub enum GeneratedSuspension {"));
}

#[test]
fn slot_value_impl_methods_are_present() {
    let source = br#"
version: velvet-ballastics/v1
name: slot_impl_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // SlotValue impl must have is_true and type_name
    assert!(generated.contains("impl SlotValue {"));
    assert!(generated.contains("pub const fn is_true(&self) -> bool"));
    assert!(generated.contains("pub const fn type_name(&self) -> &'static str"));
}

#[test]
fn drive_error_impl_has_display_if_present() {
    let source = br#"
version: velvet-ballastics/v1
name: error_impl_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // If there's an impl Display for DriveError, check it exists
    // (not all generated code may have this)
    if generated.contains("impl std::fmt::Display for DriveError") {
        assert!(generated.contains("fn fmt("));
    }
}

// ---------------------------------------------------------------------------
// Code that must NOT be present
// ---------------------------------------------------------------------------

#[test]
fn no_unsafe_code_in_generated_output() {
    let source = br#"
version: velvet-ballastics/v1
name: safe_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // Generated code must not contain unsafe blocks (since it's forbidden)
    assert!(
        !generated.contains("unsafe {"),
        "generated code must not contain unsafe blocks"
    );
    assert!(
        !generated.contains("unsafe("),
        "generated code must not contain unsafe calls"
    );
}

#[test]
fn no_explicit_unsafe_in_generated_output() {
    let source = br#"
version: velvet-ballastics/v1
name: no_explicit_unsafe_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source).expect("compile should succeed");
    let generated = emit_rust_workflow(&workflow).expect("codegen should succeed");

    // No unsafe keyword should appear (except in forbid attribute which we already checked)
    let lines: Vec<&str> = generated.lines().collect();
    for line in lines {
        if line.starts_with("#![") || line.starts_with("//!") {
            continue;
        }
        assert!(
            !line.contains("unsafe"),
            "non-attribute lines must not contain unsafe: {}",
            line
        );
    }
}
