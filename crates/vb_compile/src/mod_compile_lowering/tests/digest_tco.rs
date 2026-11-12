//! Deep-nesting TCO regression: iterative digest walks preserve recursive bytes.
//!
//! 64-deep digests equal the recursive baseline; 65-deep fails closed with
//! `PrimitiveLoweringLimitExceeded` on a bounded stack. Baselines below are
//! test-local recursive references mirroring the pre-TCO byte order
//! (pre-order + `TAG_END`), independent of the production heap-stack engine.

use crate::mod_compile_lowering::part_05::{digest_author_value, digest_step_primitive};
use crate::{AuthorEntry, AuthorValue, CompileError, StepAst, StepPrimitive};
use blake3::Hasher;

const TAG_STRING: u8 = 0x01;
const TAG_I64: u8 = 0x02;
const TAG_U16: u8 = 0x03;
const TAG_U32: u8 = 0x04;
const TAG_SOME: u8 = 0x10;
const TAG_NONE: u8 = 0x11;
const TAG_END: u8 = 0x22;

fn base_string(hasher: &mut Hasher, value: &str) {
    hasher.update(&[TAG_STRING]);
    let len = u32::try_from(value.len()).expect("test string fits u32");
    hasher.update(&len.to_le_bytes());
    hasher.update(value.as_bytes());
}

fn base_len(hasher: &mut Hasher, value: usize) {
    let len = u32::try_from(value).expect("test len fits u32");
    hasher.update(&len.to_le_bytes());
}

fn base_i64(hasher: &mut Hasher, value: i64) {
    hasher.update(&[TAG_I64]);
    hasher.update(&value.to_le_bytes());
}

fn base_u16(hasher: &mut Hasher, value: u16) {
    hasher.update(&[TAG_U16]);
    hasher.update(&value.to_le_bytes());
}

fn base_u32(hasher: &mut Hasher, value: u32) {
    hasher.update(&[TAG_U32]);
    hasher.update(&value.to_le_bytes());
}

fn base_optional_string(hasher: &mut Hasher, tag: &[u8], value: Option<&str>) {
    hasher.update(tag);
    match value {
        Some(s) => {
            hasher.update(&[TAG_SOME]);
            base_string(hasher, s);
        }
        None => {
            hasher.update(&[TAG_NONE]);
        }
    }
}

fn base_optional_u32(hasher: &mut Hasher, tag: &[u8], value: Option<u32>) {
    hasher.update(tag);
    base_u32(hasher, value.unwrap_or(1));
}

fn base_scalar(hasher: &mut Hasher, value: &crate::ScalarValue) {
    match value {
        crate::ScalarValue::String(s) => {
            hasher.update(&[0x01]);
            base_string(hasher, s);
        }
        crate::ScalarValue::Integer(n) => {
            hasher.update(&[0x02]);
            base_i64(hasher, *n);
        }
        _ => panic!("test baseline never sees unknown scalar variant"),
    }
}

fn base_author_value(hasher: &mut Hasher, value: &AuthorValue) {
    match value {
        AuthorValue::Null => {
            hasher.update(&[0x00]);
        }
        AuthorValue::Bool(b) => {
            hasher.update(&[0x01]);
            hasher.update(&[if *b { 1 } else { 0 }]);
        }
        AuthorValue::I64(n) => {
            hasher.update(&[0x02]);
            base_i64(hasher, *n);
        }
        AuthorValue::Text(s) => {
            hasher.update(&[0x03]);
            base_string(hasher, s);
        }
        AuthorValue::Sequence(seq) => {
            hasher.update(&[0x04]);
            base_len(hasher, seq.len());
            for item in seq {
                base_author_value(hasher, item);
            }
            hasher.update(&[TAG_END]);
        }
        AuthorValue::Mapping(entries) => {
            hasher.update(&[0x05]);
            let mut sorted: Vec<&AuthorEntry<AuthorValue>> = entries.iter().collect();
            sorted.sort_by(|a, b| a.key.cmp(&b.key));
            base_len(hasher, sorted.len());
            for entry in sorted {
                base_string(hasher, &entry.key);
                base_author_value(hasher, &entry.value);
            }
            hasher.update(&[TAG_END]);
        }
        _ => panic!("test baseline never sees unknown author variant"),
    }
}

