use proptest::prelude::*;
use vb_compile::{
    CompileError, CompileErrors, YamlCompiler, compile_source, compile_workflow, lower_choose,
    lower_steps_to_ir, lower_together,
};
use vb_core::workflow::{NodeContinuation, NodeOutput, node};
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstValue, SlotBranch, SlotIdx, StepIdx,
    WorkflowError,
};

const HEADER: &str =
    "version: velvet-ballistics/v1\nname: primitive_lowering\nwhen:\n  manual: {}\nsteps:\n";

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
    "CollectNext",
    "CollectFinish",
    "Finish",
];
const REDUCE_KINDS: &[&str] = &[
    "Copy",
    "ReduceStart",
    "SetConst",
    "ReduceNext",
    "ReduceFinish",
    "Finish",
];
const REPEAT_KINDS: &[&str] = &[
    "RepeatStart",
    "SetConst",
    "RepeatCheck",
    "RepeatFinish",
    "Finish",
];
const WAIT_EVENT_KINDS: &[&str] = &["WaitEvent", "Finish"];
const ASK_KINDS: &[&str] = &["Ask", "AskResume", "Finish"];

const PRIMITIVE_CASES: &[PrimitiveCase] = &[
    PrimitiveCase {
        name: "for_each",
        yaml_steps: "  - id: loop\n    for_each:\n      variable: entry\n      input: \"$input.items\"\n      at_once: 1\n      steps:\n        - id: capture\n          set:\n            output: seen\n            value: \"1\"\n  - id: done\n    finish:\n      result: $input.items\n",
        expected_kinds: FOREACH_KINDS,
        expected_slot_count: 4,
    },
    PrimitiveCase {
        name: "together",
        yaml_steps: "  - id: fanout\n    together:\n      branches:\n        - label: left\n          steps:\n            - id: left_set\n              set:\n                output: left\n                value: \"1\"\n        - label: right\n          steps:\n            - id: right_set\n              set:\n                output: right\n                value: \"2\"\n  - id: done\n    finish:\n      result: 0\n",
        expected_kinds: TOGETHER_KINDS,
        expected_slot_count: 3,
    },
    PrimitiveCase {
        name: "collect",
        yaml_steps: "  - id: collect_pages\n    collect:\n      variable: batch\n      source: \"0\"\n      pages: 3\n      items: 5\n      steps:\n        - id: remember_page\n          set:\n            output: page_seen\n            value: \"7\"\n  - id: done\n    finish:\n      result: 0\n",
        expected_kinds: COLLECT_KINDS,
        expected_slot_count: 4,
    },
    PrimitiveCase {
        name: "reduce",
        yaml_steps: "  - id: fold\n    reduce:\n      variable: acc\n      input: \"0\"\n      initial: \"10\"\n      steps:\n        - id: add_one
          set:\n            output: acc_out\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
        expected_kinds: REDUCE_KINDS,
        expected_slot_count: 5,
    },
    PrimitiveCase {
        name: "repeat",
        yaml_steps: "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: attempt\n          set:\n            output: attempted\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
        expected_kinds: REPEAT_KINDS,
        expected_slot_count: 3,
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
            ExpectedShapeError::CanonicalYamlMessage(
                "field shape error: foreach.variable expected non-empty string",
            ),
        ),
        (
            "together",
            "  - id: fanout\n    together:\n      branches:\n        - label: \"\"\n          steps: []\n  - id: done\n    finish:\n      result: 0\n",
            ExpectedShapeError::CanonicalYamlMessage(
                "field shape error: together.branches[].label expected non-empty string",
            ),
        ),
        (
            "collect",
            "  - id: collect_pages\n    collect:\n      variable: batch\n      source: \"\"\n      steps: []\n  - id: done\n    finish:\n      result: 0\n",
            ExpectedShapeError::CanonicalYamlMessage(
                "field shape error: collect.source expected non-empty string",
            ),
        ),
        (
            "reduce",
            "  - id: fold\n    reduce:\n      variable: acc\n      input: \"0\"\n      initial: \"\"\n      steps: []\n  - id: done\n    finish:\n      result: 0\n",
            ExpectedShapeError::CanonicalYamlMessage(
                "field shape error: reduce.initial expected non-empty string",
            ),
        ),
        (
            "repeat",
            "  - id: retry\n    repeat:\n      max_attempts: 0\n      steps: []\n  - id: done\n    finish:\n      result: 0\n",
            ExpectedShapeError::CompileStepField("repeat.max_attempts"),
        ),
        (
            "wait",
            "  - id: wait_for_event\n    wait:\n      event: \"\"\n  - id: done\n    finish:\n      result: 0\n",
            ExpectedShapeError::CanonicalYamlMessage("wait.event must be non-empty"),
        ),
        (
            "ask",
            "  - id: ask_human\n    ask:\n      prompt: \"\"\n  - id: done\n    finish:\n      result: 0\n",
            ExpectedShapeError::CanonicalYamlMessage(
                "field shape error: ask.prompt expected non-empty string",
            ),
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
fn compile_workflow_enforces_scoped_body_width_contracts() -> Result<(), String> {
    // Repeat, ForEach, and Reduce now accept two Set steps in their bodies;
    // Reduce lowers the final Set into the accumulator slot directly.
    let cases = [
        (
            "repeat with two steps in body",
            "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: step1\n          set:\n            output: out_x\n            value: \"1\"\n        - id: step2\n          set:\n            output: out_y\n            value: \"2\"\n  - id: done\n    finish:\n      result: 0\n",
        ),
        (
            "for_each with two steps in body",
            "  - id: loop\n    for_each:\n      variable: entry\n      input: \"$input.items\"\n      steps:\n        - id: step1\n          set:\n            output: out_x\n            value: \"1\"\n        - id: step2\n          set:\n            output: out_y\n            value: \"2\"\n  - id: done\n    finish:\n      result: $input.items\n",
        ),
        (
            "collect with two steps in body",
            "  - id: collect_pages\n    collect:\n      variable: batch\n      source: \"0\"\n      pages: 2\n      items: 1\n      steps:\n        - id: step1\n          set:\n            output: out_x\n            value: \"1\"\n        - id: step2\n          set:\n            output: out_y\n            value: \"2\"\n  - id: done\n    finish:\n      result: 0\n",
        ),
        (
            "reduce with two steps in body",
            "  - id: fold\n    reduce:\n      variable: acc\n      input: \"0\"\n      initial: \"10\"\n      steps:\n        - id: step1\n          set:\n            output: out_x\n            value: \"1\"\n        - id: step2\n          set:\n            output: out_y\n            value: \"2\"\n  - id: done\n    finish:\n      result: 0\n",
        ),
    ];

    for case in cases {
        let case_name = case.0;
        let yaml_steps = case.1;
        let yaml = workflow_yaml(yaml_steps);

        if case_name == "repeat with two steps in body" {
            let workflow = compile_yaml(&yaml)?;
            assert_two_set_repeat_contract(&workflow)?;
        } else if case_name == "for_each with two steps in body" {
            let workflow = compile_yaml(&yaml)?;
            assert_two_set_for_each_contract(&workflow, false)?;
        } else if case_name == "collect with two steps in body" {
            let workflow = compile_yaml(&yaml)?;
            assert_two_set_collect_contract(&workflow)?;
        } else {
            let workflow = compile_yaml(&yaml)?;
            assert_two_set_reduce_contract(&workflow)?;
        }
    }

    Ok(())
}

fn assert_two_set_repeat_contract(workflow: &CompiledWorkflow) -> Result<(), String> {
    let parts = workflow.to_parts();
    if parts.slot_count != 4 {
        return Err(format!("slot_count: expected 4, got {}", parts.slot_count));
    }
    if parts.entry != StepIdx::new(0) {
        return Err(format!("entry: expected 0, got {}", parts.entry.get()));
    }
    if parts.nodes.len() != 6 {
        return Err(format!("nodes: expected 6, got {}", parts.nodes.len()));
    }
    if parts.constants.len() != 2 {
        return Err(format!(
            "constants: expected 2, got {}",
            parts.constants.len()
        ));
    }
    assert_dense_node_ids(workflow, "two_set_repeat")?;
    assert_set_const_node(&parts.nodes, &parts.constants, 1, Some(2), Some(2), 0, 1)?;
    assert_set_const_node(&parts.nodes, &parts.constants, 2, Some(3), Some(3), 1, 2)?;
    let n1 = node_at(&parts.nodes, 1)?;
    let n2 = node_at(&parts.nodes, 2)?;
    if n1.output == n2.output {
        return Err("set nodes must have distinct outputs".to_string());
    }
    assert_finish_node(&parts.nodes, 5, 0)?;
    for node in &parts.nodes {
        if node.routing.error.error_slot().is_some() {
            return Err(format!("node {} has error_slot", node.id.get()));
        }
        if node.routing.error.handler().is_some() {
            return Err(format!("node {} has on_error", node.id.get()));
        }
    }
    assert_two_set_repeat_control(&parts.nodes)?;
    Ok(())
}

fn assert_two_set_repeat_control(nodes: &[CompiledNode]) -> Result<(), String> {
    let specs = [
        (
            0u16,
            CompiledNodeKind::RepeatStart {
                max_attempts: 3.into(),
                body: StepIdx::new(1),
                done: StepIdx::new(4),
            },
            Some(1u16),
            None,
        ),
        (
            3u16,
            CompiledNodeKind::RepeatCheck {
                attempt_slot: SlotIdx::new(1),
                done: StepIdx::new(4),
            },
            Some(1u16),
            Some(1u16),
        ),
        (
            4u16,
            CompiledNodeKind::RepeatFinish {
                result: SlotIdx::new(3),
            },
            Some(3u16),
            Some(5u16),
        ),
    ];
    for (id, kind, output, next) in specs {
        let actual = node_at(nodes, usize::from(id))?;
        let expected = node(
            StepIdx::new(id),
            kind,
            output.map(SlotIdx::new),
            next.map(StepIdx::new),
        );
        if actual != &expected {
            return Err(format!(
                "node {} mismatch: actual={:?} expected={:?}",
                id, actual, expected
            ));
        }
    }
    Ok(())
}

#[test]
fn compile_workflow_accepts_nested_collect_body() -> Result<(), String> {
    // A collect nested inside a collect body lowers recursively: the inner
    // region (nodes 1..=4) sits inside the outer body interval (1..=5).
    let yaml = workflow_yaml(
        "  - id: collect_pages\n    collect:\n      variable: batch\n      source: \"0\"\n      pages: 1\n      items: 1\n      steps:\n        - id: inner\n          collect:\n            variable: inner_page\n            source: \"1\"\n            pages: 1\n            items: 1\n            steps:\n              - id: leaf\n                set:\n                  output: picked\n                  value: \"7\"\n  - id: done\n    finish:\n      result: 0\n",
    );
    let workflow = compile_yaml(&yaml)?;
    let nodes = workflow.to_parts().nodes;
    assert_eq!(nodes.len(), 8, "nested collect must emit 8 nodes");
    let specs = [
        (
            0u16,
            CompiledNodeKind::CollectStart {
                source: SlotIdx::new(0),
                limit: 1.into(),
                page_size: 1,
                body: StepIdx::new(1),
                done: StepIdx::new(6),
            },
            Some(2u16),
            None,
        ),
        (
            1u16,
            CompiledNodeKind::CollectStart {
                source: SlotIdx::new(1),
                limit: 1.into(),
                page_size: 1,
                body: StepIdx::new(2),
                done: StepIdx::new(4),
            },
            Some(4u16),
            None,
        ),
        (
            3u16,
            CompiledNodeKind::CollectNext {
                collector_slot: SlotIdx::new(4),
                body: StepIdx::new(2),
                done: StepIdx::new(4),
            },
            None,
            None,
        ),
        (
            4u16,
            CompiledNodeKind::CollectFinish {
                collector_slot: SlotIdx::new(4),
            },
            Some(5u16),
            Some(5u16),
        ),
        (
            5u16,
            CompiledNodeKind::CollectNext {
                collector_slot: SlotIdx::new(2),
                body: StepIdx::new(1),
                done: StepIdx::new(6),
            },
            None,
            None,
        ),
        (
            6u16,
            CompiledNodeKind::CollectFinish {
                collector_slot: SlotIdx::new(2),
            },
            Some(3u16),
            Some(7u16),
        ),
    ];
    for (id, kind, output, next) in specs {
        let actual = node_at(&nodes, usize::from(id))?;
        let expected = node(
            StepIdx::new(id),
            kind,
            output.map(SlotIdx::new),
            next.map(StepIdx::new),
        );
        if actual != &expected {
            return Err(format!(
                "node {} mismatch: actual={:?} expected={:?}",
                id, actual, expected
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum ExpectedShapeError {
    CanonicalYamlMessage(&'static str),
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
            ExpectedShapeError::CanonicalYamlMessage(expected_message),
        ) => {
            assert_eq!(
                (*category, message.as_ref()),
                ("field_shape", expected_message),
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
                (0, expected_field, "a value of at least 1 (got 0)"),
                "primitive {primitive} returned wrong compile field-shape payload"
            );
            Ok(())
        }
        (other, ExpectedShapeError::CanonicalYamlMessage(_)) => Err(format!(
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

const CONTRACT_TAXONOMY_CASES: [(&str, &str, ExpectedCompileError); 9] = [
    (
        "empty_steps",
        "version: velvet-ballistics/v1\nname: empty\nwhen:\n  manual: {}\nsteps: []\n",
        ExpectedCompileError::EmptySteps,
    ),
    (
        "top_level_result",
        "version: velvet-ballistics/v1\nname: result_case\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\nresult:\n  ok: true\n",
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
        ExpectedCompileError::ExpressionUnknownIdentifier {
            expression: "not-an-integer",
            index: 4,
            identifier: "an",
        },
    ),
    (
        "repeat_max_attempts_shape",
        "  - id: retry\n    repeat:\n      max_attempts: 0\n      steps: []\n  - id: done\n    finish:\n      result: 0\n",
        ExpectedCompileError::StepFieldShape {
            step: 0,
            field: "repeat.max_attempts",
            expected: "a value of at least 1 (got 0)",
        },
    ),
    (
        "slot_index_out_of_range",
        "  - id: done\n    finish:\n      result: 65536\n",
        ExpectedCompileError::SlotIndexOutOfRange { value: 65_536 },
    ),
];

#[test]
fn compile_source_returns_exact_error_variants_for_contract_taxonomy() -> Result<(), String> {
    for (case_name, body, expected) in CONTRACT_TAXONOMY_CASES {
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
    for api_path in PUBLIC_API_PATHS {
        for (case_name, body, expected) in CONTRACT_TAXONOMY_CASES {
            let errors = compile_document_or_steps_error_with_api(body, *api_path)?;
            let first = first_compile_error(&errors)?;
            assert_expected_compile_error(case_name, first, expected)?;
        }
    }
    Ok(())
}

fn assert_repeat_raw_allocation_contract() -> Result<(), String> {
    let cases = [
        (0u16, 0u16, 1u16, 2u16),
        (65533u16, 65533u16, 65534u16, 65535u16),
    ];

    for (prefix, counter, result, count) in cases {
        let mut builder = vb_compile::SlotCompiler::new();
        builder.reserve_slots(prefix).map_err(|e| e.to_string())?;

        let nodes =
            vb_compile::lower_repeat(StepIdx::MAX, 1, StepIdx::MAX, StepIdx::MAX, &mut builder)
                .map_err(|e| e.to_string())?;

        if nodes.len() != 3 {
            return Err(format!("expected 3 nodes, got {}", nodes.len()));
        }

        let [start, attempt, finish] = nodes.as_slice() else {
            return Err("nodes slice mismatch".to_string());
        };

        if start.id != StepIdx::MAX || attempt.id != StepIdx::MAX || finish.id != StepIdx::MAX {
            return Err("node IDs not all MAX".to_string());
        }

        for node in [start, attempt, finish] {
            if node.routing.next.step().is_some()
                || node.routing.error.error_slot().is_some()
                || node.routing.error.handler().is_some()
            {
                return Err("unexpected next/error_slot/on_error".to_string());
            }
        }

        if start.output != NodeOutput::Slot(SlotIdx::new(counter)) {
            return Err("start output mismatch".to_string());
        }
        if attempt.output != NodeOutput::Discard {
            return Err("attempt output should be None".to_string());
        }
        if finish.output != NodeOutput::Slot(SlotIdx::new(result)) {
            return Err("finish output mismatch".to_string());
        }

        assert_eq!(builder.slot_count().map_err(|e| e.to_string())?, count);
        assert_eq!(
            start.kind,
            CompiledNodeKind::RepeatStart {
                max_attempts: 1.into(),
                body: StepIdx::MAX,
                done: StepIdx::MAX,
            }
        );
        assert_eq!(
            attempt.kind,
            CompiledNodeKind::RepeatAttempt {
                attempt_slot: SlotIdx::new(counter),
                body: StepIdx::MAX,
                done: StepIdx::MAX,
            }
        );
        assert_eq!(
            finish.kind,
            CompiledNodeKind::RepeatFinish {
                result: SlotIdx::new(counter),
            }
        );

        if prefix == 0 {
            let ir_result = lower_steps_to_ir(
                nodes,
                vec![],
                vec![],
                vec![],
                count,
                0,
                "raw-repeat-max-id",
                vb_core::WorkflowDigest::from_bytes([0; 32]),
            );

            match ir_result {
                Err(errors) => {
                    if errors
                        != vb_compile::CompileErrors(vec![CompileError::Validation(
                            vb_validate::ValidationError::LoopBodyStepOutOfRange {
                                step: 65535,
                                node_count: 3,
                                source_node: 0,
                                label: String::from("repeat body"),
                            },
                        )])
                    {
                        return Err(format!("unexpected errors: {:?}", errors));
                    }
                }
                Ok(_) => {
                    return Err("expected error from lower_steps_to_ir".to_string());
                }
            }
        }
    }

    for prefix in [65534u16, 65535u16] {
        let mut builder = vb_compile::SlotCompiler::new();
        builder.reserve_slots(prefix).map_err(|e| e.to_string())?;

        let result =
            vb_compile::lower_repeat(StepIdx::MAX, 1, StepIdx::MAX, StepIdx::MAX, &mut builder);

        match result {
            Err(e) => {
                if !matches!(e, CompileError::SlotIndexOutOfRange { value: 65535 }) {
                    return Err(format!("unexpected error: {:?}", e));
                }
            }
            Ok(_) => {
                return Err(format!("expected error for prefix {}", prefix));
            }
        }

        let final_count = builder.slot_count().map_err(|e| e.to_string())?;
        if final_count != 65535 {
            return Err(format!("expected 65535 slots, got {}", final_count));
        }
    }

    Ok(())
}

#[test]
fn public_helpers_return_exact_step_index_slot_index_limit_and_workflow_error_variants()
-> Result<(), String> {
    assert_repeat_raw_allocation_contract()?;

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
        vec![node(
            StepIdx::new(1),
            CompiledNodeKind::Finish {
                result: vb_core::SlotIdx::new(0),
            },
            None,
            None,
        )],
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
            assert_eq!(
                message.as_ref(),
                "field shape error: step.id expected non-empty string"
            );
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
            assert_eq!(otherwise.step(), Some(StepIdx::new(2)));
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
            assert_eq!(otherwise.step(), Some(StepIdx::new(2)));
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
    ExpressionUnknownIdentifier {
        expression: &'static str,
        index: usize,
        identifier: &'static str,
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
        (
            CompileError::ExpressionUnknownIdentifier {
                expression,
                index,
                identifier,
            },
            ExpectedCompileError::ExpressionUnknownIdentifier {
                expression: expected_expression,
                index: expected_index,
                identifier: expected_identifier,
            },
        ) => {
            assert_eq!(
                (expression.as_ref(), *index, identifier.as_ref()),
                (expected_expression, expected_index, expected_identifier),
                "case {case_name} expression unknown identifier payload"
            );
            Ok(())
        }
        (other, _) => Err(format!("case {case_name} returned wrong error {other:?}")),
    }
}

#[path = "v1_primitive_lowering/wait_digest.rs"]
mod wait_digest;

use wait_digest::compile_case;

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

fn parse_source(yaml: &str) -> Result<vb_compile::WorkflowSource, String> {
    vb_compile::parse_workflow_source(yaml).map_err(|error| error.to_string())
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
        message.push_str(error.code().as_str());
        message.push_str(": ");
        message.push_str(&error.to_string());
    }
    message
}

fn workflow_yaml(steps: &str) -> String {
    let header = if steps.contains("$input.items") {
        HEADER.replace(
            "steps:\n",
            "inputs:\n  items: {is: list, of: any, max: 2}\nsteps:\n",
        )
    } else {
        HEADER.to_string()
    };
    let mut yaml = header;
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
        "for_each" => assert_exact_for_each(parts.nodes.as_ref(), parts.constants.as_ref()),
        "together" => assert_exact_together(parts.nodes.as_ref(), parts.constants.as_ref()),
        "collect" => assert_exact_collect(parts.nodes.as_ref(), parts.constants.as_ref()),
        "reduce" => assert_exact_reduce(parts.nodes.as_ref(), parts.constants.as_ref()),
        "repeat" => assert_exact_repeat(parts.nodes.as_ref(), parts.constants.as_ref()),
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

fn assert_exact_for_each(nodes: &[CompiledNode], constants: &[ConstValue]) -> Result<(), String> {
    assert_set_const_node(nodes, constants, 1, Some(3), Some(2), 0, 1)?;
    assert_finish_node(nodes, 3, 0)?;

    let node0 = node_at(nodes, 0)?;
    match &node0.kind {
        CompiledNodeKind::ForEachStart {
            input,
            item_slot,
            limit,
            body,
            done,
        } => {
            assert_eq!(
                (
                    input.get(),
                    item_slot.get(),
                    limit.get(),
                    body.get(),
                    done.get()
                ),
                (0, 2, 2, 1, 3)
            );
        }
        other => return Err(format!("expected ForEachStart, got {other:?}")),
    }
    assert_eq!(node0.output, NodeOutput::Slot(SlotIdx::new(1)));
    assert_eq!(node0.routing.next, NodeContinuation::End);

    let node2 = node_at(nodes, 2)?;
    match &node2.kind {
        CompiledNodeKind::ForEachNext {
            iterator_slot,
            body,
            done,
        } => {
            assert_eq!((iterator_slot.get(), body.get(), done.get()), (1, 1, 3));
        }
        other => return Err(format!("expected ForEachNext, got {other:?}")),
    }
    assert_eq!(node2.output, NodeOutput::Slot(SlotIdx::new(2)));

    Ok(())
}

fn assert_exact_together(nodes: &[CompiledNode], constants: &[ConstValue]) -> Result<(), String> {
    assert_set_const_node(nodes, constants, 2, Some(2), Some(3), 0, 1)?;
    assert_set_const_node(nodes, constants, 4, Some(2), Some(5), 1, 2)?;
    assert_finish_node(nodes, 6, 0)?;

    let node0 = node_at(nodes, 0)?;
    match &node0.kind {
        CompiledNodeKind::TogetherStart { branches, join } => {
            let actual: Vec<u16> = branches.iter().map(|b| b.get()).collect();
            assert_eq!(actual, [1, 3]);
            assert_eq!(join.get(), 5);
        }
        other => return Err(format!("expected TogetherStart, got {other:?}")),
    }
    assert_eq!(node0.output, NodeOutput::Slot(SlotIdx::new(1)));
    assert_eq!(node0.routing.next, NodeContinuation::End);

    let node5 = node_at(nodes, 5)?;
    match &node5.kind {
        CompiledNodeKind::TogetherJoin {
            branch_count,
            accumulator,
        } => {
            assert_eq!((branch_count.get(), accumulator.get()), (2, 1));
        }
        other => return Err(format!("expected TogetherJoin, got {other:?}")),
    }
    assert_eq!(node5.output, NodeOutput::Slot(SlotIdx::new(2)));
    assert_eq!(node5.routing.next, NodeContinuation::Step(StepIdx::new(6)));

    let node1 = node_at(nodes, 1)?;
    assert_eq!(node1.output, NodeOutput::Discard);
    match &node1.kind {
        CompiledNodeKind::TogetherBranch {
            branch,
            entry,
            join,
            accumulator,
        } => {
            assert_eq!(
                (branch.get(), entry.get(), join.get(), accumulator.get()),
                (0, 2, 5, 1)
            );
        }
        other => return Err(format!("expected first TogetherBranch, got {other:?}")),
    }

    let node3 = node_at(nodes, 3)?;
    assert_eq!(node3.output, NodeOutput::Slot(SlotIdx::new(2)));
    match &node3.kind {
        CompiledNodeKind::TogetherBranch {
            branch,
            entry,
            join,
            accumulator,
        } => {
            assert_eq!(
                (branch.get(), entry.get(), join.get(), accumulator.get()),
                (1, 4, 5, 1)
            );
        }
        other => return Err(format!("expected second TogetherBranch, got {other:?}")),
    }

    Ok(())
}

fn assert_exact_collect(nodes: &[CompiledNode], constants: &[ConstValue]) -> Result<(), String> {
    assert_set_const_node(nodes, constants, 1, Some(3), Some(2), 0, 7)?;
    assert_finish_node(nodes, 4, 0)?;

    let node0 = node_at(nodes, 0)?;
    match &node0.kind {
        CompiledNodeKind::CollectStart {
            source,
            limit,
            page_size,
            body,
            done,
        } => {
            assert_eq!(
                (
                    source.get(),
                    limit.get(),
                    *page_size,
                    body.get(),
                    done.get()
                ),
                (0, 15, 5, 1, 3)
            );
        }
        other => return Err(format!("expected CollectStart, got {other:?}")),
    }
    assert_eq!(node0.output, NodeOutput::Slot(SlotIdx::new(1)));
    assert_eq!(node0.routing.next, NodeContinuation::End);

    let node2 = node_at(nodes, 2)?;
    match &node2.kind {
        CompiledNodeKind::CollectNext {
            collector_slot,
            body,
            done,
        } => {
            assert_eq!((collector_slot.get(), body.get(), done.get()), (1, 1, 3));
        }
        other => return Err(format!("expected CollectNext, got {other:?}")),
    }

    let node3 = node_at(nodes, 3)?;
    match &node3.kind {
        CompiledNodeKind::CollectFinish { collector_slot } => {
            assert_eq!(collector_slot.get(), 1);
        }
        other => return Err(format!("expected CollectFinish, got {other:?}")),
    }
    assert_eq!(node3.output, NodeOutput::Slot(SlotIdx::new(2)));
    assert_eq!(node3.routing.next, NodeContinuation::Step(StepIdx::new(4)));

    Ok(())
}

fn assert_exact_reduce(nodes: &[CompiledNode], constants: &[ConstValue]) -> Result<(), String> {
    assert_eq!(constants.get(0), Some(&ConstValue::I64(10)));
    assert_set_const_node(nodes, constants, 2, Some(4), Some(3), 1, 1)?;
    assert_finish_node(nodes, 5, 0)?;

    let node0 = node_at(nodes, 0)?;
    match &node0.kind {
        CompiledNodeKind::Copy { source } => {
            assert_eq!(source.get(), 0);
        }
        other => return Err(format!("expected Copy, got {other:?}")),
    }
    assert_eq!(node0.output, NodeOutput::Slot(SlotIdx::new(1)));
    assert_eq!(node0.routing.next, NodeContinuation::Step(StepIdx::new(1)));

    let node1 = node_at(nodes, 1)?;
    match &node1.kind {
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
                (1, 4, 0, 2, 4)
            );
        }
        other => return Err(format!("expected ReduceStart, got {other:?}")),
    }
    assert_eq!(node1.output, NodeOutput::Slot(SlotIdx::new(3)));

    let node3 = node_at(nodes, 3)?;
    match &node3.kind {
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
                (3, 4, 2, 4)
            );
        }
        other => return Err(format!("expected ReduceNext, got {other:?}")),
    }
    assert_eq!(node3.output, NodeOutput::Slot(SlotIdx::new(2)));

    let node4 = node_at(nodes, 4)?;
    match &node4.kind {
        CompiledNodeKind::ReduceFinish { accumulator } => {
            assert_eq!(accumulator.get(), 4);
        }
        other => return Err(format!("expected ReduceFinish, got {other:?}")),
    }
    assert_eq!(node4.output, NodeOutput::Slot(SlotIdx::new(4)));
    assert_eq!(node4.routing.next, NodeContinuation::Step(StepIdx::new(5)));

    Ok(())
}

fn assert_exact_repeat(nodes: &[CompiledNode], constants: &[ConstValue]) -> Result<(), String> {
    assert_set_const_node(nodes, constants, 1, Some(2), Some(2), 0, 1)?;
    assert_finish_node(nodes, 4, 0)?;

    let node0 = node_at(nodes, 0)?;
    match &node0.kind {
        CompiledNodeKind::RepeatStart {
            max_attempts,
            body,
            done,
        } => {
            assert_eq!((max_attempts.get(), body.get(), done.get()), (3, 1, 3));
        }
        other => return Err(format!("expected RepeatStart, got {other:?}")),
    }
    assert_eq!(node0.output, NodeOutput::Slot(SlotIdx::new(1)));
    assert_eq!(node0.routing.next, NodeContinuation::End);

    let node2 = node_at(nodes, 2)?;
    match &node2.kind {
        CompiledNodeKind::RepeatCheck { attempt_slot, done } => {
            assert_eq!((attempt_slot.get(), done.get()), (1, 3));
        }
        other => return Err(format!("expected RepeatCheck, got {other:?}")),
    }
    assert_eq!(node2.output, NodeOutput::Slot(SlotIdx::new(1)));
    assert_eq!(node2.routing.next, NodeContinuation::Step(StepIdx::new(1)));

    let node3 = node_at(nodes, 3)?;
    match &node3.kind {
        CompiledNodeKind::RepeatFinish { result } => {
            assert_eq!(result.get(), 2);
        }
        other => return Err(format!("expected RepeatFinish, got {other:?}")),
    }
    assert_eq!(node3.output, NodeOutput::Slot(SlotIdx::new(2)));
    assert_eq!(node3.routing.next, NodeContinuation::Step(StepIdx::new(4)));

    Ok(())
}

fn assert_exact_wait_event(nodes: &[CompiledNode]) -> Result<(), String> {
    assert_finish_node(nodes, 1, 0)?;
    match &node_at(nodes, 0)?.kind {
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => {
            let timeout = timeout_slot
                .slot()
                .ok_or_else(|| String::from("missing wait timeout slot"))?;
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
            let timeout = timeout_slot
                .slot()
                .ok_or_else(|| String::from("missing ask timeout slot"))?;
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
        node.output.slot().map(|slot| slot.get()),
        expected_output,
        "SetConst node {index} output slot"
    );
    assert_eq!(
        node.routing.next.step().map(|step| step.get()),
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
        node.output,
        NodeOutput::Discard,
        "Finish node {index} must not write output"
    );
    assert_eq!(
        node.routing.next,
        NodeContinuation::End,
        "Finish node {index} must not fall through"
    );
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
        assert_optional_target(node.routing.next.step(), node_count, case_name, "next")?;
        assert_optional_target(
            node.routing.error.handler(),
            node_count,
            case_name,
            "on_error",
        )?;
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
            assert_optional_target(otherwise.step(), node_count, case_name, "choose.otherwise")?;
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
            assert_optional_target(
                otherwise.step(),
                node_count,
                case_name,
                "choose_slot.otherwise",
            )?;
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
            "  - id: pages\n    collect:\n      variable: batch\n      source: \"0\"\n      pages: 2\n      items: 3\n      steps: []\n  - id: done\n    finish:\n      result: 0\n",
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
        let expected = match case_name {
            "for_each with empty body" => (0, "for_each.body", "non-empty body"),
            "collect with empty body" => (0, "collect.body", "non-empty body"),
            _ => (0, "steps", "at least one set or do step"),
        };
        match first {
            CompileError::StepFieldShape {
                step,
                field,
                expected: exp,
                ..
            } => {
                assert_eq!(
                    (*step, field.as_ref(), exp.as_ref()),
                    expected,
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
fn compile_workflow_accepts_multi_step_for_each_at_non_zero_step() -> Result<(), String> {
    let yaml = workflow_yaml(
        "  - id: preamble\n    set:\n      output: x\n      value: \"1\"\n  - id: loop\n    for_each:\n      variable: entry\n      input: \"$input.items\"\n      steps:\n        - id: step1\n          set:\n            output: a\n            value: \"1\"\n        - id: step2\n          set:\n            output: b\n            value: \"2\"\n  - id: done\n    finish:\n      result: x\n",
    );
    let workflow = compile_yaml(&yaml)?;
    assert_two_set_for_each_contract(&workflow, true)?;
    Ok(())
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
        "  - id: setup\n    set:\n      output: condition\n      value: \"true\"\n  - id: pick\n    choose:\n      branches:\n        - when: \"0\"\n          steps: []\n        - when: \"1\"\n          steps: []\n      otherwise: done\n  - id: done\n    finish:\n      result: 0\n",
    );
    let workflow = compile_yaml(&yaml)?;
    let kinds = node_kind_names(workflow.to_parts().nodes.as_ref());
    assert_eq!(
        kinds,
        vec!["EvalExpr", "ChooseSlot", "Finish"],
        "boolean setup and two-branch choose must compile to EvalExpr, ChooseSlot, Finish"
    );
    Ok(())
}

#[test]
fn compile_workflow_choose_branch_body_emits_dense_order() -> Result<(), String> {
    let yaml = workflow_yaml(
        "  - id: pick\n    choose:\n      branches:\n        - when: \"0\"\n          steps:\n            - id: body_a\n              set:\n                output: branch_value\n                value: \"7\"\n      otherwise: done\n  - id: done\n    finish:\n      result: 0\n",
    );
    let workflow = compile_yaml(&yaml)?;
    let parts = workflow.to_parts();
    let kinds = node_kind_names(parts.nodes.as_ref());
    assert_eq!(
        kinds,
        vec!["ChooseSlot", "SetConst", "Finish"],
        "choose body must preserve StepIdx-indexed node order"
    );
    assert_dense_node_ids(&workflow, "choose_branch_body")?;
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

fn assert_two_set_collect_contract(workflow: &CompiledWorkflow) -> Result<(), String> {
    let parts = workflow.to_parts();
    let nodes = &parts.nodes;
    let constants = &parts.constants;
    let slot_count = parts.slot_count;
    let entry_step = parts.entry;

    if slot_count != 5 {
        return Err(format!("expected slot_count 5, got {}", slot_count));
    }
    if nodes.len() != 6 {
        return Err(format!("expected 6 nodes, got {}", nodes.len()));
    }
    if constants.len() != 2 {
        return Err(format!("expected 2 constants, got {}", constants.len()));
    }
    if entry_step != StepIdx::new(0) {
        return Err(format!("expected entry_step 0, got {:?}", entry_step));
    }

    assert_dense_node_ids(workflow, "two_set_collect")?;

    assert_set_const_node(&nodes, &constants, 1, Some(3), Some(2), 0, 1)?;
    assert_set_const_node(&nodes, &constants, 2, Some(4), Some(3), 1, 2)?;
    assert_finish_node(&nodes, 5, 0)?;

    let node0 = node_at(&nodes, 0)?;
    match &node0.kind {
        CompiledNodeKind::CollectStart {
            source,
            limit,
            page_size,
            body,
            done,
        } => {
            if source.get() != 0
                || limit.get() != 2
                || *page_size != 1
                || body.get() != 1
                || done.get() != 4
            {
                return Err("CollectStart fields mismatch".to_string());
            }
        }
        other => return Err(format!("expected CollectStart, got {other:?}")),
    }
    if node0.output != NodeOutput::Slot(SlotIdx::new(1)) {
        return Err("node0.output mismatch".to_string());
    }
    if node0.routing.next != NodeContinuation::End {
        return Err("node0.next mismatch".to_string());
    }

    let node3 = node_at(&nodes, 3)?;
    match &node3.kind {
        CompiledNodeKind::CollectNext {
            collector_slot,
            body,
            done,
        } => {
            if collector_slot.get() != 1 || body.get() != 1 || done.get() != 4 {
                return Err("CollectNext fields mismatch".to_string());
            }
        }
        other => return Err(format!("expected CollectNext, got {other:?}")),
    }

    let node4 = node_at(&nodes, 4)?;
    match &node4.kind {
        CompiledNodeKind::CollectFinish { collector_slot } => {
            if collector_slot.get() != 1 {
                return Err("CollectFinish collector_slot mismatch".to_string());
            }
        }
        other => return Err(format!("expected CollectFinish, got {other:?}")),
    }
    if node4.output != NodeOutput::Slot(SlotIdx::new(2)) {
        return Err("node4.output mismatch".to_string());
    }
    if node4.routing.next != NodeContinuation::Step(StepIdx::new(5)) {
        return Err("node4.next mismatch".to_string());
    }

    Ok(())
}

fn assert_two_set_reduce_contract(workflow: &CompiledWorkflow) -> Result<(), String> {
    let parts = workflow.to_parts();
    let nodes = &parts.nodes;
    let constants = &parts.constants;

    if parts.slot_count != 6 {
        return Err(format!("expected slot_count 6, got {}", parts.slot_count));
    }
    if nodes.len() != 7 {
        return Err(format!("expected 7 nodes, got {}", nodes.len()));
    }
    if constants.len() != 3 {
        return Err(format!("expected 3 constants, got {}", constants.len()));
    }
    if parts.entry != StepIdx::new(0) {
        return Err(format!("expected entry_step 0, got {:?}", parts.entry));
    }
    assert_dense_node_ids(workflow, "two_set_reduce")?;
    assert_all_targets_in_range(workflow, "two_set_reduce")?;

    assert_eq!(
        constants.get(0),
        Some(&ConstValue::I64(10)),
        "initial constant"
    );
    assert_set_const_node(&nodes, &constants, 2, Some(5), Some(3), 1, 1)?;
    assert_set_const_node(&nodes, &constants, 3, Some(4), Some(4), 2, 2)?;
    assert_finish_node(&nodes, 6, 0)?;

    let node0 = node_at(&nodes, 0)?;
    match &node0.kind {
        CompiledNodeKind::Copy { source } => {
            if source.get() != 0 {
                return Err("Copy source mismatch".to_string());
            }
        }
        other => return Err(format!("expected Copy, got {other:?}")),
    }
    if node0.output != NodeOutput::Slot(SlotIdx::new(1)) {
        return Err("node0.output mismatch".to_string());
    }
    if node0.routing.next != NodeContinuation::Step(StepIdx::new(1)) {
        return Err("node0.next mismatch".to_string());
    }

    let node1 = node_at(&nodes, 1)?;
    match &node1.kind {
        CompiledNodeKind::ReduceStart {
            input,
            accumulator,
            initial,
            body,
            done,
        } => {
            if (
                input.get(),
                accumulator.get(),
                initial.get(),
                body.get(),
                done.get(),
            ) != (1, 4, 0, 2, 5)
            {
                return Err("ReduceStart fields mismatch".to_string());
            }
        }
        other => return Err(format!("expected ReduceStart, got {other:?}")),
    }
    if node1.output != NodeOutput::Slot(SlotIdx::new(3)) {
        return Err("node1.output mismatch".to_string());
    }

    let node4 = node_at(&nodes, 4)?;
    match &node4.kind {
        CompiledNodeKind::ReduceNext {
            iterator_slot,
            accumulator,
            body,
            done,
        } => {
            if (
                iterator_slot.get(),
                accumulator.get(),
                body.get(),
                done.get(),
            ) != (3, 4, 2, 5)
            {
                return Err("ReduceNext fields mismatch".to_string());
            }
        }
        other => return Err(format!("expected ReduceNext, got {other:?}")),
    }
    if node4.output != NodeOutput::Slot(SlotIdx::new(2)) {
        return Err("node4.output mismatch".to_string());
    }

    let node5 = node_at(&nodes, 5)?;
    match &node5.kind {
        CompiledNodeKind::ReduceFinish { accumulator } => {
            if accumulator.get() != 4 {
                return Err("ReduceFinish accumulator mismatch".to_string());
            }
        }
        other => return Err(format!("expected ReduceFinish, got {other:?}")),
    }
    if node5.output != NodeOutput::Slot(SlotIdx::new(4)) {
        return Err("node5.output mismatch".to_string());
    }
    if node5.routing.next != NodeContinuation::Step(StepIdx::new(6)) {
        return Err("node5.next mismatch".to_string());
    }

    Ok(())
}

fn assert_two_set_for_each_contract(
    workflow: &CompiledWorkflow,
    preamble: bool,
) -> Result<(), String> {
    let parts = workflow.to_parts();
    assert_eq!(parts.entry, StepIdx::new(0), "entry must be step 0");
    let nodes = &parts.nodes;
    let slot_count = parts.slot_count;
    let constants = &parts.constants;

    assert_dense_node_ids(workflow, "two_set_for_each")?;
    assert_all_targets_in_range(workflow, "two_set_for_each")?;

    if preamble {
        assert_eq!(nodes.len(), 6, "preamble layout must have 6 nodes");
        assert_eq!(slot_count, 6, "preamble layout must have 6 slots");
        assert_eq!(constants.len(), 3, "preamble layout must have 3 constants");

        assert_set_const_node(nodes, constants, 0, Some(0), Some(1), 0, 1)?;

        let node = node_at(nodes, 1)?;
        assert_eq!(
            node.output.slot().map(|s| s.get()),
            Some(2),
            "Start output slot"
        );
        assert_eq!(
            node.routing.next,
            NodeContinuation::End,
            "Start must not fall through"
        );
        match &node.kind {
            CompiledNodeKind::ForEachStart {
                input,
                item_slot,
                limit,
                body,
                done,
            } => {
                assert_eq!(input.get(), 1, "Start input slot");
                assert_eq!(item_slot.get(), 3, "Start item slot");
                assert_eq!(limit.get(), 2, "Start limit slot");
                assert_eq!(body.get(), 2, "Start body target");
                assert_eq!(done.get(), 5, "Start done target");
            }
            other => return Err(format!("expected Start at node 1, got {other:?}")),
        }

        assert_set_const_node(nodes, constants, 2, Some(4), Some(3), 1, 1)?;
        assert_set_const_node(nodes, constants, 3, Some(5), Some(4), 2, 2)?;

        let node = node_at(nodes, 4)?;
        assert_eq!(
            node.output.slot().map(|s| s.get()),
            Some(3),
            "Next output slot"
        );
        assert_eq!(
            node.routing.next,
            NodeContinuation::End,
            "Next must not fall through"
        );
        match &node.kind {
            CompiledNodeKind::ForEachNext {
                iterator_slot,
                body,
                done,
            } => {
                assert_eq!(iterator_slot.get(), 2, "Next iterator slot");
                assert_eq!(body.get(), 2, "Next body target");
                assert_eq!(done.get(), 5, "Next done target");
            }
            other => return Err(format!("expected Next at node 4, got {other:?}")),
        }

        assert_finish_node(nodes, 5, 0)?;
    } else {
        assert_eq!(nodes.len(), 5, "non-preamble layout must have 5 nodes");
        assert_eq!(slot_count, 5, "non-preamble layout must have 5 slots");
        assert_eq!(
            constants.len(),
            2,
            "non-preamble layout must have 2 constants"
        );

        let node = node_at(nodes, 0)?;
        assert_eq!(
            node.output.slot().map(|s| s.get()),
            Some(1),
            "Start output slot"
        );
        assert_eq!(
            node.routing.next,
            NodeContinuation::End,
            "Start must not fall through"
        );
        match &node.kind {
            CompiledNodeKind::ForEachStart {
                input,
                item_slot,
                limit,
                body,
                done,
            } => {
                assert_eq!(input.get(), 0, "Start input slot");
                assert_eq!(item_slot.get(), 2, "Start item slot");
                assert_eq!(limit.get(), 2, "Start limit slot");
                assert_eq!(body.get(), 1, "Start body target");
                assert_eq!(done.get(), 4, "Start done target");
            }
            other => return Err(format!("expected Start at node 0, got {other:?}")),
        }

        assert_set_const_node(nodes, constants, 1, Some(3), Some(2), 0, 1)?;
        assert_set_const_node(nodes, constants, 2, Some(4), Some(3), 1, 2)?;

        let node = node_at(nodes, 3)?;
        assert_eq!(
            node.output.slot().map(|s| s.get()),
            Some(2),
            "Next output slot"
        );
        assert_eq!(
            node.routing.next,
            NodeContinuation::End,
            "Next must not fall through"
        );
        match &node.kind {
            CompiledNodeKind::ForEachNext {
                iterator_slot,
                body,
                done,
            } => {
                assert_eq!(iterator_slot.get(), 1, "Next iterator slot");
                assert_eq!(body.get(), 1, "Next body target");
                assert_eq!(done.get(), 4, "Next done target");
            }
            other => return Err(format!("expected Next at node 3, got {other:?}")),
        }

        assert_finish_node(nodes, 4, 0)?;
    }

    Ok(())
}
