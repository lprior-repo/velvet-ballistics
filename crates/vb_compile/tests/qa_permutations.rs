//! vb-9y985 permutation tests for compiler semantics fixes.

use vb_compile::{CompileError, compile_workflow};

fn workflow(body: &str) -> String {
    format!("version: velvet-ballistics/v1\nname: qa_wf\nwhen:\n  manual: {{}}\n{body}")
}

fn compile(body: &str) -> Result<vb_core::CompiledWorkflow, String> {
    let yaml = workflow(body);
    compile_workflow(yaml.as_bytes()).map_err(|e| e.to_string())
}

#[test]
fn qa_nested_step_ids_are_lexically_scoped() {
    // Branch bodies may reuse outer step IDs (scoped shadowing); only
    // same-list duplicates are errors and those predate this change.
    let body = "steps:\n  - id: a\n    set: {output: x, value: '1'}\n  - id: r\n    choose:\n      branches: [{when: 'true', steps: [{id: a, set: {output: y1, value: '2'}}]}]\n      otherwise: done\n  - id: done\n    finish: {result: x}\n";
    let result = compile(body);
    assert_eq!(result.is_ok(), true, "scoped shadow rejected: {result:?}");
    let dup_top =
        "steps:\n  - id: a\n    set: {output: x, value: '1'}\n  - id: a\n    finish: {result: x}\n";
    let err = compile(dup_top).expect_err("top-level duplicate must fail");
    assert!(err.contains("duplicate step id: a"), "got {err}");
}

#[test]
fn qa_set_output_shadowing_input_rejected() {
    let body = "inputs:\n  needed: {is: text}\nsteps:\n  - id: first\n    set: {output: needed, value: '1'}\n  - id: done\n    finish: {result: needed}\n";
    let err = compile(body).expect_err("shadow must fail");
    assert!(err.contains("shadows declared input"), "got {err}");
}

#[test]
fn qa_choose_empty_branches_rules() {
    let no_otherwise =
        "steps:\n  - id: pick\n    choose: {branches: []}\n  - id: done\n    finish: {result: 0}\n";
    assert_eq!(compile(no_otherwise).is_err(), true);
    let with_otherwise = "steps:\n  - id: pick\n    choose: {branches: [], otherwise: done}\n  - id: done\n    finish: {result: 0}\n";
    let result = compile(with_otherwise);
    assert_eq!(
        result.is_ok(),
        true,
        "else-only choose rejected: {result:?}"
    );
}

#[test]
fn qa_constant_faults_rejected_at_compile() {
    for value in [
        "1 / 0",
        "9223372036854775807 + 1",
        "9223372036854775807 * 2",
    ] {
        let body = format!(
            "steps:\n  - id: first\n    set: {{output: x, value: '{value}'}}\n  - id: done\n    finish: {{result: x}}\n"
        );
        let err = compile(&body).expect_err("constant fault must fail");
        assert!(
            err.contains("constant expression"),
            "value {value} got {err}"
        );
    }
    let ok = "steps:\n  - id: first\n    set: {output: x, value: '1 + 2'}\n  - id: done\n    finish: {result: x}\n";
    assert_eq!(compile(ok).is_ok(), true);
}

#[test]
fn qa_integer_overflow_scalar_not_float_error() {
    let body = "steps:\n  - id: done\n    finish: {result: 9223372036854775808}\n";
    let yaml = workflow(body);
    let errors = compile_workflow(yaml.as_bytes()).expect_err("overflow must fail");
    let text = errors.to_string();
    assert!(
        errors
            .0
            .iter()
            .any(|e| matches!(e, CompileError::IntegerScalarOutOfRange { .. }))
            || text.contains("i64 range"),
        "got {errors:?}"
    );
    assert!(
        !text.contains("floating-point"),
        "float misdiagnosis: {text}"
    );
}

#[test]
fn qa_repeat_zero_attempts_value_message() {
    let body = "steps:\n  - id: retry\n    repeat:\n      max_attempts: 0\n      steps: [{id: inner, set: {output: x, value: '1'}}]\n  - id: done\n    finish: {result: x}\n";
    let err = compile(body).expect_err("zero attempts must fail");
    assert!(err.contains("at least 1 (got 0)"), "got {err}");
}

#[test]
fn qa_vars_non_scalar_names_field() -> Result<(), Box<dyn std::error::Error>> {
    use vb_core::{CompiledNodeKind as K, ConstIdx, ConstValue, SlotIdx};

    let body =
        "vars:\n  items: [1, 2]\nsteps:\n  - id: done\n    finish: {result: '$vars.items'}\n";
    let parts = compile(body)?.to_parts();
    assert_eq!(
        parts.constants.as_ref(),
        [ConstValue::I64(1), ConstValue::I64(2)]
    );
    assert_eq!(
        parts
            .nodes
            .iter()
            .map(|node| node.kind.clone())
            .collect::<Vec<_>>(),
        [
            K::SetConst {
                value: ConstIdx::new(0)
            },
            K::SetConst {
                value: ConstIdx::new(1)
            },
            K::BuildList {
                items: Box::from([SlotIdx::ZERO, SlotIdx::new(1)])
            },
            K::Finish {
                result: SlotIdx::new(2)
            },
        ]
    );
    for vars in ["vars: [1, 2]", "vars: 7"] {
        let yaml = workflow(&format!(
            "{vars}\nsteps:\n  - id: done\n    finish: {{result: 0}}\n"
        ));
        let errors = compile_workflow(yaml.as_bytes())
            .err()
            .ok_or("non-mapping vars accepted")?;
        assert_eq!(
            errors.0,
            [CompileError::CanonicalYaml {
                category: "field_shape",
                message: "field shape error: vars expected mapping".into(),
            }]
        );
    }
    Ok(())
}

#[test]
fn qa_core_tags_consistent_across_apis() {
    let yaml = "version: !!str velvet-ballistics/v1\nname: qa_wf\nwhen:\n  manual: {}\nsteps:\n  - id: first\n    set: {output: x, value: '1'}\n  - id: done\n    finish: {result: x}\n";
    let compiled = compile_workflow(yaml.as_bytes());
    assert_eq!(compiled.is_ok(), true, "compile_workflow: {compiled:?}");
    let compiler = vb_compile::YamlCompiler::default();
    let parsed = compiler.parse_ast(yaml.as_bytes());
    if let Err(errors) = &parsed {
        assert!(
            !errors
                .0
                .iter()
                .any(|e| matches!(e, CompileError::TagForbidden { .. })),
            "parse_ast rejected core tag: {errors:?}"
        );
    }
    let custom = yaml.replace("!!str", "!mytag");
    assert_eq!(compile_workflow(custom.as_bytes()).is_err(), true);
    assert_eq!(compiler.parse_ast(custom.as_bytes()).is_err(), true);
}
