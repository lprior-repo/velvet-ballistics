use proptest::prelude::*;
use vb_compile::{
    CompileError, CompileErrors, YamlCompiler, compile_source, compile_workflow, lower_steps_to_ir,
    lower_together,
};
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstValue, StepIdx, WorkflowError,
};

const HEADER: &str =
    "version: velvet-ballastics/v1\nname: primitive-lowering\nwhen:\n  manual: {}\nsteps:\n";

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
        name: "parallel",
        yaml_steps: "  - id: fanout\n    parallel:\n      branches:\n        - label: left\n          steps:\n            - id: left_set\n              set:\n                output: left\n                value: \"1\"\n        - label: right\n          steps:\n            - id: right_set\n              set:\n                output: right\n                value: \"2\"\n  - id: done\n    finish:\n      result: 0\n",
        expected_kinds: TOGETHER_KINDS,
        expected_slot_count: 1,
    },
    PrimitiveCase {
        name: "collect",
        yaml_steps: "  - id: collect_pages\n    collect:\n      variable: page\n      source: \"0\"\n      pages: 3\n      items: 5\n      steps:\n        - id: remember_page\n          set:\n            output: page_seen\n            value: \"7\"\n  - id: done\n    finish:\n      result: 0\n",
        expected_kinds: COLLECT_KINDS,
        expected_slot_count: 1,
    },
    PrimitiveCase {
        name: "aggregate",
        yaml_steps: "  - id: fold\n    aggregate:\n      variable: acc\n      input: \"0\"\n      initial: \"10\"\n      steps:\n        - id: add_one\n          set:\n            output: acc_out\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
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
            "parallel",
            "  - id: fanout\n    parallel:\n      branches:\n        - label: \"\"\n          steps: []\n  - id: done\n    finish:\n      result: 0\n",
            ExpectedShapeError::CanonicalYamlField("parallel.branches[].label"),
        ),
        (
            "collect",
            "  - id: collect_pages\n    collect:\n      variable: page\n      source: \"\"\n      steps: []\n  - id: done\n    finish:\n      result: 0\n",
            ExpectedShapeError::CanonicalYamlField("collect.source"),
        ),
        (
            "aggregate",
            "  - id: fold\n    aggregate:\n      variable: acc\n      input: \"0\"\n      initial: \"\"\n      steps: []\n  - id: done\n    finish:\n      result: 0\n",
            ExpectedShapeError::CanonicalYamlField("aggregate.initial"),
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
    let cases = [
        (
            "save",
            "  - id: save_value\n    save:\n      output: stored\n      value: \"1\"\n  - id: done\n    finish:\n      result: stored\n",
            "save",
        ),
        (
            "do",
            "  - id: call_action\n    do:\n      action: action.name\n      input: \"0\"\n  - id: done\n    finish:\n      result: 0\n",
            "do",
        ),
        (
            "choose",
            "  - id: branch\n    choose:\n      branches:\n        - when: \"0\"\n          steps: []\n  - id: done\n    finish:\n      result: 0\n",
            "choose",
        ),
    ];

    for (case_name, yaml_steps, expected_primitive) in cases {
        let yaml = workflow_yaml(yaml_steps);
        let errors = compile_yaml_error(&yaml)?;
        let first = first_compile_error(&errors)?;
        match first {
            CompileError::UnsupportedStepPrimitive { step, primitive } => {
                assert_eq!(
                    (*step, *primitive),
                    (0, expected_primitive),
                    "case {case_name} returned wrong unsupported primitive"
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
fn public_compile_apis_return_unsupported_step_primitive_for_save_do_choose_only()
-> Result<(), String> {
    let cases = [
        (
            "save",
            "  - id: save_value\n    save:\n      output: stored\n      value: \"1\"\n  - id: done\n    finish:\n      result: stored\n",
            "save",
        ),
        (
            "do",
            "  - id: call_action\n    do:\n      action: action.name\n      input: \"0\"\n  - id: done\n    finish:\n      result: 0\n",
            "do",
        ),
        (
            "choose",
            "  - id: branch\n    choose:\n      branches:\n        - when: \"0\"\n          steps: []\n  - id: done\n    finish:\n      result: 0\n",
            "choose",
        ),
    ];

    for api_path in PUBLIC_API_PATHS {
        for (case_name, yaml_steps, expected_primitive) in cases {
            let errors = compile_steps_error_with_api(yaml_steps, *api_path)?;
            let first = first_compile_error(&errors)?;
            assert_unsupported_step_primitive(case_name, *api_path, first, 0, expected_primitive)?;
        }
    }
    Ok(())
}

#[test]
fn compile_source_returns_exact_error_variants_for_contract_taxonomy() -> Result<(), String> {
    let cases = [
        (
            "empty_steps",
            "version: velvet-ballastics/v1\nname: empty\nwhen:\n  manual: {}\nsteps: []\n",
            ExpectedCompileError::EmptySteps,
        ),
        (
            "top_level_inputs",
            "version: velvet-ballastics/v1\nname: inputs\nwhen:\n  manual: {}\ninputs:\n  account:\n    type: string\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
            ExpectedCompileError::UnsupportedTopLevelDeclaration("inputs"),
        ),
        (
            "top_level_result",
            "version: velvet-ballastics/v1\nname: result\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\nresult:\n  ok: true\n",
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
            "version: velvet-ballastics/v1\nname: empty\nwhen:\n  manual: {}\nsteps: []\n",
            ExpectedCompileError::EmptySteps,
        ),
        (
            "top_level_inputs",
            "version: velvet-ballastics/v1\nname: inputs\nwhen:\n  manual: {}\ninputs:\n  account:\n    type: string\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
            ExpectedCompileError::UnsupportedTopLevelDeclaration("inputs"),
        ),
        (
            "top_level_result",
            "version: velvet-ballastics/v1\nname: result\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\nresult:\n  ok: true\n",
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
            b"version: velvet-ballastics/v1\nname: bad\nwhen:\n  manual: {}\nsteps:\n  - id:\n",
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
                ("parallel", "branches", 65_536, 65_535)
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
}

fn primitive_case_strategy() -> impl Strategy<Value = PrimitiveCase> {
    prop::sample::select(PRIMITIVE_CASES.to_vec())
}

fn compile_case(case: &PrimitiveCase) -> Result<CompiledWorkflow, String> {
    let yaml = workflow_yaml(case.yaml_steps);
    compile_yaml(&yaml).map_err(|error| format!("primitive {} failed: {error}", case.name))
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

fn compile_steps_error_with_api(
    steps: &str,
    api_path: PublicApiPath,
) -> Result<CompileErrors, String> {
    let yaml = workflow_yaml(steps);
    compile_yaml_error_with_api(&yaml, api_path)
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
        "parallel" => assert_exact_together(parts.nodes.as_ref()),
        "collect" => assert_exact_collect(parts.nodes.as_ref()),
        "aggregate" => assert_exact_reduce(parts.nodes.as_ref()),
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
    assert_set_const_node(nodes, &[], 1, Some(1), None, 0, 7)?;
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
    assert_set_const_node(nodes, &[], 1, Some(1), None, 1, 1)?;
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
    assert_set_const_node(nodes, &[], 1, Some(1), None, 0, 1)?;
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

fn assert_unsupported_step_primitive(
    case_name: &str,
    api_path: PublicApiPath,
    actual: &CompileError,
    expected_step: usize,
    expected_primitive: &'static str,
) -> Result<(), String> {
    match actual {
        CompileError::UnsupportedStepPrimitive { step, primitive } => {
            assert_eq!(
                (*step, *primitive),
                (expected_step, expected_primitive),
                "api {api_path:?} case {case_name} unsupported primitive payload"
            );
            Ok(())
        }
        other => Err(format!(
            "api {api_path:?} case {case_name} expected UnsupportedStepPrimitive, got {other:?}"
        )),
    }
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
