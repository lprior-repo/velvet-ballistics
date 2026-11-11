fn assert_program_constructors_reject_has_depth(depth: MalformedHasDepth) {
    let ops = depth.ops();
    let expected = depth.constructor_error();
    assert_eq!(
        ExprProgram::try_from_ops(ops.clone().into_boxed_slice()),
        Err(expected.clone())
    );
    assert_eq!(
        ExprProgram::try_from_parts(ops.into_boxed_slice(), depth.values()),
        Err(expected)
    );
}

fn assert_workflow_constructor_rejects_has_depth(depth: MalformedHasDepth) {
    let key = SymbolId::new(0);
    let actual = crate::workflow::CompiledWorkflow::try_from_parts(has_bytecode_parts(
        malformed_has_program(depth),
        key,
    ));
    assert_eq!(
        actual,
        Err(crate::workflow::WorkflowError::Expression(
            depth.constructor_error()
        ))
    );
}

#[test]
fn has_bytecode_boundary_validators_reject_zero_one_and_three_stack_values() {
    for depth in [
        MalformedHasDepth::Zero,
        MalformedHasDepth::One,
        MalformedHasDepth::Three,
    ] {
        assert_program_constructors_reject_has_depth(depth);
        assert_workflow_constructor_rejects_has_depth(depth);
    }
}

#[test]
#[cfg(feature = "test-util")]
fn has_bytecode_boundary_hot_underflows_with_zero_stack_values() -> Result<(), CoreError> {
    let (mut store, run, key) = has_bytecode_fixture()?;
    assert_eq!(
        eval_hot_has(
            malformed_has_program(MalformedHasDepth::Zero),
            key,
            &run,
            &mut store,
        ),
        Err(crate::errors::EngineError::ExpressionStackUnderflow)
    );
    Ok(())
}

#[test]
#[cfg(feature = "test-util")]
fn has_bytecode_boundary_hot_underflows_with_one_stack_value() -> Result<(), CoreError> {
    let (mut store, run, key) = has_bytecode_fixture()?;
    assert_eq!(
        eval_hot_has(
            malformed_has_program(MalformedHasDepth::One),
            key,
            &run,
            &mut store,
        ),
        Err(crate::errors::EngineError::ExpressionStackUnderflow)
    );
    Ok(())
}

#[test]
#[cfg(feature = "test-util")]
fn has_bytecode_boundary_hot_rejects_three_values_at_finalization() -> Result<(), CoreError> {
    let (mut store, run, key) = has_bytecode_fixture()?;
    assert_eq!(
        eval_hot_has(
            malformed_has_program(MalformedHasDepth::Three),
            key,
            &run,
            &mut store,
        ),
        Err(crate::errors::EngineError::InvalidCompiledWorkflow {
            reason: "expression leaves non-single result",
        })
    );
    Ok(())
}

#[test]
fn has_bytecode_boundary_hot_validated_control_returns_true() -> Result<(), CoreError> {
    let (mut store, run, key) = has_bytecode_fixture()?;
    let plan = checked_has_plan(key).map_err(|_| CoreError::InvalidCompiledWorkflow {
        reason: "valid Has positive-control plan rejected",
    })?;
    assert_eq!(
        crate::engine::eval_expr_with_store(&plan, &run, &mut store, ExprIdx::new(0)),
        Ok((SlotValue::Bool(true), Taint::Clean))
    );
    Ok(())
}