fn base_step(hasher: &mut Hasher, step: &StepAst) {
    base_string(hasher, &step.id);
    base_optional_string(hasher, b"step_name:", step.name.as_deref());
    base_optional_string(hasher, b"step_condition:", step.condition.as_deref());
    hasher.update(b"step_primitive:");
    base_primitive(hasher, &step.primitive);
    hasher.update(b"step_with:");
    match &step.with {
        Some(value) => {
            hasher.update(&[1]);
            base_author_value(hasher, value);
        }
        None => {
            hasher.update(&[0]);
        }
    }
    hasher.update(b"step_retry:");
    if let Some(retry) = &step.retry {
        hasher.update(&[TAG_SOME]);
        base_u16(hasher, retry.max_attempts);
        base_optional_string(hasher, b"retry_delay:", retry.delay.as_deref());
    } else {
        hasher.update(&[TAG_NONE]);
    }
    hasher.update(b"step_on_error:");
    if let Some(handler) = &step.on_error {
        hasher.update(&[TAG_SOME]);
        base_string(hasher, &handler.handler);
    } else {
        hasher.update(&[TAG_NONE]);
    }
    base_optional_string(hasher, b"step_then:", step.then.as_deref());
}

fn base_body(hasher: &mut Hasher, body: &[StepAst]) {
    base_len(hasher, body.len());
    for step in body {
        base_step(hasher, step);
    }
    hasher.update(&[TAG_END]);
}

#[allow(clippy::too_many_lines)]
fn base_primitive(hasher: &mut Hasher, primitive: &StepPrimitive) {
    match primitive {
        StepPrimitive::Set { output, value } => {
            hasher.update(b"set");
            base_string(hasher, output);
            base_string(hasher, value);
        }
        StepPrimitive::Save { value } => {
            hasher.update(b"save");
            base_scalar(hasher, value);
        }
        StepPrimitive::Do { action, input } => {
            hasher.update(b"do");
            base_string(hasher, action);
            base_string(hasher, input);
        }
        StepPrimitive::Choose {
            branches,
            otherwise,
        } => {
            hasher.update(b"choose");
            base_len(hasher, branches.len());
            for branch in branches {
                base_string(hasher, &branch.when);
                base_len(hasher, branch.steps.len());
                for step in &branch.steps {
                    base_step(hasher, step);
                }
                hasher.update(&[TAG_END]);
            }
            hasher.update(&[TAG_END]);
            base_optional_string(hasher, b"otherwise:", otherwise.as_deref());
        }
        StepPrimitive::ForEach {
            variable,
            input,
            at_once,
            body,
        } => {
            hasher.update(b"for_each");
            base_string(hasher, variable);
            base_string(hasher, input);
            base_optional_u32(hasher, b"at_once:", *at_once);
            base_body(hasher, body);
        }
        StepPrimitive::Together { branches } => {
            hasher.update(b"together");
            let count = u16::try_from(branches.len()).expect("test branches fit u16");
            base_u16(hasher, count);
            for branch in branches {
                base_string(hasher, &branch.label);
                base_len(hasher, branch.steps.len());
                for step in &branch.steps {
                    base_step(hasher, step);
                }
                hasher.update(&[TAG_END]);
            }
            hasher.update(&[TAG_END]);
        }
        StepPrimitive::Collect {
            variable,
            source,
            pages,
            items,
            body,
        } => {
            hasher.update(b"collect");
            base_string(hasher, variable);
            base_string(hasher, source);
            base_optional_u32(hasher, b"pages:", *pages);
            base_optional_u32(hasher, b"items:", *items);
            base_body(hasher, body);
        }
        StepPrimitive::Aggregate {
            variable,
            input,
            initial,
            body,
        } => {
            hasher.update(b"aggregate");
            base_string(hasher, variable);
            base_string(hasher, input);
            base_string(hasher, initial);
            base_body(hasher, body);
        }
        StepPrimitive::Wait { event, timeout } => {
            hasher.update(b"wait");
            base_optional_string(hasher, b"event:", event.as_deref());
            base_optional_string(hasher, b"timeout:", timeout.as_deref());
        }
        StepPrimitive::Ask { prompt, timeout } => {
            hasher.update(b"ask");
            base_string(hasher, prompt);
            base_optional_string(hasher, b"timeout:", timeout.as_deref());
        }
        StepPrimitive::Finish { result } => {
            hasher.update(b"finish");
            base_scalar(hasher, result);
        }
        StepPrimitive::Repeat { max_attempts, body } => {
            hasher.update(b"repeat");
            base_u16(hasher, *max_attempts);
            base_body(hasher, body);
        }
        _ => panic!("test baseline never sees unknown primitive variant"),
    }
}

fn base_primitive_bytes(primitive: &StepPrimitive) -> [u8; 32] {
    let mut hasher = Hasher::new();
    base_primitive(&mut hasher, primitive);
    *hasher.finalize().as_bytes()
}

