use super::fixtures::*;
use super::*;

fn assert_width_results(
    body: &[StepAst],
    overhead: usize,
    legacy: Result<usize, CompileError>,
    canonical: Result<usize, CompileError>,
) {
    let measured_legacy = body_width(body, overhead);
    let measured_canonical = canonical_body_width(body, overhead);
    let measured_production = super::super::width::body_width(body, overhead);
    assert_eq!(measured_legacy, legacy);
    assert_eq!(measured_canonical, canonical);
    assert_eq!(measured_production, canonical);
}

fn assert_width_modes(body: &[StepAst], overhead: usize, legacy: usize, canonical: usize) {
    assert_width_results(body, overhead, Ok(legacy), Ok(canonical));
}

#[test]
fn separate_empty_container_policies() {
    let empty = vec![];
    let body = vec![
        for_each(empty.clone(), None),
        collect(empty.clone(), None, None),
        repeat(empty.clone(), 0),
        choose(vec![]),
    ];
    assert_width_modes(&body, 0, 9, 14);
}

#[test]
fn canonical_do_condition_expands_without_affecting_legacy() {
    let mut s = do_step();
    s.condition = Some("true".to_owned());
    let body = vec![s];
    assert_width_modes(&body, 0, 1, 2);
}

#[test]
fn canonical_save_is_not_legacy() {
    let body = vec![step(unsupported(0))];
    assert_width_results(
        &body,
        0,
        Err(CompileError::UnsupportedStepPrimitive {
            step: 0,
            primitive: "save",
        }),
        Ok(1),
    );
}

#[test]
fn nested_children_keep_policy() {
    let body = nested_body();
    assert_width_modes(&body, 5, 18, 23);
}

#[test]
fn tracking_is_not_legacy_mode() {
    let mut s = do_step();
    s.condition = Some("true".to_owned());
    let body = vec![for_each(vec![s], None)];
    assert_width_modes(&body, 0, 3, 10);
}

#[test]
fn each_policy_keeps_its_error_order() {
    let body = vec![for_each(vec![step(unsupported(3))], None)];
    assert_width_results(
        &body,
        usize::MAX,
        Err(CompileError::UnsupportedStepPrimitive {
            step: 0,
            primitive: "wait",
        }),
        Err(CompileError::StepIndexOutOfRange { value: usize::MAX }),
    );
}

#[test]
fn deep_repeat_keeps_both_literal_widths() {
    let body = deep_repeat_body();
    let l = body_width(&body, 0);
    let c = canonical_body_width(&body, 0);
    let p = super::super::width::body_width(&body, 0);
    release_repeat_body(body);
    assert_eq!(l, Ok(12289));
    assert_eq!(c, Ok(16385));
    assert_eq!(p, Ok(16385));
}
