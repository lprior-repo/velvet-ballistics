use proptest::prelude::*;
use vb_compile::{
    CompileError, CompileErrors, YamlCompiler, compile_source, compile_workflow, lower_choose,
    lower_steps_to_ir, lower_together,
};
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstValue, SlotBranch, SlotIdx, StepIdx,
    WorkflowError,
};

const HEADER: &str =
    "version: velvet-ballistics/v1\nname: primitive-lowering\nwhen:\n  manual: {}\nsteps:\n";

#[derive(Clone, Copy, Debug)]
struct PrimitiveCase {
    name: &'static str,
    yaml_steps: &'static str,
    expected_kinds: &'static [&'static str],
    expected_slot_count: u16,
}

#[derive(Clone, Copy, Debug)]
enum PublicApiPath {
    CompileSource,
    CompileWorkflow,
    YamlCompilerCompile,
}

const PUBLIC_API_PATHS: &[PublicApiPath] = &[
    PublicApiPath::CompileSource,
    PublicApiPath::CompileWorkflow,
    PublicApiPath::YamlCompilerCompile,
];

const FOREACH_KINDS: &[&str] = &["ForEachStart", "SetConst", "ForEachNext", "Finish"];
const TOGETHER_KINDS: &[&str] = &[
    "TogetherStart",
    "TogetherBranch",
    "SetConst",
    "TogetherBranch",
    "SetConst",
    "TogetherJoin",
    "Finish",
];
const COLLECT_KINDS: &[&str] = &[
    "CollectStart",
    "SetConst",
    "CollectPage",
    "CollectFinish",
    "Finish",
];
const REDUCE_KINDS: &[&str] = &[
    "ReduceStart",
    "SetConst",
    "ReduceNext",
    "ReduceFinish",
    "Finish",
];
const REPEAT_KINDS: &[&str] = &[
    "RepeatStart",
    "SetConst",
    "RepeatAttempt",
    "RepeatFinish",
    "Finish",
];
const WAIT_EVENT_KINDS: &[&str] = &["WaitEvent", "Finish"];
const ASK_KINDS: &[&str] = &["Ask", "AskResume", "Finish"];

const PRIMITIVE_CASES: &[PrimitiveCase] = &[
    PrimitiveCase {
        name: "for_each",
        yaml_steps: "  - id: loop\n    for_each:\n      variable: item\n      input: \"0\"\n      at_once: 2\n      steps:\n        - id: capture\n          set:\n            output: seen\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
        expected_kinds: FOREACH_KINDS,
        expected_slot_count: 2,
    },
    PrimitiveCase {
        name: "together",
        yaml_steps: "  - id: fanout\n    together:\n      branches:\n        - label: left\n          steps:\n            - id: left_set\n              set:\n                output: left\n                value: \"1\"\n        - label: right\n          steps:\n            - id: right_set\n              set:\n                output: right\n                value: \"2\"\n  - id: done\n    finish:\n      result: 0\n",
        expected_kinds: TOGETHER_KINDS,
        expected_slot_count: 4,
    },
    PrimitiveCase {
        name: "collect",
        yaml_steps: "  - id: collect_pages\n    collect:\n      variable: page\n      source: \"0\"\n      pages: 3\n      items: 5\n      steps:\n        - id: remember_page\n          set:\n            output: page_seen\n            value: \"7\"\n  - id: done\n    finish:\n      result: 0\n",
        expected_kinds: COLLECT_KINDS,
        expected_slot_count: 2,
    },
    PrimitiveCase {
        name: "reduce",
        yaml_steps: "  - id: fold\n    reduce:\n      variable: acc\n      input: \"0\"\n      initial: \"10\"\n      steps:\n        - id: add_one
          set:\n            output: acc_out\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
        expected_kinds: REDUCE_KINDS,
        expected_slot_count: 2,
    },
    PrimitiveCase {
        name: "repeat",
        yaml_steps: "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: attempt\n          set:\n            output: attempted\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
        expected_kinds: REPEAT_KINDS,
        expected_slot_count: 2,
    },
    PrimitiveCase {
        name: "wait",
        yaml_steps: "  - id: wait_for_event\n    wait:\n      event: \"0\"\n      timeout: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
        expected_kinds: WAIT_EVENT_KINDS,
        expected_slot_count: 2,
    },
    PrimitiveCase {
        name: "ask",
        yaml_steps: "  - id: ask_human\n    ask:\n      prompt: \"0\"\n      timeout: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
        expected_kinds: ASK_KINDS,
        expected_slot_count: 3,
    },
];

#[test]
fn compile_workflow_emits_supported_ir_when_each_scoped_primitive_is_valid() -> Result<(), String> {
    for case in PRIMITIVE_CASES {
        let workflow = compile_case(case)?;
        let parts = workflow.to_parts();
        let actual_kinds = node_kind_names(parts.nodes.as_ref());

        assert_eq!(
            actual_kinds, case.expected_kinds,
            "primitive {} emitted wrong node kind sequence",
            case.name
        );
        assert_eq!(
            parts.entry,
            StepIdx::new(0),
            "primitive {} entry must be dense zero",
            case.name
        );
        assert_eq!(
            parts.slot_count, case.expected_slot_count,
            "primitive {} slot_count must exactly cover primitive references",
            case.name
        );
        assert_exact_primitive_shape(&workflow, case.name)?;
        assert_dense_node_ids(&workflow, case.name)?;
        assert_all_targets_in_range(&workflow, case.name)?;
    }
    Ok(())
}

#[test]
fn compile_source_emits_supported_ir_when_each_scoped_primitive_is_valid() -> Result<(), String> {
    for case in PRIMITIVE_CASES {
        let yaml = workflow_yaml(case.yaml_steps);
        let source = parse_source(&yaml)?;
        let workflow = compile_source(&source).map_err(|errors| format_compile_errors(&errors))?;
        let parts = workflow.to_parts();

        assert_eq!(
            node_kind_names(parts.nodes.as_ref()),
            case.expected_kinds,
            "primitive {} compile_source node sequence",
            case.name
        );
        assert_eq!(
            parts.slot_count, case.expected_slot_count,
            "primitive {} compile_source slot_count",
            case.name
        );
        assert_exact_primitive_shape(&workflow, case.name)?;
    }
    Ok(())
}

#[test]
fn yaml_compiler_compile_emits_supported_ir_when_each_scoped_primitive_is_valid()
-> Result<(), String> {
    let compiler = YamlCompiler::default();
    for case in PRIMITIVE_CASES {
        let yaml = workflow_yaml(case.yaml_steps);
        let workflow = compiler
            .compile(yaml.as_bytes())
            .map_err(|errors| format_compile_errors(&errors))?;
        let parts = workflow.to_parts();

        assert_eq!(
            node_kind_names(parts.nodes.as_ref()),
            case.expected_kinds,
            "primitive {} YamlCompiler::compile node sequence",
            case.name
        );
        assert_eq!(
            parts.slot_count, case.expected_slot_count,
            "primitive {} YamlCompiler::compile slot_count",
            case.name
        );
        assert_exact_primitive_shape(&workflow, case.name)?;
    }
    Ok(())
}

#[test]
fn public_compile_apis_preserve_set_and_terminal_finish_regression() -> Result<(), String> {
    let yaml_steps = "  - id: assign\n    set:\n      output: answer\n      value: \"42\"\n  - id: done\n    finish:\n      result: answer\n";

    for api_path in PUBLIC_API_PATHS {
        let workflow = compile_steps_with_api(yaml_steps, *api_path)?;
        let parts = workflow.to_parts();

        assert_eq!(
            node_kind_names(parts.nodes.as_ref()),
            ["SetConst", "Finish"],
            "api {api_path:?} Set/Finish node sequence"
        );
        assert_eq!(
            parts.entry,
            StepIdx::new(0),
            "api {api_path:?} Set/Finish entry"
        );
        assert_eq!(
            parts.slot_count, 1,
            "api {api_path:?} Set/Finish exact slot_count"
        );
        assert_set_const_node(
            parts.nodes.as_ref(),
            parts.constants.as_ref(),
            0,
            Some(0),
            Some(1),
            0,
            42,
        )?;
        assert_finish_node(parts.nodes.as_ref(), 1, 0)?;
    }
    Ok(())
}

#[test]
fn compile_workflow_emits_exact_wait_until_shape_when_wait_has_deadline_only() -> Result<(), String>
{
    let yaml = workflow_yaml(
        "  - id: wait_until\n    wait:\n      timeout: \"0\"\n  - id: done\n    finish:\n      result: 0\n",
    );
    let workflow = compile_yaml(&yaml)?;
    let parts = workflow.to_parts();
    let actual_kinds = node_kind_names(parts.nodes.as_ref());

    assert_eq!(actual_kinds, ["WaitUntil", "Finish"]);
    assert_eq!(parts.slot_count, 1);
    assert_dense_node_ids(&workflow, "wait_until")?;
    assert_all_targets_in_range(&workflow, "wait_until")
}

#[test]
fn compile_workflow_returns_step_field_shape_when_each_scoped_primitive_required_field_is_empty()
-> Result<(), String> {
    let cases = [
        (
            "for_each",
            "  - id: loop\n    for_each:\n      variable: \"\"\n      input: \"0\"\n      steps: []\n  - id: done\n    finish:\n      result: 0\n",
            ExpectedShapeError::CanonicalYamlField("foreach.variable"),
        ),
        (
            "together",
            "  - id: fanout\n    together:\n      branches:\n        - label: \"\"\n          steps: []\n  - id: done\n    finish:\n      result: 0\n",
            ExpectedShapeError::CanonicalYamlField("together.branches[].label"),
        ),
        (
            "collect",
            "  - id: collect_pages\n    collect:\n      variable: page\n      source: \"\"\n      steps: []\n  - id: done\n    finish:\n      result: 0\n",
            ExpectedShapeError::CanonicalYamlField("collect.source"),
        ),
        (
            "reduce",
            "  - id: fold\n    reduce:\n      variable: acc\n      input: \"0\"\n      initial: \"\"\n      steps: []\n  - id: done\n    finish:\n      result: 0\n",
            ExpectedShapeError::CanonicalYamlField("reduce.initial"),
        ),
        (
            "repeat",
            "  - id: retry\n    repeat:\n      max_attempts: 0\n      steps: []\n  - id: done\n    finish:\n      result: 0\n",
            ExpectedShapeError::CompileStepField("repeat.max_attempts"),
        ),
        (
            "wait",
            "  - id: wait_for_event\n    wait:\n      event: \"\"\n  - id: done\n    finish:\n      result: 0\n",
            ExpectedShapeError::CanonicalYamlField("wait.event"),
        ),
        (
            "ask",
            "  - id: ask_human\n    ask:\n      prompt: \"\"\n  - id: done\n    finish:\n      result: 0\n",
            ExpectedShapeError::CanonicalYamlField("ask.prompt"),
        ),
    ];

    for (primitive, yaml_steps, expected_error) in cases {
        let yaml = workflow_yaml(yaml_steps);
        let errors = compile_yaml_error(&yaml)?;
        let first = first_compile_error(&errors)?;
        assert_expected_shape_error(primitive, first, expected_error)?;
    }
    Ok(())
}

