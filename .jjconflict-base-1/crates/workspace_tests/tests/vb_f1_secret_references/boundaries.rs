use super::*;

#[test]
fn duplicate_and_unknown_secret_aliases_fail_with_declared_names() -> TestResult {
    for (secrets, result, duplicate) in [
        ("{token: TOKEN, token: OTHER}", "$secrets.token", true),
        ("{token: TOKEN}", "$secrets.missing", false),
    ] {
        let source = format!(
            "version: velvet-ballistics/v1\nname: aliases\nwhen: {{manual: {{}}}}\nsecrets: {secrets}\nsteps:\n  - id: done\n    finish: {{result: '{result}'}}\n"
        );
        let errors = vb_compile::compile_workflow(source.as_bytes())
            .err()
            .ok_or("invalid secret alias accepted")?;
        assert_eq!(errors.0.len(), 1);
        if duplicate {
            assert_eq!(
                errors.0,
                vec![vb_compile::CompileError::CanonicalYaml {
                    category: "duplicate_key",
                    message: "duplicate key found: token at line 4, column 24".into()
                }]
            );
        } else {
            assert_eq!(
                errors.0,
                vec![vb_compile::CompileError::UnknownReferenceName {
                    kind: "secrets",
                    reference: "$secrets.missing".into(),
                    name: "missing".into()
                }]
            );
        }
    }
    Ok(())
}

#[test]
fn immutable_secret_roots_reject_implicit_writers() -> TestResult {
    let workflow = workflow()?;
    let slot = workflow
        .source_metadata()
        .ok_or("metadata")?
        .secret_requirements()
        .first()
        .ok_or("binding")?
        .slot()
        .ok_or("slot")?;
    let step = vb_core::StepIdx::new;
    let kinds = [
        CompiledNodeKind::ForEachNext {
            iterator_slot: slot,
            body: step(1),
            done: step(2),
        },
        CompiledNodeKind::ReduceNext {
            iterator_slot: slot,
            accumulator: vb_core::SlotIdx::new(0),
            body: step(1),
            done: step(2),
        },
        CompiledNodeKind::RetryCheck {
            policy_slot: slot,
            body: step(1),
            exhausted: step(2),
        },
        CompiledNodeKind::ErrorHandler {
            body: step(1),
            handler: step(2),
            error_slot: Some(slot).into(),
        },
    ];
    for kind in kinds {
        let mut parts = workflow.to_parts();
        parts.nodes.first_mut().ok_or("node")?.kind = kind;
        assert_eq!(
            CompiledWorkflow::try_from_parts(parts).map(|_| ()),
            Err(vb_core::WorkflowError::SourceMetadata {
                detail: "invalid immutable secret input binding".into()
            })
        );
    }
    let mut parts = workflow.to_parts();
    parts.nodes.first_mut().ok_or("Ask")?.kind = CompiledNodeKind::Ask {
        prompt: slot,
        timeout_slot: None.into(),
    };
    parts.nodes.first_mut().ok_or("Ask output")?.output = vb_core::workflow::NodeOutput::Discard;
    parts.nodes.get_mut(1).ok_or("AskResume")?.kind = CompiledNodeKind::AskResume { answer: slot };
    assert_eq!(
        CompiledWorkflow::try_from_parts(parts).map(|_| ()),
        Err(vb_core::WorkflowError::SourceMetadata {
            detail: "invalid immutable secret input binding".into()
        })
    );

    let mut parts = workflow.to_parts();
    parts.nodes.first_mut().ok_or("reader")?.kind = CompiledNodeKind::AskResume { answer: slot };
    CompiledWorkflow::try_from_parts(parts)?;
    Ok(())
}

#[test]
fn raw_shard_submission_cannot_bypass_presence_or_forge_an_opaque_root() -> TestResult {
    let workflow = workflow()?;
    let binding = *workflow
        .source_metadata()
        .ok_or("metadata")?
        .secret_requirements()
        .first()
        .ok_or("binding")?;
    let journal = std::sync::Arc::new(vb_runtime::journal::VolatileRuntimeJournal::new());
    let mut shard = vb_runtime::shard::Shard::new_with_journal(
        vb_runtime::shard::ShardConfig {
            policy: vb_core::RuntimePolicy::Relaxed,
            ..vb_runtime::shard::ShardConfig::default()
        },
        journal.clone(),
    );
    assert_eq!(
        shard.enqueue(vb_runtime::shard::ShardCommand::Submit {
            run: RunId::new(20),
            workflow: workflow.clone(),
            caps: CapabilitySet::empty()
        }),
        Err(RuntimeError::SecretUnavailable)
    );
    shard.set_available_secret_names(std::sync::Arc::from([Box::<str>::from("TOKEN")]));
    assert_eq!(
        shard.enqueue(vb_runtime::shard::ShardCommand::SubmitWithInputs {
            run: RunId::new(20),
            workflow,
            inputs: Box::new([(
                binding.slot().ok_or("slot")?,
                SlotValue::Symbol(binding.alias())
            )]),
            caps: CapabilitySet::empty()
        }),
        Err(vb_core::CoreError::InternalInvariantViolation {
            reason: vb_core::InvariantKind::Unknown("secret_input_binding")
        }
        .into())
    );
    assert_eq!(journal.snapshot()?, vec![]);
    Ok(())
}

#[test]
fn restored_frame_keeps_exact_secret_handle_and_taint() -> TestResult {
    let workflow = workflow()?;
    let binding = *workflow
        .source_metadata()
        .ok_or("metadata")?
        .secret_requirements()
        .first()
        .ok_or("binding")?;
    let root = binding.slot().ok_or("slot")?;
    let (mut runtime, _) = runtime();
    runtime.set_available_secret_names(Box::new([Box::from("TOKEN")]));
    submit(&mut runtime, RunId::new(21), workflow.clone())?;
    let state = runtime.inspect_run_state(RunId::new(21))?;
    let frame = state.frame.checkpoint_v1();
    let store = state.store.checkpoint_v1();
    for change in 0..4 {
        let mut frame = frame.clone();
        match change {
            0 => {}
            1 => *frame.taint.get_mut(root.as_usize()).ok_or("taint")? = Taint::Clean,
            2 => {
                *frame.slots.get_mut(root.as_usize()).ok_or("slot")? =
                    Some(SlotValue::Symbol(binding.alias()))
            }
            _ => {
                *frame.slots.get_mut(root.as_usize()).ok_or("slot")? = None;
                *frame.taint.get_mut(root.as_usize()).ok_or("taint")? = Taint::Clean;
            }
        }
        let bytes = postcard::to_allocvec(&frame)?;
        let restored = vb_core::RunFrame::from_checkpoint_v1(postcard::from_bytes(&bytes)?)
            .map_err(|error| format!("frame restore: {error:?}"))?;
        let result = vb_core::root_bundle::validate_runtime_store(&restored, &store, &workflow);
        if change == 0 {
            result?;
        } else {
            let Err(vb_core::root_bundle::OwnedRootBundleError::Checkpoint(error)) = result else {
                return Err("forged frame accepted or wrong rejection".into());
            };
            assert_eq!(
                error,
                vb_core::CoreError::InternalInvariantViolation {
                    reason: vb_core::InvariantKind::Unknown("secret_input_binding")
                }
            );
        }
    }
    Ok(())
}
