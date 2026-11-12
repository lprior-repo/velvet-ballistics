use super::*;

#[test]
fn body_depth_limit_and_error_restore_the_callers_budget() {
    let outputs = HashMap::new();
    let steps = HashMap::new();
    let types = HashMap::new();
    let scope = CanonicalScope {
        outputs: &outputs,
        step_outputs: &steps,
        slot_types: &types,
        parent: None,
    };
    for (existing, accepted) in [(62, true), (63, true), (64, false)] {
        let mut builder = seeded_builder(0);
        builder.body_depth = existing;
        let result = emit_scoped_typed_body(
            &[step(0, 0)],
            StepIdx::new(1),
            StepIdx::new(77),
            scope,
            &mut builder,
            "collect",
        );
        assert_eq!(
            result.is_ok(),
            accepted,
            "existing depth {existing}: {result:?}"
        );
        assert_eq!(builder.body_depth, existing);
        if !accepted {
            assert!(
                matches!(result, Err(CompileErrors(ref errors)) if errors.iter().any(|error|
                    matches!(error, CompileError::PrimitiveLoweringLimitExceeded {
                        field: "body nesting depth", value: 65, limit: 64, ..
                    })
                ))
            );
        }
    }
    let mut builder = seeded_builder(0);
    builder.body_depth = 63;
    let result = emit_scoped_typed_body(
        &[step(0, 4)],
        StepIdx::new(1),
        StepIdx::new(77),
        scope,
        &mut builder,
        "collect",
    );
    assert!(
        matches!(result, Err(CompileErrors(ref errors)) if errors.iter().any(|error|
            matches!(error, CompileError::UnknownReferenceName { .. })
        ))
    );
    assert_eq!(builder.body_depth, 63);
}
