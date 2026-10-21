use super::*;

#[test]
fn malformed_ir_and_trailing_payload_bytes_reject_after_envelope_rehash() -> TestResult {
    let original = prepare_compiled_artifact(&workflow()?, RuntimePolicy::Journaled, &[])?;
    let mut trailing = original.clone();
    trailing.ir.push(0);
    rebind(&mut trailing)?;
    assert!(matches!(
        verify_accepted_artifact(&trailing),
        Err(JournalError::ArtifactMalformed)
    ));
    let mut invalid = original;
    let mut parts: vb_core::WorkflowParts = postcard::from_bytes(&invalid.ir)?;
    parts.resource_contract.max_steps = 1;
    invalid.ir = postcard::to_allocvec(&parts)?;
    rebind(&mut invalid)?;
    match verify_accepted_artifact(&invalid) {
        Err(JournalError::WorkflowReconstruction(
            vb_core::WorkflowError::ResourceContractExceeded { resource },
        )) => assert_eq!(resource, "max_steps"),
        other => {
            return Err(format!("expected real resource limit rejection, got {other:?}").into());
        }
    }
    Ok(())
}

#[test]
fn changed_valid_ir_and_contract_declarations_cannot_reuse_old_evidence() -> TestResult {
    let mut artifact = prepare_compiled_artifact(&workflow()?, RuntimePolicy::Journaled, &[])?;
    let mut parts: vb_core::WorkflowParts = postcard::from_bytes(&artifact.ir)?;
    parts.constants = Box::new([vb_core::ConstValue::I64(43)]);
    let altered = canonicalize(parts)?;
    artifact.ir = postcard::to_allocvec(&altered.to_parts())?;
    rebind(&mut artifact)?;
    assert!(matches!(
        verify_accepted_artifact(&artifact),
        Err(JournalError::ArtifactMalformed)
    ));
    let mut artifact = prepare_compiled_artifact(
        &action_workflow()?,
        RuntimePolicy::Journaled,
        &[contract()?],
    )?;
    let declaration = artifact
        .action_contracts
        .first_mut()
        .ok_or("missing action contract")?;
    declaration.max_output_bytes = 63;
    rebind(&mut artifact)?;
    assert!(matches!(
        verify_accepted_artifact(&artifact),
        Err(JournalError::ArtifactMalformed)
    ));
    Ok(())
}

#[test]
fn bound_is_checked_before_decoding_or_hashing_the_payload() -> TestResult {
    let mut artifact = prepare_compiled_artifact(&workflow()?, RuntimePolicy::Journaled, &[])?;
    artifact.ir = vec![0; 16_777_217];
    assert!(matches!(
        verify_accepted_artifact(&artifact),
        Err(JournalError::PayloadTooLarge {
            len: 16_777_217,
            max: 16_777_216
        })
    ));
    Ok(())
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(64))]
    #[test]
    fn arbitrary_payload_bit_change_is_rejected(value in -10_000i64..10_000, offset in 0usize..100_000) {
        let source=format!("version: velvet-ballistics/v1\nname: artifact_property\nwhen: {{ manual: {{}} }}\nsteps:\n  - id: make\n    set: {{ output: answer, value: \"{value}\" }}\n  - id: done\n    finish: {{ result: answer }}\n");
        let workflow=vb_compile::compile_workflow(source.as_bytes()).map_err(|e|proptest::test_runner::TestCaseError::fail(e.to_string()))?;
        let mut artifact=prepare_compiled_artifact(&workflow,RuntimePolicy::Journaled,&[]).map_err(|e|proptest::test_runner::TestCaseError::fail(e.to_string()))?;
        proptest::prop_assert_eq!(verify_accepted_artifact(&artifact).map_err(|e|proptest::test_runner::TestCaseError::fail(e.to_string()))?,workflow);
        let index=offset.checked_rem(artifact.ir.len()).ok_or_else(||proptest::test_runner::TestCaseError::fail("empty IR"))?;
        let byte=artifact.ir.get_mut(index).ok_or_else(||proptest::test_runner::TestCaseError::fail("IR index unavailable"))?;
        *byte ^= 1;
        proptest::prop_assert!(matches!(verify_accepted_artifact(&artifact),Err(JournalError::ArtifactChecksumMismatch)));
    }
}