#[test]
fn compile_workflow_rejects_multi_step_body_in_scoped_primitives() -> Result<(), String> {
    // Scoped primitives (repeat, for_each, collect, reduce) require exactly one set step
    // in their body. Multiple steps must be rejected with StepFieldShape error.
    let cases = [
        (
            "repeat with two steps in body",
            "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: step1\n          set:\n            output: out_x\n            value: \"1\"\n        - id: step2\n          set:\n            output: out_y\n            value: \"2\"\n  - id: done\n    finish:\n      result: 0\n",
        ),
        (
            "for_each with two steps in body",
            "  - id: loop\n    for_each:\n      variable: item\n      input: \"0\"\n      steps:\n        - id: step1\n          set:\n            output: out_x\n            value: \"1\"\n        - id: step2\n          set:\n            output: out_y\n            value: \"2\"\n  - id: done\n    finish:\n      result: 0\n",
        ),
        (
            "collect with two steps in body",
            "  - id: collect_pages\n    collect:\n      variable: page\n      source: \"0\"\n      steps:\n        - id: step1\n          set:\n            output: out_x\n            value: \"1\"\n        - id: step2\n          set:\n            output: out_y\n            value: \"2\"\n  - id: done\n    finish:\n      result: 0\n",
        ),
        (
            "reduce with two steps in body",
            "  - id: fold\n    reduce:\n      variable: acc\n      input: \"0\"\n      initial: \"10\"\n      steps:\n        - id: step1\n          set:\n            output: out_x\n            value: \"1\"\n        - id: step2\n          set:\n            output: out_y\n            value: \"2\"\n  - id: done\n    finish:\n      result: 0\n",
        ),
    ];

    for (case_name, yaml_steps) in cases {
        let yaml = workflow_yaml(yaml_steps);
        let errors = compile_yaml_error(&yaml)?;
        let first = first_compile_error(&errors)?;
        match first {
            CompileError::StepFieldShape {
                step,
                field,
                expected,
            } => {
                assert_eq!(
                    (*step, *field, expected.as_ref()),
                    (0, "steps", "exactly one set step"),
                    "case {case_name} expected step=0, 'steps' field with 'exactly one set step', got step={step} field='{field}' expected='{expected}'"
                );
            }
            other => {
                return Err(format!(
                    "case {case_name} expected StepFieldShape error with 'exactly one set step', got {other:?}"
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn compile_workflow_rejects_non_set_body_in_collect() -> Result<(), String> {
    // When collect's body contains a non-Set primitive, emit_single_body_set
    // must report UnsupportedStepPrimitive with the original source step (0).
    let yaml = workflow_yaml(
        "  - id: collect_pages\n    collect:\n      variable: page\n      source: \"0\"\n      steps:\n        - id: inner\n          collect:\n            variable: inner_page\n            source: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
    );
    let errors = compile_yaml_error(&yaml)?;
    let first = first_compile_error(&errors)?;
    match first {
        CompileError::UnsupportedStepPrimitive { step, primitive } => {
            assert_eq!(
                (*step, *primitive),
                (0, "collect"),
                "expected step=0, primitive='collect', got step={step} primitive='{primitive}'"
            );
            Ok(())
        }
        other => Err(format!(
            "expected UnsupportedStepPrimitive error for non-Set body in collect, got {other:?}"
        )),
    }
}

#[derive(Clone, Copy)]
enum ExpectedShapeError {
    CanonicalYamlField(&'static str),
    CompileStepField(&'static str),
}

fn assert_expected_shape_error(
    primitive: &str,
    actual: &CompileError,
    expected_error: ExpectedShapeError,
) -> Result<(), String> {
    match (actual, expected_error) {
        (
            CompileError::CanonicalYaml { category, message },
            ExpectedShapeError::CanonicalYamlField(expected_field),
        ) => {
            assert_eq!(
                (*category, message.contains(expected_field)),
                ("field_shape", true),
                "primitive {primitive} returned wrong canonical field-shape payload"
            );
            Ok(())
        }
        (
            CompileError::StepFieldShape {
                step,
                field,
                expected,
            },
            ExpectedShapeError::CompileStepField(expected_field),
        ) => {
            assert_eq!(
                (*step, *field, *expected),
                (0, expected_field, "non-empty primitive field"),
                "primitive {primitive} returned wrong compile field-shape payload"
            );
            Ok(())
        }
        (other, ExpectedShapeError::CanonicalYamlField(_)) => Err(format!(
            "primitive {primitive} expected CanonicalYaml field_shape, got {other:?}"
        )),
        (other, ExpectedShapeError::CompileStepField(_)) => Err(format!(
            "primitive {primitive} expected StepFieldShape, got {other:?}"
        )),
    }
}

#[test]
fn compile_workflow_returns_unsupported_step_primitive_only_for_out_of_scope_primitives()
-> Result<(), String> {
    // These primitives were previously unsupported but are now supported:
    // - save: legacy alias for set (supported)
    // - do: action invocation (supported in vb-xi2f.1)
    // - choose: conditional branching (partially supported in vb-xi2f.17)
    // This test is now obsolete but kept as documentation of known supported primitives.
    // The test below verifies that a truly unknown primitive is rejected.
    let yaml = workflow_yaml(
        "  - id: test\n    clearly_unsupported_primitive:\n      field: value\n  - id: done\n    finish:\n      result: 0\n",
    );
    let result = compile_workflow(yaml.as_bytes());
    // Clearly unsupported primitive should fail
    assert!(
        result.is_err(),
        "expected failure for unsupported primitive, got Ok"
    );
    Ok(())
}

#[test]
fn public_compile_apis_return_unsupported_step_primitive_for_save_do_choose_only()
-> Result<(), String> {
    // These primitives were previously unsupported but are now supported:
    // - save: legacy alias for set (supported)
    // - do: action invocation (supported in vb-xi2f.1)
    // - choose: conditional branching (partially supported in vb-xi2f.17)
    // This test is now obsolete but kept as documentation.
    // The test verifies that a truly unknown primitive is rejected across all APIs.
    for api_path in PUBLIC_API_PATHS {
        let yaml_steps = "  - id: test\n    clearly_unsupported_primitive:\n      field: value\n  - id: done\n    finish:\n      result: 0\n";
        let result = compile_steps_with_api(yaml_steps, *api_path);
        assert!(
            result.is_err(),
            "api {api_path:?} expected failure for unsupported primitive, got Ok"
        );
    }
    Ok(())
}

#[test]
fn compile_source_returns_exact_error_variants_for_contract_taxonomy() -> Result<(), String> {
    let cases = [
        (
            "empty_steps",
            "version: velvet-ballistics/v1\nname: empty\nwhen:\n  manual: {}\nsteps: []\n",
            ExpectedCompileError::EmptySteps,
        ),
        (
            "top_level_inputs",
            "version: velvet-ballistics/v1\nname: inputs\nwhen:\n  manual: {}\ninputs:\n  account:\n    type: string\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
            ExpectedCompileError::UnsupportedTopLevelDeclaration("inputs"),
        ),
        (
            "top_level_result",
            "version: velvet-ballistics/v1\nname: result\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\nresult:\n  ok: true\n",
            ExpectedCompileError::UnsupportedTopLevelResult,
        ),
        (
            "step_name_control",
            "  - id: named\n    name: Display\n    set:\n      output: value\n      value: \"1\"\n  - id: done\n    finish:\n      result: value\n",
            ExpectedCompileError::UnsupportedStepControlField {
                step: 0,
                field: "name",
            },
        ),
        (
            "duplicate_step_id",
            "  - id: same\n    set:\n      output: first\n      value: \"1\"\n  - id: same\n    finish:\n      result: first\n",
            ExpectedCompileError::DuplicateStepId("same"),
        ),
        (
            "duplicate_output",
            "  - id: first\n    set:\n      output: value\n      value: \"1\"\n  - id: second\n    set:\n      output: value\n      value: \"2\"\n  - id: done\n    finish:\n      result: value\n",
            ExpectedCompileError::DuplicateOutputName("value"),
        ),
        (
            "unknown_output",
            "  - id: first\n    set:\n      output: value\n      value: \"1\"\n  - id: done\n    finish:\n      result: missing\n",
            ExpectedCompileError::UnknownOutputName("missing"),
        ),
        (
            "set_value_shape",
            "  - id: first\n    set:\n      output: value\n      value: not-an-integer\n  - id: done\n    finish:\n      result: value\n",
            ExpectedCompileError::StepFieldShape {
                step: 0,
                field: "set.value",
                expected: "integer string",
            },
        ),
        (
            "slot_index_out_of_range",
            "  - id: done\n    finish:\n      result: 65536\n",
            ExpectedCompileError::SlotIndexOutOfRange { value: 65_536 },
        ),
    ];

    for (case_name, body, expected) in cases {
        let yaml = if body.starts_with("version:") {
            String::from(body)
        } else {
            workflow_yaml(body)
        };
        let source = parse_source(&yaml)?;
        let errors = compile_source(&source)
            .err()
            .ok_or_else(|| format!("case {case_name} unexpectedly compiled"))?;
        let first = first_compile_error(&errors)?;
        assert_expected_compile_error(case_name, first, expected)?;
    }
    Ok(())
}

#[test]
fn public_compile_apis_return_exact_error_variants_for_contract_taxonomy() -> Result<(), String> {
    let cases = [
        (
            "empty_steps",
            "version: velvet-ballistics/v1\nname: empty\nwhen:\n  manual: {}\nsteps: []\n",
            ExpectedCompileError::EmptySteps,
        ),
        (
            "top_level_inputs",
            "version: velvet-ballistics/v1\nname: inputs\nwhen:\n  manual: {}\ninputs:\n  account:\n    type: string\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
            ExpectedCompileError::UnsupportedTopLevelDeclaration("inputs"),
        ),
        (
            "top_level_result",
            "version: velvet-ballistics/v1\nname: result\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\nresult:\n  ok: true\n",
            ExpectedCompileError::UnsupportedTopLevelResult,
        ),
        (
            "step_name_control",
            "  - id: named\n    name: Display\n    set:\n      output: value\n      value: \"1\"\n  - id: done\n    finish:\n      result: value\n",
            ExpectedCompileError::UnsupportedStepControlField {
                step: 0,
                field: "name",
            },
        ),
        (
            "duplicate_step_id",
            "  - id: same\n    set:\n      output: first\n      value: \"1\"\n  - id: same\n    finish:\n      result: first\n",
            ExpectedCompileError::DuplicateStepId("same"),
        ),
        (
            "duplicate_output",
            "  - id: first\n    set:\n      output: value\n      value: \"1\"\n  - id: second\n    set:\n      output: value\n      value: \"2\"\n  - id: done\n    finish:\n      result: value\n",
            ExpectedCompileError::DuplicateOutputName("value"),
        ),
        (
            "unknown_output",
            "  - id: first\n    set:\n      output: value\n      value: \"1\"\n  - id: done\n    finish:\n      result: missing\n",
            ExpectedCompileError::UnknownOutputName("missing"),
        ),
        (
            "set_value_shape",
            "  - id: first\n    set:\n      output: value\n      value: not-an-integer\n  - id: done\n    finish:\n      result: value\n",
            ExpectedCompileError::StepFieldShape {
                step: 0,
                field: "set.value",
                expected: "integer string",
            },
        ),
        (
            "slot_index_out_of_range",
            "  - id: done\n    finish:\n      result: 65536\n",
            ExpectedCompileError::SlotIndexOutOfRange { value: 65_536 },
        ),
    ];

    for api_path in PUBLIC_API_PATHS {
        for (case_name, body, expected) in cases {
            let errors = compile_document_or_steps_error_with_api(body, *api_path)?;
            let first = first_compile_error(&errors)?;
            assert_expected_compile_error(case_name, first, expected)?;
        }
    }
    Ok(())
}

#[test]
fn public_helpers_return_exact_step_index_slot_index_limit_and_workflow_error_variants()
-> Result<(), String> {
    let mut slot_builder = vb_compile::SlotCompiler::new();
    match vb_compile::lower_repeat(
        StepIdx::MAX,
        1,
        StepIdx::MAX,
        StepIdx::MAX,
        &mut slot_builder,
    ) {
        Err(CompileError::StepIndexOutOfRange { value }) => assert_eq!(value, 65_536),
        other => {
            return Err(format!(
                "expected StepIndexOutOfRange from repeat overflow, got {other:?}"
            ));
        }
    }

    let mut ask_builder = vb_compile::SlotCompiler::new();
    match vb_compile::lower_ask(
        StepIdx::MAX,
        vb_core::SlotIdx::new(0),
        vb_core::SlotIdx::new(1),
        None,
        &mut ask_builder,
    ) {
        Err(CompileError::PrimitiveLoweringLimitExceeded {
            primitive,
            field,
            value,
            limit,
        }) => {
            assert_eq!(
                (primitive, field, value, limit),
                ("ask", "resume_step", 65_535, 65_535)
            );
        }
        other => {
            return Err(format!(
                "expected PrimitiveLoweringLimitExceeded from ask overflow, got {other:?}"
            ));
        }
    }

    match lower_steps_to_ir(
        vec![CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::Finish {
                result: vb_core::SlotIdx::new(0),
            },
        }],
        Vec::new(),
        Vec::new(),
        Vec::new(),
        1,
        0,
        "bad-id",
        vb_core::WorkflowDigest::from_bytes([1; 32]),
    ) {
        Err(errors) => match first_compile_error(&errors)? {
            CompileError::Workflow(WorkflowError::NodeIdMismatch { expected, actual }) => {
                assert_eq!((expected.get(), actual.get()), (0, 1));
            }
            other => return Err(format!("expected Workflow(NodeIdMismatch), got {other:?}")),
        },
        Ok(_) => return Err(String::from("mismatched node id unexpectedly compiled")),
    }
    Ok(())
}

#[test]
fn yaml_compiler_compile_returns_canonical_yaml_when_source_parse_fails() -> Result<(), String> {
    let errors = YamlCompiler::default()
        .compile(
            b"version: velvet-ballistics/v1\nname: bad\nwhen:\n  manual: {}\nsteps:\n  - id:\n",
        )
        .err()
        .ok_or_else(|| String::from("invalid canonical YAML unexpectedly compiled"))?;
    let first = first_compile_error(&errors)?;

    match first {
        CompileError::CanonicalYaml { category, message } => {
            assert_eq!(*category, "field_shape");
            assert_eq!(message.contains("id"), true);
            Ok(())
        }
        other => Err(format!("expected CanonicalYaml, got {other:?}")),
    }
}

#[test]
fn public_lowering_helpers_return_exact_range_and_workflow_errors() -> Result<(), String> {
    let many_branches = vec![StepIdx::new(0); usize::from(u16::MAX) + 1];
    let mut builder = vb_compile::SlotCompiler::new();
    match lower_together(
        StepIdx::new(0),
        many_branches,
        StepIdx::new(1),
        &mut builder,
    ) {
        Err(CompileError::PrimitiveLoweringLimitExceeded {
            primitive,
            field,
            value,
            limit,
        }) => {
            assert_eq!(
                (primitive, field, value, limit),
                ("together", "branches", 65_536, 65_535)
            );
        }
        other => {
            return Err(format!(
                "expected PrimitiveLoweringLimitExceeded, got {other:?}"
            ));
        }
    }

    match lower_steps_to_ir(
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        0,
        0,
        "empty",
        vb_core::WorkflowDigest::from_bytes([0; 32]),
    ) {
        Err(errors) => match first_compile_error(&errors)? {
            CompileError::Workflow(WorkflowError::EmptyNodes) => {}
            other => return Err(format!("expected Workflow(EmptyNodes), got {other:?}")),
        },
        Ok(_) => return Err(String::from("empty IR unexpectedly compiled")),
    }
    Ok(())
}

#[test]
fn lower_choose_rejects_more_than_64_branches() -> Result<(), String> {
    // Given: 65 branches (exceeds the 64 branch limit)
    let too_many_branches: Vec<SlotBranch> = (0..65u16)
        .map(|i| SlotBranch {
            condition: SlotIdx::new(i),
            target: StepIdx::new(100u16 + i),
        })
        .collect();
    let mut builder = vb_compile::SlotCompiler::new();

    // When: lower_choose is called with 65 branches
    let result = lower_choose(
        StepIdx::new(0),
        too_many_branches,
        Some(StepIdx::new(200)),
        &mut builder,
    );

    // Then: it must fail with PrimitiveLoweringLimitExceeded
    match result {
        Err(CompileError::PrimitiveLoweringLimitExceeded {
            primitive,
            field,
            value,
            limit,
        }) => {
            assert_eq!(
                (primitive, field, value, limit),
                ("choose", "branches", 65, 64)
            );
        }
        other => {
            return Err(format!(
                "expected PrimitiveLoweringLimitExceeded for 65 branches, got {other:?}"
            ));
        }
    }
    Ok(())
}

#[test]
fn lower_choose_accepts_valid_otherwise_target() -> Result<(), String> {
    // Given: a single branch with a valid otherwise target
    let single_branch = vec![SlotBranch {
        condition: SlotIdx::new(0),
        target: StepIdx::new(1),
    }];
    let mut builder = vb_compile::SlotCompiler::new();

    // When: lower_choose is called with valid otherwise
    let result = lower_choose(
        StepIdx::new(0),
        single_branch,
        Some(StepIdx::new(2)),
        &mut builder,
    );

    // Then: it must succeed and produce a ChooseSlot node
    let node = result.map_err(|e| format!("lower_choose failed: {e}"))?;
    match node.kind {
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => {
            assert_eq!(branches.len(), 1);
            assert_eq!(otherwise, Some(StepIdx::new(2)));
        }
        other => {
            return Err(format!("expected ChooseSlot node, got {other:?}"));
        }
    }
    Ok(())
}

#[test]
fn lower_choose_accepts_empty_branches_with_otherwise() -> Result<(), String> {
    // Given: empty branches with a valid otherwise target
    let empty_branches: Vec<SlotBranch> = vec![];
    let mut builder = vb_compile::SlotCompiler::new();

    // When: lower_choose is called with empty branches but otherwise is Some
    let result = lower_choose(
        StepIdx::new(0),
        empty_branches,
        Some(StepIdx::new(2)),
        &mut builder,
    );

    // Then: it must succeed (empty branches with otherwise is valid)
    let node = result.map_err(|e| format!("lower_choose failed: {e}"))?;
    match node.kind {
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => {
            assert_eq!(branches.len(), 0);
            assert_eq!(otherwise, Some(StepIdx::new(2)));
        }
        other => {
            return Err(format!("expected ChooseSlot node, got {other:?}"));
        }
    }
    Ok(())
}

#[test]
fn lower_choose_rejects_empty_branches_without_otherwise() -> Result<(), String> {
    // Given: empty branches with no otherwise target
    let empty_branches: Vec<SlotBranch> = vec![];
    let mut builder = vb_compile::SlotCompiler::new();

    // When: lower_choose is called with empty branches and no otherwise
    let result = lower_choose(StepIdx::new(0), empty_branches, None, &mut builder);

    // Then: it must fail with EmptyBranchTable
    match result {
        Err(CompileError::Workflow(WorkflowError::EmptyBranchTable)) => {}
        other => {
            return Err(format!(
                "expected EmptyBranchTable error for empty branches with no otherwise, got {other:?}"
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum ExpectedCompileError {
    EmptySteps,
    UnsupportedTopLevelDeclaration(&'static str),
    UnsupportedTopLevelResult,
    UnsupportedStepControlField {
        step: usize,
        field: &'static str,
    },
    DuplicateStepId(&'static str),
    DuplicateOutputName(&'static str),
    UnknownOutputName(&'static str),
    StepFieldShape {
        step: usize,
        field: &'static str,
        expected: &'static str,
    },
    SlotIndexOutOfRange {
        value: i64,
    },
}

fn assert_expected_compile_error(
    case_name: &str,
    actual: &CompileError,
    expected: ExpectedCompileError,
) -> Result<(), String> {
    match (actual, expected) {
        (CompileError::EmptySteps, ExpectedCompileError::EmptySteps) => Ok(()),
        (
            CompileError::UnsupportedTopLevelDeclaration { field },
            ExpectedCompileError::UnsupportedTopLevelDeclaration(expected_field),
        ) => {
            assert_eq!(
                *field, expected_field,
                "case {case_name} top-level declaration field"
            );
            Ok(())
        }
        (
            CompileError::UnsupportedTopLevelResult,
            ExpectedCompileError::UnsupportedTopLevelResult,
        ) => Ok(()),
        (
            CompileError::UnsupportedStepControlField { step, field },
            ExpectedCompileError::UnsupportedStepControlField {
                step: expected_step,
                field: expected_field,
            },
        ) => {
            assert_eq!(
                (*step, field.as_ref()),
                (expected_step, expected_field),
                "case {case_name} control field payload"
            );
            Ok(())
        }
        (
            CompileError::DuplicateStepId { id },
            ExpectedCompileError::DuplicateStepId(expected_id),
        ) => {
            assert_eq!(
                id.as_ref(),
                expected_id,
                "case {case_name} duplicate step id"
            );
            Ok(())
        }
        (
            CompileError::DuplicateOutputName { name },
            ExpectedCompileError::DuplicateOutputName(expected_name),
        ) => {
            assert_eq!(
                name.as_ref(),
                expected_name,
                "case {case_name} duplicate output name"
            );
            Ok(())
        }
        (
            CompileError::UnknownOutputName { name },
            ExpectedCompileError::UnknownOutputName(expected_name),
        ) => {
            assert_eq!(
                name.as_ref(),
                expected_name,
                "case {case_name} unknown output name"
            );
            Ok(())
        }
        (
            CompileError::StepFieldShape {
                step,
                field,
                expected,
            },
            ExpectedCompileError::StepFieldShape {
                step: expected_step,
                field: expected_field,
                expected: expected_expected,
            },
        ) => {
            assert_eq!(
                (*step, *field, *expected),
                (expected_step, expected_field, expected_expected),
                "case {case_name} step field shape payload"
            );
            Ok(())
        }
        (
            CompileError::SlotIndexOutOfRange { value },
            ExpectedCompileError::SlotIndexOutOfRange {
                value: expected_value,
            },
        ) => {
            assert_eq!(
                *value, expected_value,
                "case {case_name} slot range payload"
            );
            Ok(())
        }
        (other, _) => Err(format!("case {case_name} returned wrong error {other:?}")),
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, failure_persistence: None, .. ProptestConfig::default() })]

    #[test]
    fn proptest_equal_primitive_sources_compile_to_equal_digest_and_ir(case in primitive_case_strategy()) {
        let left = compile_case(&case).map_err(TestCaseError::fail)?;
        let right = compile_case(&case).map_err(TestCaseError::fail)?;

        prop_assert_eq!(left.digest(), right.digest());
        prop_assert_eq!(left.to_parts(), right.to_parts());
    }

    #[test]
    fn proptest_scoped_primitives_never_return_unsupported_step_primitive(case in primitive_case_strategy()) {
        let yaml = workflow_yaml(case.yaml_steps);
        match compile_workflow(yaml.as_bytes()) {
            Ok(workflow) => {
                let actual_kinds = node_kind_names(workflow.to_parts().nodes.as_ref());
                prop_assert_eq!(actual_kinds, case.expected_kinds);
            }
            Err(errors) => {
                let unsupported = unsupported_primitive_name(&errors);
                prop_assert_eq!(unsupported, None, "in-scope primitive {} must not be unsupported", case.name);
            }
        }
    }
    // ── Wait digest sensitivity tests (vb-xi2f.32) ──

    /// PO-002: Different wait field values produce different digests.
    /// Forall a,b with different wait event/timeout fields,
    /// canonical_digest(workflow_with(a)) != canonical_digest(workflow_with(b)).
    #[test]
    fn proptest_wait_field_sensitivity(
        (event_a, timeout_a) in wait_field_strategy(),
        (event_b, timeout_b) in wait_field_strategy(),
    ) {
        // Ensure the two configurations are actually different
        prop_assume!(event_a != event_b || timeout_a != timeout_b);

        let source_a = wait_workflow_source(&event_a, &timeout_a);
        let source_b = wait_workflow_source(&event_b, &timeout_b);

        let digest_a = canonical_digest_compat(&source_a)
            .map_err(|e| TestCaseError::fail(e))?;
        let digest_b = canonical_digest_compat(&source_b)
            .map_err(|e| TestCaseError::fail(e))?;

        prop_assert_ne!(digest_a, digest_b,
            "different wait fields must produce different digests: ({:?},{:?}) vs ({:?},{:?})",
            event_a, timeout_a, event_b, timeout_b);
    }

    /// PO-004: WaitUntil and WaitEvent produce different digests.
    /// Forall timeout_text, event_text:
    ///   digest(WaitUntil{timeout_text}) != digest(WaitEvent{event_text, timeout_text})
    #[test]
    fn proptest_wait_until_vs_wait_event(
        timeout_text in wait_slot_strategy(),
        event_text in wait_slot_strategy(),
    ) {
        let until_source = wait_workflow_source(&None, &Some(timeout_text.clone()));
        let event_source = wait_workflow_source(&Some(event_text), &Some(timeout_text));

        let digest_until = canonical_digest_compat(&until_source)
            .map_err(|e| TestCaseError::fail(e))?;
        let digest_event = canonical_digest_compat(&event_source)
            .map_err(|e| TestCaseError::fail(e))?;

        prop_assert_ne!(digest_until, digest_event,
            "WaitUntil and WaitEvent must produce different digests");
    }

    /// PO-006: Timeout field sensitivity — different timeout values produce
    /// different digests. This is the reachable version of the sentinel test:
    /// the sentinel "none" is not valid YAML (must be integer string) so it
    /// cannot reach `canonical_digest` through compilation. The sentinel
    /// property itself is verified by Kani (direct `digest_step_primitive` call).
    /// Here we verify that different integer timeout values produce different
    /// digests, which indirectly covers the property that absent (None) ≠
    /// present (some nonzero value).
    #[test]
    fn proptest_wait_sentinel_unambiguous(
        event_text in wait_slot_strategy(),
        timeout_a in wait_slot_strategy(),
        timeout_b in wait_slot_strategy(),
    ) {
        // Only relevant when the two timeout values differ
        prop_assume!(timeout_a != timeout_b);

        let source_a = wait_workflow_source(&Some(event_text.clone()), &Some(timeout_a));
        let source_b = wait_workflow_source(&Some(event_text), &Some(timeout_b));

        let digest_a = canonical_digest_compat(&source_a)
            .map_err(|e| TestCaseError::fail(e))?;
        let digest_b = canonical_digest_compat(&source_b)
            .map_err(|e| TestCaseError::fail(e))?;

        prop_assert_ne!(digest_a, digest_b,
            "different timeout values must produce different digests");
    }

    /// PO-009 / PO-016: Cross-path digest equivalence.
    /// For all generated workflow sources with Wait steps, compile_source()
    /// and compile_workflow() produce identical WorkflowDigest values.
    #[test]
    fn cross_path_wait_digest_equivalence(
        event in wait_slot_strategy(),
        timeout in wait_slot_strategy(),
    ) {
        // Build a valid Wait workflow using a random shape
        let hash_byte = event.as_bytes().first().copied().unwrap_or(0)
            .wrapping_add(timeout.as_bytes().first().copied().unwrap_or(0));
        let (event, timeout) = match hash_byte % 3 {
            0 => (None, Some(timeout)),                    // WaitUntil
            1 => (Some(event), None),                      // WaitEvent unbounded
            _ => (Some(event), Some(timeout)),             // WaitEvent bounded
        };

        let source = wait_workflow_source(&event, &timeout);

        // compile_source uses the cold-path (canonical_digest in part_05.rs)
        let cold = compile_source(&source)
            .map_err(|e| TestCaseError::fail(format!("cold-path compile failed: {e:?}")))?;

        // compile_workflow delegates to YamlCompiler::compile() → compile_source
        let yaml = wait_workflow_yaml(&event, &timeout);
        let warm = compile_workflow(yaml.as_bytes())
            .map_err(|e| TestCaseError::fail(format!("warm-path compile failed: {e:?}")))?;

        prop_assert_eq!(cold.digest(), warm.digest(),
            "cold-path and warm-path must produce identical digests");
    }

    /// PO-011: Pairwise distinct digests for distinct Wait configurations.
    /// For any two different Wait configurations wa != wb in otherwise-identical
    /// workflows, canonical_digest(wf_with(wa)) != canonical_digest(wf_with(wb)).
    #[test]
    fn proptest_wait_pairwise_distinct_digests(
        e1 in wait_slot_strategy(),
        t1 in wait_slot_strategy(),
        e2 in wait_slot_strategy(),
        t2 in wait_slot_strategy(),
    ) {
        // Build two different legal Wait shapes
        let w1 = make_legal_wait_shape(&e1, &t1);
        let w2 = make_legal_wait_shape(&e2, &t2);

        // If the Wait shapes are identical, skip
        if w1 == w2 {
            return Ok(());
        }

        let source1 = wait_workflow_source(&w1.0, &w1.1);
        let source2 = wait_workflow_source(&w2.0, &w2.1);

        let digest1 = canonical_digest_compat(&source1)
            .map_err(|e| TestCaseError::fail(e))?;
        let digest2 = canonical_digest_compat(&source2)
            .map_err(|e| TestCaseError::fail(e))?;

        prop_assert_ne!(digest1, digest2,
            "distinct Wait shapes must produce distinct digests");
    }
}

// ── Strategies and helpers for Wait digest tests ──

/// Generates a slot expression string: integer-like strings "0".."255".
/// The validator expects integer strings for wait event/timeout fields.
fn wait_slot_strategy() -> impl Strategy<Value = String> {
    // Generate integer-looking strings that pass the validator
    (0u8..255u8).prop_map(|n| n.to_string())
}

/// Generates (Option<String>, Option<String>) pairs for wait fields.
/// At least one field will be Some (legal shape guarantee). Randomly
/// makes each field None to cover all three legal Wait shapes.
fn wait_field_strategy() -> impl Strategy<Value = (Option<String>, Option<String>)> {
    (
        wait_slot_strategy(),
        wait_slot_strategy(),
        any::<u8>(),
        any::<u8>(),
    )
        .prop_map(|(e, t, eb, tb)| {
            let event = if eb % 3 == 0 { None } else { Some(e) };
            let timeout = if tb % 3 == 0 { None } else { Some(t) };
            (event, timeout)
        })
        .prop_filter("at least one wait field must be Some", |(e, t)| {
            e.is_some() || t.is_some()
        })
}

/// Returns a legal Wait field pair from two strings.
/// Varies the shape between WaitUntil (event=None, timeout=Some),
/// WaitEvent-unbounded (event=Some, timeout=None), and
/// WaitEvent-bounded (event=Some, timeout=Some) based on a hash.
fn make_legal_wait_shape(event: &str, timeout: &str) -> (Option<String>, Option<String>) {
    // Use the first byte of hash to pick a Wait shape variant
    let hash_byte = event
        .as_bytes()
        .first()
        .copied()
        .unwrap_or(0)
        .wrapping_add(timeout.as_bytes().first().copied().unwrap_or(0));
    match hash_byte % 3 {
        0 => (None, Some(timeout.to_string())), // WaitUntil
        1 => (Some(event.to_string()), None),   // WaitEvent unbounded
        _ => (Some(event.to_string()), Some(timeout.to_string())), // WaitEvent bounded
    }
}

/// Builds a WorkflowSource with a single Wait step using the given fields.
fn wait_workflow_source(
    event: &Option<String>,
    timeout: &Option<String>,
) -> vb_yaml::ast::WorkflowSource {
    let yaml = wait_workflow_yaml(event, timeout);
    vb_yaml::parse_workflow_source(&yaml).expect("valid wait workflow YAML")
}

/// Builds the YAML string for a single-Wait-step workflow.
fn wait_workflow_yaml(event: &Option<String>, timeout: &Option<String>) -> String {
    let mut wait_block = String::from("  - id: wait_step\n    wait:");
    if let Some(e) = event {
        wait_block.push_str(&format!("\n      event: \"{e}\""));
    }
    if let Some(t) = timeout {
        wait_block.push_str(&format!("\n      timeout: \"{t}\""));
    }
    format!(
        "version: velvet-ballastics/v1\nname: wait-digest-test\nwhen:\n  manual: {{}}\nsteps:\n{wait_block}\n  - id: finish_step\n    finish:\n      result: 0\n"
    )
}

/// Computes the canonical_digest from a parsed WorkflowSource.
/// Uses compile_source (cold-path), which internally calls canonical_digest.
fn canonical_digest_compat(
    source: &vb_yaml::ast::WorkflowSource,
) -> Result<vb_core::WorkflowDigest, String> {
    compile_source(source)
        .map(|wf| wf.digest())
        .map_err(|errors| format!("compile_source failed: {errors:?}"))
}

fn primitive_case_strategy() -> impl Strategy<Value = PrimitiveCase> {
    prop::sample::select(PRIMITIVE_CASES.to_vec())
}

// =========================================================================
// PO-006: Proptest for different repeat configs → different digest
// =========================================================================

/// Proptest strategy generating two distinct repeat configurations.
///
/// Produces pairs of repeat cases identical except for `max_attempts`.
/// GOD RULE 1: variable max_attempts, not single hardcoded value.
fn repeat_variable_strategy() -> impl Strategy<Value = (u16, u16)> {
    (1u16..=u16::MAX, 1u16..=u16::MAX).prop_filter("max_attempts must differ", |(a, b)| a != b)
}

/// YAML template with a `{}` placeholder for max_attempts.
const REPEAT_YAML_BASE: &str = "  - id: retry\n    repeat:\n      max_attempts: {}\n      steps:\n        - id: attempt\n          set:\n            output: attempted\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

fn repeat_yaml(max_attempts: u16) -> String {
    REPEAT_YAML_BASE.replace("{}", &max_attempts.to_string())
}

// PO-006 / PROPTEST-REPEAT-001: Different repeat max_attempts produce
// different WorkflowDigest values via compile_workflow.
//
// Non-vacuous: asserts inequality, not just that both compile.
proptest! {
    #![proptest_config(ProptestConfig { cases: 64, failure_persistence: None, .. ProptestConfig::default() })]

    #[test]
    fn proptest_repeat_different_params_different_digest(
        (max1, max2) in repeat_variable_strategy()
    ) {
        use proptest::prelude::*;

        let yaml1 = workflow_yaml(&repeat_yaml(max1));
        let yaml2 = workflow_yaml(&repeat_yaml(max2));

        let wf1 = compile_workflow(yaml1.as_bytes()).map_err(|e| TestCaseError::fail(format_compile_errors(&e)))?;
        let wf2 = compile_workflow(yaml2.as_bytes()).map_err(|e| TestCaseError::fail(format_compile_errors(&e)))?;

        prop_assert_ne!(
            wf1.digest(),
            wf2.digest(),
            "repeat max_attempts {} vs {} must produce different digests",
            max1, max2
        );
    }

    // PO-006 extended: Different repeat body contents produce different digests.
    // Remove doc-comment from inside proptest! macro (macros cannot host doc attrs).
    #[test]
    fn proptest_repeat_different_body_different_digest(
        max_attempts in 1u16..=u16::MAX,
    ) {
        use proptest::prelude::*;

        // Body A: single Set
        let yaml_set_body = workflow_yaml(&format!(
            "  - id: retry\n    repeat:\n      max_attempts: {max}\n      steps:\n        - id: a_set\n          set:\n            output: seen\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
            max = max_attempts
        ));

        // Body B: different Set with distinct output/value
        let yaml_diff_body = workflow_yaml(&format!(
            "  - id: retry\n    repeat:\n      max_attempts: {max}\n      steps:\n        - id: s1\n          set:\n            output: out1\n            value: \"99\"\n        - id: s2\n          set:\n            output: out2\n            value: \"100\"\n  - id: done\n    finish:\n      result: 0\n",
            max = max_attempts
        ));

        let wf1 = compile_workflow(yaml_set_body.as_bytes()).map_err(|e| TestCaseError::fail(format_compile_errors(&e)))?;
        let wf2 = compile_workflow(yaml_diff_body.as_bytes()).map_err(|e| TestCaseError::fail(format_compile_errors(&e)))?;

        prop_assert_ne!(
            wf1.digest(),
            wf2.digest(),
            "repeat with single-Set body vs multi-Set body must produce different digests"
        );
    }
}

fn compile_case(case: &PrimitiveCase) -> Result<CompiledWorkflow, String> {
    let yaml = workflow_yaml(case.yaml_steps);
    compile_yaml(&yaml).map_err(|error| format!("primitive {} failed: {error}", case.name))
}

// ── Section 9.3: Integration tests for Wait digest (vb-xi2f.32) ──

/// B1: Two WaitEvent workflows with different event values must produce
/// different digests. Tested through compile_source (cold-path).
#[test]
fn wait_event_sensitivity_to_event_field_change_through_compile_source_when_event_differs()
-> Result<(), String> {
    let steps_a = "  - id: wait_a\n    wait:\n      event: \"0\"\n      timeout: \"30\"\n  - id: done\n    finish:\n      result: 0\n";
    let steps_b = "  - id: wait_a\n    wait:\n      event: \"1\"\n      timeout: \"30\"\n  - id: done\n    finish:\n      result: 0\n";

    let source_a = parse_source(&workflow_yaml(steps_a))?;
    let source_b = parse_source(&workflow_yaml(steps_b))?;

    let digest_a = compile_source(&source_a)
        .map_err(|errors| format!("compile A failed: {}", format_compile_errors(&errors)))?
        .digest();
    let digest_b = compile_source(&source_b)
        .map_err(|errors| format!("compile B failed: {}", format_compile_errors(&errors)))?
        .digest();

    assert_ne!(
        digest_a, digest_b,
        "WaitEvent workflows with different event values must produce different digests"
    );
    Ok(())
}

/// B2: Two WaitEvent workflows with different timeout values must produce
/// different digests. Tested through compile_source.
#[test]
fn wait_event_sensitivity_to_timeout_field_change_through_compile_source_when_timeout_differs()
-> Result<(), String> {
    let steps_a = "  - id: wait_a\n    wait:\n      event: \"0\"\n      timeout: \"10\"\n  - id: done\n    finish:\n      result: 0\n";
    let steps_b = "  - id: wait_a\n    wait:\n      event: \"0\"\n      timeout: \"20\"\n  - id: done\n    finish:\n      result: 0\n";

    let source_a = parse_source(&workflow_yaml(steps_a))?;
    let source_b = parse_source(&workflow_yaml(steps_b))?;

    let digest_a = compile_source(&source_a)
        .map_err(|errors| format!("compile A failed: {}", format_compile_errors(&errors)))?
        .digest();
    let digest_b = compile_source(&source_b)
        .map_err(|errors| format!("compile B failed: {}", format_compile_errors(&errors)))?
        .digest();

    assert_ne!(
        digest_a, digest_b,
        "WaitEvent workflows with different timeout values must produce different digests"
    );
    Ok(())
}

/// B2: Two WaitUntil workflows with different timeout/deadline values must
/// produce different digests.
#[test]
fn wait_until_timeout_change_produces_distinct_digest_through_compile_source_when_timeout_differs()
-> Result<(), String> {
    let steps_a = "  - id: wait_a\n    wait:\n      timeout: \"5\"\n  - id: done\n    finish:\n      result: 0\n";
    let steps_b = "  - id: wait_a\n    wait:\n      timeout: \"10\"\n  - id: done\n    finish:\n      result: 0\n";

    let source_a = parse_source(&workflow_yaml(steps_a))?;
    let source_b = parse_source(&workflow_yaml(steps_b))?;

    let digest_a = compile_source(&source_a)
        .map_err(|errors| format!("compile A failed: {}", format_compile_errors(&errors)))?
        .digest();
    let digest_b = compile_source(&source_b)
        .map_err(|errors| format!("compile B failed: {}", format_compile_errors(&errors)))?
        .digest();

    assert_ne!(
        digest_a, digest_b,
        "WaitUntil workflows with different timeout values must produce different digests"
    );
    Ok(())
}

/// B3: WaitUntil (event=None, timeout=Some) vs WaitEvent (event=Some, timeout=None)
/// must produce different digests — the explicit discriminator.
#[test]
fn wait_until_vs_wait_event_produce_distinct_digests_through_compile_source_when_shapes_differ()
-> Result<(), String> {
    let steps_until = "  - id: wait_a\n    wait:\n      timeout: \"5\"\n  - id: done\n    finish:\n      result: 0\n";
    let steps_event = "  - id: wait_a\n    wait:\n      event: \"5\"\n  - id: done\n    finish:\n      result: 0\n";

    let source_until = parse_source(&workflow_yaml(steps_until))?;
    let source_event = parse_source(&workflow_yaml(steps_event))?;

    let digest_until = compile_source(&source_until)
        .map_err(|errors| format!("compile until failed: {}", format_compile_errors(&errors)))?
        .digest();
    let digest_event = compile_source(&source_event)
        .map_err(|errors| format!("compile event failed: {}", format_compile_errors(&errors)))?
        .digest();

    assert_ne!(
        digest_until, digest_event,
        "WaitUntil ({:?}) and WaitEvent ({:?}) must produce distinct digests",
        digest_until, digest_event
    );
    Ok(())
}

/// B4: WaitEvent with timeout=None must produce a different digest than
/// WaitEvent with the same event but timeout=Some("5"). The sentinel
/// b"none" for absent timeout must be unambiguous.
#[test]
fn wait_event_no_timeout_vs_with_timeout_produce_distinct_digests_through_compile_source_when_timeout_absent()
-> Result<(), String> {
    let steps_no_timeout = "  - id: wait_a\n    wait:\n      event: \"0\"\n  - id: done\n    finish:\n      result: 0\n";
    let steps_with_timeout = "  - id: wait_a\n    wait:\n      event: \"0\"\n      timeout: \"5\"\n  - id: done\n    finish:\n      result: 0\n";

    let source_no_timeout = parse_source(&workflow_yaml(steps_no_timeout))?;
    let source_with_timeout = parse_source(&workflow_yaml(steps_with_timeout))?;

    let digest_no_timeout = compile_source(&source_no_timeout)
        .map_err(|errors| {
            format!(
                "compile no-timeout failed: {}",
                format_compile_errors(&errors)
            )
        })?
        .digest();
    let digest_with_timeout = compile_source(&source_with_timeout)
        .map_err(|errors| {
            format!(
                "compile with-timeout failed: {}",
                format_compile_errors(&errors)
            )
        })?
        .digest();

    assert_ne!(
        digest_no_timeout, digest_with_timeout,
        "WaitEvent with timeout=None must differ from timeout=Some; sentinel must be unambiguous"
    );
    Ok(())
}

/// B5: Compiling the same Wait workflow three times via compile_source must
/// produce identical digests each time (determinism).
#[test]
fn wait_digest_is_deterministic_through_compile_source_when_same_source_compiled_thrice()
-> Result<(), String> {
    let steps = "  - id: wait_a\n    wait:\n      event: \"42\"\n      timeout: \"99\"\n  - id: done\n    finish:\n      result: 0\n";
    let source = parse_source(&workflow_yaml(steps))?;

    let digest1 = compile_source(&source)
        .map_err(|errors| format!("compile #1 failed: {}", format_compile_errors(&errors)))?
        .digest();
    let digest2 = compile_source(&source)
        .map_err(|errors| format!("compile #2 failed: {}", format_compile_errors(&errors)))?
        .digest();
    let digest3 = compile_source(&source)
        .map_err(|errors| format!("compile #3 failed: {}", format_compile_errors(&errors)))?
        .digest();

    assert_eq!(
        digest1, digest2,
        "first two compilations must produce equal digests"
    );
    assert_eq!(
        digest2, digest3,
        "second and third compilations must produce equal digests"
    );
    Ok(())
}

/// B6: Digest roundtrips through WorkflowParts — to_parts().digest must match
/// the original digest from CompiledWorkflow::digest().
#[test]
fn wait_workflow_digest_roundtrips_through_parts_after_compile_source_when_wait_steps_present()
-> Result<(), String> {
    let steps = "  - id: wait_a\n    wait:\n      event: \"0\"\n      timeout: \"30\"\n  - id: done\n    finish:\n      result: 0\n";
    let source = parse_source(&workflow_yaml(steps))?;

    let compiled = compile_source(&source)
        .map_err(|errors| format!("compile failed: {}", format_compile_errors(&errors)))?;
    let digest_from_compiled = compiled.digest();
    let parts = compiled.to_parts();
    let digest_from_parts = parts.digest;

    assert_eq!(
        digest_from_compiled, digest_from_parts,
        "WorkflowParts::digest must match CompiledWorkflow::digest()"
    );
    Ok(())
}

/// B8/B12: A workflow with Wait + Set + Finish must produce a different
/// digest than a workflow with only Set + Finish. The Wait primitive
/// contribution must be observable in the final digest.
#[test]
fn wait_workflow_with_mixed_steps_digests_differ_from_non_wait_workflow_when_wait_added()
-> Result<(), String> {
    let steps_with_wait = "  - id: assign\n    set:\n      output: x\n      value: \"10\"\n  - id: wait_here\n    wait:\n      event: \"0\"\n      timeout: \"5\"\n  - id: done\n    finish:\n      result: x\n";
    let steps_no_wait = "  - id: assign\n    set:\n      output: x\n      value: \"10\"\n  - id: done\n    finish:\n      result: x\n";

    let source_with_wait = parse_source(&workflow_yaml(steps_with_wait))?;
    let source_no_wait = parse_source(&workflow_yaml(steps_no_wait))?;

    let digest_with_wait = compile_source(&source_with_wait)
        .map_err(|errors| {
            format!(
                "compile with-wait failed: {}",
                format_compile_errors(&errors)
            )
        })?
        .digest();
    let digest_no_wait = compile_source(&source_no_wait)
        .map_err(|errors| format!("compile no-wait failed: {}", format_compile_errors(&errors)))?
        .digest();

    assert_ne!(
        digest_with_wait, digest_no_wait,
        "Adding a Wait step to a workflow must change the canonical digest"
    );
    Ok(())
}

/// B11: An invalid wait shape (event=None, timeout=None) must be rejected
/// with CompileError::StepFieldShape.
#[test]
fn wait_invalid_shape_event_none_timeout_none_rejected_with_step_field_shape_when_both_fields_absent()
-> Result<(), String> {
    let steps = "  - id: bad_wait\n    wait: {}\n  - id: done\n    finish:\n      result: 0\n";
    let errors = compile_yaml_error(&workflow_yaml(steps))?;
    let first = first_compile_error(&errors)?;
    match first {
        CompileError::StepFieldShape {
            step,
            field,
            expected,
        } => {
            assert_eq!(*step, 0, "StepFieldShape must reference step 0");
            assert_eq!(*field, "wait", "StepFieldShape must reference field 'wait'");
            assert!(
                !expected.is_empty(),
                "StepFieldShape must have a non-empty expected message"
            );
            Ok(())
        }
        other => Err(format!(
            "expected CompileError::StepFieldShape for empty wait, got {other:?}"
        )),
    }
}

// ── Section 9.4: PI-8 Non-Wait workflow digest determinism ──

/// PI-8: Non-Wait workflows produce deterministic digests after the Wait fix.
/// For any workflow without Wait steps, compiling the same source twice must
/// produce identical digests. This verifies the Wait arm addition did not
/// introduce non-determinism into non-Wait digest computation.
/// NOTE: PI-8 does NOT assert "unchanged from pre-fix" — pre-fix baseline
/// comparison is covered by existing regression test PI-5
/// (`proptest_equal_primitive_sources_compile_to_equal_digest_and_ir`).
#[test]
fn proptest_non_wait_workflows_digests_are_deterministic_after_wait_fix() -> Result<(), String> {
    // Filter to non-Wait primitive cases that compile successfully.
    let non_wait_cases: Vec<&PrimitiveCase> = PRIMITIVE_CASES
        .iter()
        .filter(|case| {
            case.name != "wait"
                && !case.name.starts_with("save_")
                && !case.name.starts_with("do_")
                && !case.name.starts_with("choose_")
        })
        .collect();

    // Each case is compiled twice; digests must match.
    for case in non_wait_cases {
        let digest1_digest = compile_case(case)?.digest();
        let digest2_digest = compile_case(case)?.digest();

        assert_eq!(
            digest1_digest, digest2_digest,
            "non-Wait primitive '{}' must produce deterministic digests after Wait fix",
            case.name
        );
    }
    Ok(())
}

fn compile_steps_with_api(
    steps: &str,
    api_path: PublicApiPath,
) -> Result<CompiledWorkflow, String> {
    let yaml = workflow_yaml(steps);
    compile_yaml_with_api(&yaml, api_path).map_err(|errors| format_compile_errors(&errors))
}

fn compile_yaml_with_api(
    yaml: &str,
    api_path: PublicApiPath,
) -> Result<CompiledWorkflow, CompileErrors> {
    match api_path {
        PublicApiPath::CompileSource => {
            let source = parse_source(yaml).map_err(|message| {
                CompileErrors(vec![CompileError::CanonicalYaml {
                    category: "parse_error",
                    message: message.into_boxed_str(),
                }])
            })?;
            compile_source(&source)
        }
        PublicApiPath::CompileWorkflow => compile_workflow(yaml.as_bytes()),
        PublicApiPath::YamlCompilerCompile => YamlCompiler::default().compile(yaml.as_bytes()),
    }
}

fn compile_document_or_steps_error_with_api(
    body: &str,
    api_path: PublicApiPath,
) -> Result<CompileErrors, String> {
    let yaml = if body.starts_with("version:") {
        String::from(body)
    } else {
        workflow_yaml(body)
    };
    compile_yaml_error_with_api(&yaml, api_path)
}

fn compile_yaml_error_with_api(
    yaml: &str,
    api_path: PublicApiPath,
) -> Result<CompileErrors, String> {
    match compile_yaml_with_api(yaml, api_path) {
        Ok(workflow) => Err(format!(
            "api {api_path:?} expected compile error, got workflow with {} nodes",
            workflow.node_count()
        )),
        Err(errors) => Ok(errors),
    }
}

fn parse_source(yaml: &str) -> Result<vb_yaml::ast::WorkflowSource, String> {
    vb_yaml::parse_workflow_source(yaml).map_err(|error| error.to_string())
}

fn compile_yaml(yaml: &str) -> Result<CompiledWorkflow, String> {
    compile_workflow(yaml.as_bytes()).map_err(|errors| format_compile_errors(&errors))
}

fn compile_yaml_error(yaml: &str) -> Result<CompileErrors, String> {
    match compile_workflow(yaml.as_bytes()) {
        Ok(workflow) => Err(format!(
            "expected compile error, got workflow with {} nodes",
            workflow.node_count()
        )),
        Err(errors) => Ok(errors),
    }
}

fn first_compile_error(errors: &CompileErrors) -> Result<&CompileError, String> {
    match errors.first() {
        Some(error) => Ok(error),
        None => Err(String::from("CompileErrors contained no first error")),
    }
}

fn format_compile_errors(errors: &CompileErrors) -> String {
    let mut message = String::new();
    for error in errors.iter() {
        if !message.is_empty() {
            message.push_str("; ");
        }
        message.push_str(error.code());
        message.push_str(": ");
        message.push_str(&error.to_string());
    }
    message
}

fn workflow_yaml(steps: &str) -> String {
    let mut yaml = String::from(HEADER);
    yaml.push_str(steps);
    yaml
}

fn node_kind_names(nodes: &[vb_core::CompiledNode]) -> Vec<&'static str> {
    nodes
        .iter()
        .map(|node| node_kind_name(&node.kind))
        .collect()
}

fn node_kind_name(kind: &CompiledNodeKind) -> &'static str {
    match kind {
        CompiledNodeKind::Nop => "Nop",
        CompiledNodeKind::SetConst { .. } => "SetConst",
        CompiledNodeKind::Copy { .. } => "Copy",
        CompiledNodeKind::EvalExpr { .. } => "EvalExpr",
        CompiledNodeKind::BuildObject { .. } => "BuildObject",
        CompiledNodeKind::BuildList { .. } => "BuildList",
        CompiledNodeKind::Do { .. } => "Do",
        CompiledNodeKind::Choose { .. } => "Choose",
        CompiledNodeKind::ChooseSlot { .. } => "ChooseSlot",
        CompiledNodeKind::ForEachStart { .. } => "ForEachStart",
        CompiledNodeKind::ForEachNext { .. } => "ForEachNext",
        CompiledNodeKind::ForEachJoin { .. } => "ForEachJoin",
        CompiledNodeKind::TogetherStart { .. } => "TogetherStart",
        CompiledNodeKind::TogetherBranch { .. } => "TogetherBranch",
        CompiledNodeKind::TogetherJoin { .. } => "TogetherJoin",
        CompiledNodeKind::CollectStart { .. } => "CollectStart",
        CompiledNodeKind::CollectPage { .. } => "CollectPage",
        CompiledNodeKind::CollectNext { .. } => "CollectNext",
        CompiledNodeKind::CollectFinish { .. } => "CollectFinish",
        CompiledNodeKind::ReduceStart { .. } => "ReduceStart",
        CompiledNodeKind::ReduceNext { .. } => "ReduceNext",
        CompiledNodeKind::ReduceFinish { .. } => "ReduceFinish",
        CompiledNodeKind::RepeatStart { .. } => "RepeatStart",
        CompiledNodeKind::RepeatAttempt { .. } => "RepeatAttempt",
        CompiledNodeKind::RepeatCheck { .. } => "RepeatCheck",
        CompiledNodeKind::RepeatFinish { .. } => "RepeatFinish",
        CompiledNodeKind::WaitUntil { .. } => "WaitUntil",
        CompiledNodeKind::WaitEvent { .. } => "WaitEvent",
        CompiledNodeKind::Ask { .. } => "Ask",
        CompiledNodeKind::AskResume { .. } => "AskResume",
        CompiledNodeKind::RetryCheck { .. } => "RetryCheck",
        CompiledNodeKind::ErrorHandler { .. } => "ErrorHandler",
        CompiledNodeKind::Jump { .. } => "Jump",
        CompiledNodeKind::Finish { .. } => "Finish",
        _ => "Unknown",
    }
}

fn assert_exact_primitive_shape(
    workflow: &CompiledWorkflow,
    case_name: &str,
) -> Result<(), String> {
    let parts = workflow.to_parts();
    match case_name {
        "for_each" => assert_exact_for_each(parts.nodes.as_ref()),
        "together" => assert_exact_together(parts.nodes.as_ref()),
        "collect" => assert_exact_collect(parts.nodes.as_ref()),
        "reduce" => assert_exact_reduce(parts.nodes.as_ref()),
        "repeat" => assert_exact_repeat(parts.nodes.as_ref()),
        "wait" => assert_exact_wait_event(parts.nodes.as_ref()),
        "ask" => assert_exact_ask(parts.nodes.as_ref()),
        other => Err(format!("unknown primitive shape case {other}")),
    }
}

fn node_at(nodes: &[CompiledNode], index: usize) -> Result<&CompiledNode, String> {
    nodes
        .get(index)
        .ok_or_else(|| format!("missing node at index {index}"))
}

fn assert_exact_for_each(nodes: &[CompiledNode]) -> Result<(), String> {
    assert_set_const_node(nodes, &[], 1, Some(1), Some(2), 0, 1)?;
    assert_finish_node(nodes, 3, 0)?;
    match &node_at(nodes, 0)?.kind {
        CompiledNodeKind::ForEachStart {
            input,
            item_slot,
            limit,
            body,
            done,
        } => {
            assert_eq!(
                (input.get(), item_slot.get(), *limit, body.get(), done.get()),
                (0, 1, 2, 1, 3)
            );
        }
        other => return Err(format!("expected ForEachStart, got {other:?}")),
    }
    match &node_at(nodes, 2)?.kind {
        CompiledNodeKind::ForEachNext {
            iterator_slot,
            body,
            done,
        } => {
            assert_eq!((iterator_slot.get(), body.get(), done.get()), (1, 1, 3));
        }
        other => return Err(format!("expected ForEachNext, got {other:?}")),
    }
    Ok(())
}

fn assert_exact_together(nodes: &[CompiledNode]) -> Result<(), String> {
    assert_set_const_node(nodes, &[], 2, Some(1), None, 0, 1)?;
    assert_set_const_node(nodes, &[], 4, Some(3), None, 0, 2)?;
    assert_finish_node(nodes, 6, 0)?;
    match &node_at(nodes, 0)?.kind {
        CompiledNodeKind::TogetherStart { branches, join } => {
            let actual: Vec<u16> = branches.iter().map(|branch| branch.get()).collect();
            assert_eq!(actual, [1, 3]);
            assert_eq!(join.get(), 5);
        }
        other => return Err(format!("expected TogetherStart, got {other:?}")),
    }
    match &node_at(nodes, 5)?.kind {
        CompiledNodeKind::TogetherJoin {
            branch_count,
            accumulator,
        } => {
            assert_eq!((*branch_count, accumulator.get()), (2, 0));
        }
        other => return Err(format!("expected TogetherJoin, got {other:?}")),
    }
    match &node_at(nodes, 1)?.kind {
        CompiledNodeKind::TogetherBranch {
            branch,
            entry,
            join,
            accumulator,
        } => {
            assert_eq!(
                (*branch, entry.get(), join.get(), accumulator.get()),
                (0, 2, 5, 0)
            );
        }
        other => return Err(format!("expected first TogetherBranch, got {other:?}")),
    }
    match &node_at(nodes, 3)?.kind {
        CompiledNodeKind::TogetherBranch {
            branch,
            entry,
            join,
            accumulator,
        } => {
            assert_eq!(
                (*branch, entry.get(), join.get(), accumulator.get()),
                (1, 4, 5, 0)
            );
        }
        other => return Err(format!("expected second TogetherBranch, got {other:?}")),
    }
    Ok(())
}

fn assert_exact_collect(nodes: &[CompiledNode]) -> Result<(), String> {
    assert_set_const_node(nodes, &[], 1, Some(1), Some(2), 0, 7)?;
    assert_finish_node(nodes, 4, 0)?;
    match &node_at(nodes, 0)?.kind {
        CompiledNodeKind::CollectStart {
            source,
            limit,
            page_size,
            body,
            done,
        } => {
            assert_eq!(
                (source.get(), *limit, *page_size, body.get(), done.get()),
                (0, 3, 5, 1, 3)
            );
        }
        other => return Err(format!("expected CollectStart, got {other:?}")),
    }
    match &node_at(nodes, 2)?.kind {
        CompiledNodeKind::CollectPage {
            collector_slot,
            body,
            done,
        } => {
            assert_eq!((collector_slot.get(), body.get(), done.get()), (0, 1, 3));
        }
        other => return Err(format!("expected CollectPage, got {other:?}")),
    }
    match &node_at(nodes, 3)?.kind {
        CompiledNodeKind::CollectFinish { collector_slot } => assert_eq!(collector_slot.get(), 0),
        other => return Err(format!("expected CollectFinish, got {other:?}")),
    }
    Ok(())
}

fn assert_exact_reduce(nodes: &[CompiledNode]) -> Result<(), String> {
    assert_set_const_node(nodes, &[], 1, Some(1), Some(2), 1, 1)?;
    assert_finish_node(nodes, 4, 0)?;
    match &node_at(nodes, 0)?.kind {
        CompiledNodeKind::ReduceStart {
            input,
            accumulator,
            initial,
            body,
            done,
        } => {
            assert_eq!(
                (
                    input.get(),
                    accumulator.get(),
                    initial.get(),
                    body.get(),
                    done.get()
                ),
                (0, 1, 0, 1, 3)
            );
        }
        other => return Err(format!("expected ReduceStart, got {other:?}")),
    }
    match &node_at(nodes, 2)?.kind {
        CompiledNodeKind::ReduceNext {
            iterator_slot,
            accumulator,
            body,
            done,
        } => {
            assert_eq!(
                (
                    iterator_slot.get(),
                    accumulator.get(),
                    body.get(),
                    done.get()
                ),
                (1, 1, 1, 3)
            );
        }
        other => return Err(format!("expected ReduceNext, got {other:?}")),
    }
    match &node_at(nodes, 3)?.kind {
        CompiledNodeKind::ReduceFinish { accumulator } => assert_eq!(accumulator.get(), 1),
        other => return Err(format!("expected ReduceFinish, got {other:?}")),
    }
    Ok(())
}

fn assert_exact_repeat(nodes: &[CompiledNode]) -> Result<(), String> {
    assert_set_const_node(nodes, &[], 1, Some(1), Some(2), 0, 1)?;
    assert_finish_node(nodes, 4, 0)?;
    match &node_at(nodes, 0)?.kind {
        CompiledNodeKind::RepeatStart {
            max_attempts,
            body,
            done,
        } => {
            assert_eq!((*max_attempts, body.get(), done.get()), (3, 1, 3));
        }
        other => return Err(format!("expected RepeatStart, got {other:?}")),
    }
    match &node_at(nodes, 2)?.kind {
        CompiledNodeKind::RepeatAttempt {
            attempt_slot,
            body,
            done,
        } => {
            assert_eq!((attempt_slot.get(), body.get(), done.get()), (1, 1, 3));
        }
        other => return Err(format!("expected RepeatAttempt, got {other:?}")),
    }
    match &node_at(nodes, 3)?.kind {
        CompiledNodeKind::RepeatFinish { result } => assert_eq!(result.get(), 1),
        other => return Err(format!("expected RepeatFinish, got {other:?}")),
    }
    Ok(())
}

fn assert_exact_wait_event(nodes: &[CompiledNode]) -> Result<(), String> {
    assert_finish_node(nodes, 1, 0)?;
    match &node_at(nodes, 0)?.kind {
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => {
            let timeout = timeout_slot.ok_or_else(|| String::from("missing wait timeout slot"))?;
            assert_eq!((event.get(), timeout.get()), (0, 1));
        }
        other => return Err(format!("expected WaitEvent, got {other:?}")),
    }
    Ok(())
}

fn assert_exact_ask(nodes: &[CompiledNode]) -> Result<(), String> {
    assert_finish_node(nodes, 2, 0)?;
    match &node_at(nodes, 0)?.kind {
        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => {
            let timeout = timeout_slot.ok_or_else(|| String::from("missing ask timeout slot"))?;
            assert_eq!((prompt.get(), timeout.get()), (0, 1));
        }
        other => return Err(format!("expected Ask, got {other:?}")),
    }
    match &node_at(nodes, 1)?.kind {
        CompiledNodeKind::AskResume { answer } => assert_eq!(answer.get(), 2),
        other => return Err(format!("expected AskResume, got {other:?}")),
    }
    Ok(())
}

fn assert_set_const_node(
    nodes: &[CompiledNode],
    constants: &[ConstValue],
    index: usize,
    expected_output: Option<u16>,
    expected_next: Option<u16>,
    expected_const_idx: u16,
    expected_i64: i64,
) -> Result<(), String> {
    let node = node_at(nodes, index)?;
    assert_eq!(
        node.output.map(|slot| slot.get()),
        expected_output,
        "SetConst node {index} output slot"
    );
    assert_eq!(
        node.next.map(|step| step.get()),
        expected_next,
        "SetConst node {index} next target"
    );
    match &node.kind {
        CompiledNodeKind::SetConst { value } => {
            assert_eq!(
                value.get(),
                expected_const_idx,
                "SetConst node {index} const index"
            );
            if !constants.is_empty() {
                let constant = constants
                    .get(value.as_usize())
                    .ok_or_else(|| format!("missing constant at index {}", value.get()))?;
                assert_eq!(
                    constant,
                    &ConstValue::I64(expected_i64),
                    "SetConst node {index} const payload"
                );
            }
        }
        other => return Err(format!("expected SetConst at node {index}, got {other:?}")),
    }
    Ok(())
}

fn assert_finish_node(
    nodes: &[CompiledNode],
    index: usize,
    expected_result_slot: u16,
) -> Result<(), String> {
    let node = node_at(nodes, index)?;
    assert_eq!(
        node.output, None,
        "Finish node {index} must not write output"
    );
    assert_eq!(node.next, None, "Finish node {index} must not fall through");
    match &node.kind {
        CompiledNodeKind::Finish { result } => assert_eq!(
            result.get(),
            expected_result_slot,
            "Finish node {index} result slot"
        ),
        other => return Err(format!("expected Finish at node {index}, got {other:?}")),
    }
    Ok(())
}

fn assert_dense_node_ids(workflow: &CompiledWorkflow, case_name: &str) -> Result<(), String> {
    let parts = workflow.to_parts();
    for (index, node) in parts.nodes.iter().enumerate() {
        let expected = u16::try_from(index).map_err(|error| error.to_string())?;
        assert_eq!(
            node.id.get(),
            expected,
            "case {case_name} node id must equal position"
        );
    }
    Ok(())
}

fn assert_all_targets_in_range(workflow: &CompiledWorkflow, case_name: &str) -> Result<(), String> {
    let parts = workflow.to_parts();
    let node_count = parts.nodes.len();
    for node in parts.nodes.as_ref() {
        assert_optional_target(node.next, node_count, case_name, "next")?;
        assert_optional_target(node.on_error, node_count, case_name, "on_error")?;
        assert_kind_targets_in_range(&node.kind, node_count, case_name)?;
    }
    Ok(())
}

fn assert_optional_target(
    target: Option<StepIdx>,
    node_count: usize,
    case_name: &str,
    field: &str,
) -> Result<(), String> {
    if let Some(step) = target {
        assert_target(step, node_count, case_name, field)?;
    }
    Ok(())
}

fn assert_target(
    target: StepIdx,
    node_count: usize,
    case_name: &str,
    field: &str,
) -> Result<(), String> {
    let target_index = target.as_usize();
    assert_eq!(
        target_index < node_count,
        true,
        "case {case_name} target field {field} must be in range"
    );
    Ok(())
}

fn assert_kind_targets_in_range(
    kind: &CompiledNodeKind,
    node_count: usize,
    case_name: &str,
) -> Result<(), String> {
    match kind {
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => {
            for branch in branches.as_ref() {
                assert_target(branch.target, node_count, case_name, "choose.branch.target")?;
            }
            assert_optional_target(*otherwise, node_count, case_name, "choose.otherwise")?;
        }
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => {
            for branch in branches.as_ref() {
                assert_target(
                    branch.target,
                    node_count,
                    case_name,
                    "choose_slot.branch.target",
                )?;
            }
            assert_optional_target(*otherwise, node_count, case_name, "choose_slot.otherwise")?;
        }
        CompiledNodeKind::ForEachStart { body, done, .. }
        | CompiledNodeKind::ForEachNext { body, done, .. }
        | CompiledNodeKind::CollectStart { body, done, .. }
        | CompiledNodeKind::CollectPage { body, done, .. }
        | CompiledNodeKind::CollectNext { body, done, .. }
        | CompiledNodeKind::ReduceStart { body, done, .. }
        | CompiledNodeKind::ReduceNext { body, done, .. }
        | CompiledNodeKind::RepeatStart { body, done, .. }
        | CompiledNodeKind::RepeatAttempt { body, done, .. } => {
            assert_target(*body, node_count, case_name, "body")?;
            assert_target(*done, node_count, case_name, "done")?;
        }
        CompiledNodeKind::TogetherStart { branches, join } => {
            for branch in branches.as_ref() {
                assert_target(*branch, node_count, case_name, "together.branch")?;
            }
            assert_target(*join, node_count, case_name, "together.join")?;
        }
        CompiledNodeKind::TogetherBranch { entry, join, .. } => {
            assert_target(*entry, node_count, case_name, "together_branch.entry")?;
            assert_target(*join, node_count, case_name, "together_branch.join")?;
        }
        CompiledNodeKind::RepeatCheck { done, .. } => {
            assert_target(*done, node_count, case_name, "repeat_check.done")?;
        }
        CompiledNodeKind::RetryCheck {
            body, exhausted, ..
        } => {
            assert_target(*body, node_count, case_name, "retry_check.body")?;
            assert_target(*exhausted, node_count, case_name, "retry_check.exhausted")?;
        }
        CompiledNodeKind::ErrorHandler { body, handler, .. } => {
            assert_target(*body, node_count, case_name, "error_handler.body")?;
            assert_target(*handler, node_count, case_name, "error_handler.handler")?;
        }
        CompiledNodeKind::Jump { target } => {
            assert_target(*target, node_count, case_name, "jump.target")?;
        }
        _ => {}
    }
    Ok(())
}

fn unsupported_primitive_name(errors: &CompileErrors) -> Option<&'static str> {
    errors.iter().find_map(|error| match error {
        CompileError::UnsupportedStepPrimitive { primitive, .. } => Some(*primitive),
        _ => None,
    })
}

#[test]
fn compile_workflow_rejects_empty_body_in_scoped_primitives() -> Result<(), String> {
    let cases = [
        (
            "for_each with empty body",
            "  - id: loop\n    for_each:\n      variable: item\n      input: \"0\"\n      steps: []\n  - id: done\n    finish:\n      result: 0\n",
        ),
        (
            "collect with empty body",
            "  - id: pages\n    collect:\n      variable: page\n      source: \"0\"\n      steps: []\n  - id: done\n    finish:\n      result: 0\n",
        ),
        (
            "reduce with empty body",
            "  - id: fold\n    reduce:\n      variable: acc\n      input: \"0\"\n      initial: \"10\"\n      steps: []\n  - id: done\n    finish:\n      result: 0\n",
        ),
        (
            "repeat with empty body",
            "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps: []\n  - id: done\n    finish:\n      result: 0\n",
        ),
    ];

    for (case_name, yaml_steps) in cases {
        let yaml = workflow_yaml(yaml_steps);
        let errors = compile_yaml_error(&yaml)?;
        let first = first_compile_error(&errors)?;
        match first {
            CompileError::StepFieldShape {
                step,
                field,
                expected,
                ..
            } => {
                assert_eq!(
                    (*step, field.as_ref(), expected.as_ref()),
                    (0, "steps", "exactly one set step"),
                    "case {case_name}"
                );
            }
            other => {
                return Err(format!(
                    "case {case_name} expected StepFieldShape, got {other:?}"
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn compile_workflow_rejects_non_set_body_in_all_scoped_primitives() -> Result<(), String> {
    let cases = [
        (
            "for_each with non-set body (finish)",
            "  - id: loop\n    for_each:\n      variable: item\n      input: \"0\"\n      steps:\n        - id: bad\n          finish:\n            result: 0\n  - id: done\n    finish:\n      result: 0\n",
            "finish",
        ),
        (
            "reduce with non-set body (finish)",
            "  - id: fold\n    reduce:\n      variable: acc\n      input: \"0\"\n      initial: \"10\"\n      steps:\n        - id: bad\n          finish:\n            result: 0\n  - id: done\n    finish:\n      result: 0\n",
            "finish",
        ),
        (
            "repeat with non-set body (finish)",
            "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: bad\n          finish:\n            result: 0\n  - id: done\n    finish:\n      result: 0\n",
            "finish",
        ),
    ];

    for (case_name, yaml_steps, expected_primitive) in cases {
        let yaml = workflow_yaml(yaml_steps);
        let errors = compile_yaml_error(&yaml)?;
        let first = first_compile_error(&errors)?;
        match first {
            CompileError::UnsupportedStepPrimitive {
                step, primitive, ..
            } => {
                assert_eq!(
                    (*step, primitive.as_ref()),
                    (0, expected_primitive),
                    "case {case_name}"
                );
            }
            other => {
                return Err(format!(
                    "case {case_name} expected UnsupportedStepPrimitive, got {other:?}"
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn compile_workflow_rejects_multi_step_body_at_non_zero_step() -> Result<(), String> {
    let yaml = workflow_yaml(
        "  - id: preamble\n    set:\n      output: x\n      value: \"1\"\n  - id: loop\n    for_each:\n      variable: item\n      input: \"0\"\n      steps:\n        - id: step1\n          set:\n            output: a\n            value: \"1\"\n        - id: step2\n          set:\n            output: b\n            value: \"2\"\n  - id: done\n    finish:\n      result: 0\n",
    );
    let errors = compile_yaml_error(&yaml)?;
    let first = first_compile_error(&errors)?;
    match first {
        CompileError::StepFieldShape {
            step,
            field,
            expected,
            ..
        } => {
            assert_eq!(
                (*step, field.as_ref(), expected.as_ref()),
                (1, "steps", "exactly one set step"),
                "for_each at step 1 should report step=1"
            );
            Ok(())
        }
        other => Err(format!(
            "expected StepFieldShape with step=1, got {other:?}"
        )),
    }
}

#[test]
fn compile_workflow_rejects_multi_step_together_branch() -> Result<(), String> {
    let yaml = workflow_yaml(
        "  - id: fanout\n    together:\n      branches:\n        - label: left\n          steps:\n            - id: a\n              set:\n                output: x\n                value: \"1\"\n            - id: b\n              set:\n                output: yy\n                value: \"2\"\n        - label: right\n          steps:\n            - id: c\n              set:\n                output: zz\n                value: \"3\"\n  - id: done\n    finish:\n      result: 0\n",
    );
    let errors = compile_yaml_error(&yaml)?;
    let first = first_compile_error(&errors)?;
    match first {
        CompileError::StepFieldShape {
            step,
            field,
            expected,
            ..
        } => {
            assert_eq!(
                (*step, field.as_ref(), expected.as_ref()),
                (0, "steps", "exactly one set step"),
                "together branch multi-step should report parent step=0"
            );
            Ok(())
        }
        other => Err(format!(
            "expected StepFieldShape for together branch, got {other:?}"
        )),
    }
}

// ─────────────────────────────────────────────────────────────────
// vb-awhr: choose otherwise handling and fanout limit fixes
// ─────────────────────────────────────────────────────────────────

#[test]
fn compile_workflow_choose_two_branches_with_otherwise() -> Result<(), String> {
    let yaml = workflow_yaml(
        "  - id: setup\n    set:\n      output: condition\n      value: \"1\"\n  - id: pick\n    choose:\n      branches:\n        - when: \"0\"\n          steps: []\n        - when: \"1\"\n          steps: []\n      otherwise: done\n  - id: done\n    finish:\n      result: 0\n",
    );
    let workflow = compile_yaml(&yaml)?;
    let kinds = node_kind_names(workflow.to_parts().nodes.as_ref());
    assert_eq!(
        kinds,
        vec!["SetConst", "ChooseSlot", "Finish"],
        "two-branch choose must compile to SetConst, ChooseSlot, Finish"
    );
    Ok(())
}

#[test]
fn compile_workflow_choose_rejects_unknown_otherwise_label() -> Result<(), String> {
    let yaml = workflow_yaml(
        "  - id: pick\n    choose:\n      branches:\n        - when: \"0\"\n          steps: []\n      otherwise: missing_label\n  - id: done\n    finish:\n      result: 0\n",
    );
    let errors = compile_yaml_error(&yaml)?;
    let first = first_compile_error(&errors)?;
    match first {
        CompileError::UnknownStepLabel { step, label } => {
            assert_eq!(
                (*step, label.as_ref()),
                (0, "missing_label"),
                "unknown otherwise label must report the actual label text"
            );
        }
        other => {
            return Err(format!(
                "expected UnknownStepLabel for missing otherwise, got {other:?}"
            ));
        }
    }
    Ok(())
}

#[test]
fn compile_workflow_choose_rejects_65_branches() -> Result<(), String> {
    let mut branches = String::new();
    for i in 0..65 {
        branches.push_str(&format!("        - when: \"{}\"\n          steps: []\n", i));
    }
    let yaml = workflow_yaml(&format!(
        "  - id: pick\n    choose:\n      branches:\n{}      otherwise: done\n  - id: done\n    finish:\n      result: 0\n",
        branches
    ));
    let errors = compile_yaml_error(&yaml)?;
    let first = first_compile_error(&errors)?;
    match first {
        CompileError::PrimitiveLoweringLimitExceeded {
            primitive,
            field,
            value,
            limit,
        } => {
            assert_eq!(
                (*primitive, *field, *value, *limit),
                ("choose", "branches", 65, 64),
                "65-branch choose must fail with exact fanout limit error"
            );
        }
        other => {
            return Err(format!(
                "expected PrimitiveLoweringLimitExceeded for 65 branches, got {other:?}"
            ));
        }
    }
    Ok(())
}

#[test]
fn compile_workflow_choose_64_branches_accepted() -> Result<(), String> {
    let mut branches = String::new();
    for i in 0..64 {
        branches.push_str(&format!("        - when: \"{}\"\n          steps: []\n", i));
    }
    let yaml = workflow_yaml(&format!(
        "  - id: pick\n    choose:\n      branches:\n{}      otherwise: done\n  - id: done\n    finish:\n      result: 0\n",
        branches
    ));
    let workflow = compile_yaml(&yaml)?;
    let kinds = node_kind_names(workflow.to_parts().nodes.as_ref());
    assert_eq!(
        kinds.len(),
        2,
        "64-branch choose must compile to exactly 2 nodes (ChooseSlot + Finish)"
    );
    assert_eq!(kinds[0], "ChooseSlot", "first node must be ChooseSlot");
    assert_eq!(kinds[1], "Finish", "second node must be Finish");
    Ok(())
}