fn prod_primitive_bytes(primitive: &StepPrimitive) -> [u8; 32] {
    let mut hasher = Hasher::new();
    digest_step_primitive(&mut hasher, primitive).expect("valid test primitive within budget");
    *hasher.finalize().as_bytes()
}

fn set_leaf(id: &str) -> StepAst {
    StepAst {
        id: id.to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: "x".to_string(),
            value: "1".to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }
}

fn foreach_wrap(id: &str, body: Vec<StepAst>) -> StepAst {
    StepAst {
        id: id.to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::ForEach {
            variable: "v".to_string(),
            input: "items".to_string(),
            at_once: None,
            body,
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }
}

/// `levels` ForEach wrappers around a Set leaf. Innermost Set depth == `levels`.
fn build_foreach_chain(levels: usize) -> StepPrimitive {
    let mut body = vec![set_leaf("leaf")];
    let mut remaining = levels;
    while remaining > 1 {
        body = vec![foreach_wrap("nest", body)];
        remaining -= 1;
    }
    StepPrimitive::ForEach {
        variable: "v".to_string(),
        input: "items".to_string(),
        at_once: None,
        body,
    }
}

fn build_seq_chain(depth: usize) -> AuthorValue {
    let mut value = AuthorValue::Text("leaf".to_string());
    for _ in 0..depth {
        value = AuthorValue::Sequence(vec![value]);
    }
    value
}

fn build_map_chain(depth: usize) -> AuthorValue {
    let mut value = AuthorValue::Text("leaf".to_string());
    for _ in 0..depth {
        value = AuthorValue::Mapping(vec![AuthorEntry {
            key: "k".to_string(),
            value,
        }]);
    }
    value
}

fn run_primitive_bounded(primitive: StepPrimitive) -> Result<(), crate::CompileErrors> {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || {
            let mut hasher = Hasher::new();
            digest_step_primitive(&mut hasher, &primitive)
        })
        .expect("spawn bounded digest thread")
        .join()
        .expect("join bounded digest thread")
}

fn run_author_bounded(value: AuthorValue) -> Result<(), crate::CompileErrors> {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || {
            let mut hasher = Hasher::new();
            digest_author_value(&mut hasher, &value)
        })
        .expect("spawn bounded author digest thread")
        .join()
        .expect("join bounded author digest thread")
}

#[test]
fn digest_foreach_chain_64_matches_recursive_baseline() {
    let primitive = build_foreach_chain(64);
    assert_eq!(
        prod_primitive_bytes(&primitive),
        base_primitive_bytes(&primitive),
        "64-deep iterative digest must equal recursive baseline byte-for-byte"
    );
}

#[test]
fn digest_foreach_chain_64_in_bounded_thread_no_overflow() {
    let primitive = build_foreach_chain(64);
    let expected = base_primitive_bytes(&primitive);
    let result = std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || {
            let mut hasher = Hasher::new();
            digest_step_primitive(&mut hasher, &primitive).expect("64-deep within budget");
            *hasher.finalize().as_bytes()
        })
        .expect("spawn bounded thread")
        .join()
        .expect("join bounded thread");
    assert_eq!(
        result, expected,
        "64-deep digest on 256KiB stack must equal baseline without stack overflow"
    );
}

#[test]
fn digest_foreach_chain_65_fails_closed_without_stack_overflow() {
    let primitive = build_foreach_chain(65);
    let result = run_primitive_bounded(primitive);
    match result {
        Err(errors) => assert!(
            errors
                .0
                .iter()
                .any(|e| matches!(e, CompileError::PrimitiveLoweringLimitExceeded { .. })),
            "65-deep must fail with PrimitiveLoweringLimitExceeded, got {errors:?}"
        ),
        Ok(()) => panic!("65-deep must fail closed, not succeed"),
    }
}

#[test]
fn digest_author_seq_chain_64_matches_recursive_baseline() {
    let value = build_seq_chain(64);
    let mut prod_hasher = Hasher::new();
    digest_author_value(&mut prod_hasher, &value).expect("64-deep within budget");
    let prod = *prod_hasher.finalize().as_bytes();
    let mut base_hasher = Hasher::new();
    base_author_value(&mut base_hasher, &value);
    assert_eq!(
        prod,
        *base_hasher.finalize().as_bytes(),
        "64-deep AuthorValue sequence digest must equal recursive baseline"
    );
}

#[test]
fn digest_author_seq_chain_65_fails_closed_without_stack_overflow() {
    let value = build_seq_chain(65);
    let result = run_author_bounded(value);
    match result {
        Err(errors) => assert!(
            errors
                .0
                .iter()
                .any(|e| matches!(e, CompileError::PrimitiveLoweringLimitExceeded { .. })),
            "65-deep AuthorValue must fail with PrimitiveLoweringLimitExceeded, got {errors:?}"
        ),
        Ok(()) => panic!("65-deep AuthorValue must fail closed, not succeed"),
    }
}

#[test]
fn digest_author_map_chain_64_matches_recursive_baseline() {
    let value = build_map_chain(64);
    let mut prod_hasher = Hasher::new();
    digest_author_value(&mut prod_hasher, &value).expect("64-deep within budget");
    let prod = *prod_hasher.finalize().as_bytes();
    let mut base_hasher = Hasher::new();
    base_author_value(&mut base_hasher, &value);
    assert_eq!(
        prod,
        *base_hasher.finalize().as_bytes(),
        "64-deep AuthorValue mapping digest must equal recursive baseline"
    );
}

#[test]
fn digest_author_map_chain_65_fails_closed_without_stack_overflow() {
    let value = build_map_chain(65);
    let result = run_author_bounded(value);
    match result {
        Err(errors) => assert!(
            errors
                .0
                .iter()
                .any(|e| matches!(e, CompileError::PrimitiveLoweringLimitExceeded { .. })),
            "65-deep AuthorValue mapping must fail with PrimitiveLoweringLimitExceeded"
        ),
        Ok(()) => panic!("65-deep AuthorValue mapping must fail closed, not succeed"),
    }
}

#[test]
fn digest_mixed_nested_primitives_match_recursive_baseline() {
    let leaf = set_leaf("leaf");
    let repeat_step = StepAst {
        id: "r".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Repeat {
            max_attempts: 3,
            body: vec![leaf.clone()],
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    };
    let together_step = StepAst {
        id: "t".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Together {
            branches: vec![crate::TogetherBranch {
                label: "b1".to_string(),
                steps: vec![repeat_step],
            }],
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    };
    let choose_step = StepAst {
        id: "c".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Choose {
            branches: vec![crate::ChooseBranch {
                when: "yes".to_string(),
                steps: vec![together_step],
            }],
            otherwise: Some("done".to_string()),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    };
    let collect_step = StepAst {
        id: "co".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Collect {
            variable: "v".to_string(),
            source: "s".to_string(),
            pages: None,
            items: None,
            body: vec![choose_step],
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    };
    let primitive = StepPrimitive::ForEach {
        variable: "v".to_string(),
        input: "items".to_string(),
        at_once: Some(2),
        body: vec![collect_step],
    };
    assert_eq!(
        prod_primitive_bytes(&primitive),
        base_primitive_bytes(&primitive),
        "mixed ForEach/Collect/Choose/Together/Repeat nesting must match baseline"
    );
}

#[test]
fn validate_branch_counts_65_deep_fails_closed() {
    use crate::mod_compile_lowering::part_05::validate_branch_counts;
    use crate::{TriggerAst, WorkflowSource, WorkflowSourceParts};
    let mut body = vec![set_leaf("leaf")];
    let mut remaining = 65;
    while remaining > 1 {
        body = vec![foreach_wrap("nest", body)];
        remaining -= 1;
    }
    let top = StepAst {
        id: "top".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::ForEach {
            variable: "v".to_string(),
            input: "items".to_string(),
            at_once: None,
            body,
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    };
    let source = WorkflowSource::new(WorkflowSourceParts {
        version: "velvet-ballistics/v1".to_string(),
        name: "tco".to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![top],
        result: None,
        examples: vec![],
    });
    let result = std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || validate_branch_counts(&source))
        .expect("spawn bounded validate thread")
        .join()
        .expect("join bounded validate thread");
    match result {
        Err(errors) => assert!(
            errors
                .0
                .iter()
                .any(|e| matches!(e, CompileError::PrimitiveLoweringLimitExceeded { .. })),
            "validate must fail closed at 65-deep, got {errors:?}"
        ),
        Ok(()) => panic!("validate must fail closed at 65-deep, not succeed"),
    }
}

#[test]
fn digest_nested_with_payload_matches_recursive_baseline() {
    let mut step = set_leaf("payload");
    step.with = Some(AuthorValue::Mapping(vec![AuthorEntry {
        key: "args".into(),
        value: AuthorValue::Sequence(vec![
            AuthorValue::Bool(true),
            AuthorValue::I64(7),
            AuthorValue::Text("$vars.seed".into()),
        ]),
    }]));
    let primitive = StepPrimitive::Repeat {
        max_attempts: 2,
        body: vec![step],
    };
    assert_eq!(
        prod_primitive_bytes(&primitive),
        base_primitive_bytes(&primitive)
    );
}
