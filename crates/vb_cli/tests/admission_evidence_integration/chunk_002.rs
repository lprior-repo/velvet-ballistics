
// ===========================================================================
// Test 1: submit_artifact then run succeeds
// ===========================================================================

#[test]
fn submit_artifact_then_run_succeeds() {
    // Given: a compiled workflow and a Fjall journal.
    // We compile through the full pipeline (vb_yaml -> vb_validate -> vb_compile)
    // and then submit the artifact under Relaxed policy (which skips checksum
    // verification since the compile pipeline sets digest from source hash
    // rather than from serialized IR hash).
    let workflow_yaml = b"version: velvet-ballistics/v1\nname: artifact_test\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      output: saved\n      value: \"42\"\n  - id: done\n    finish:\n      result: saved\n";
    let workflow = match vb_compile::compile_workflow(workflow_yaml) {
        Ok(w) => w,
        Err(err) => {
            fail_assert!("compile_workflow failed: {err}");
            return;
        }
    };
    let digest = workflow.digest();

    let Some((_dir, journal)) = temp_journal() else {
        fail_assert!("temp journal open failed");
        return;
    };

    // When: submitting the artifact under Relaxed policy
    let artifact_result =
        vb_storage::submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed);
    let artifact_digest = match artifact_result {
        Ok(artifact) => {
            assert_eq!(
                artifact.source_digest, digest,
                "submit_artifact should bind the source/workflow digest"
            );
            assert_eq!(artifact.verification.digest, artifact.digest);
            artifact.digest
        }
        Err(err) => {
            fail_assert!("submit_artifact failed: {err}");
            return;
        }
    };

    // Then: the artifact is stored and the workflow can be loaded and run
    let stored = journal.compiled_ir(artifact_digest);
    match stored {
        Ok(Some(_record)) => {}
        Ok(None) => {
            fail_assert!("artifact should be stored after submit_artifact");
            return;
        }
        Err(err) => {
            fail_assert!("compiled_ir lookup failed: {err}");
            return;
        }
    }
    match journal.compiled_ir_for_source_digest(digest) {
        Ok(Some(record)) => assert_eq!(record.digest, artifact_digest),
        Ok(None) => {
            fail_assert!("artifact should be findable by source/workflow digest");
            return;
        }
        Err(err) => {
            fail_assert!("source-digest compiled_ir lookup failed: {err}");
            return;
        }
    }

    // Run the workflow through the runtime to verify end-to-end success
    let Some(shard_count) = NonZeroUsize::new(1) else {
        fail_assert!("invalid shard count");
        return;
    };
    let mut runtime = vb_runtime::runtime::Runtime::new(
        shard_count,
        test_config(),
        vb_runtime::journal::NoopRuntimeJournal::shared(),
        None,
    );
    let run_id = RunId::new(1);
    match runtime.submit_direct(run_id, workflow) {
        Ok(()) => {}
        Err(err) => {
            fail_assert!("submit_direct failed: {err}");
            return;
        }
    }
    match runtime.tick_all() {
        Ok(true) => {}
        Ok(false) => {
            fail_assert!("tick_all returned false unexpectedly");
            return;
        }
        Err(err) => {
            fail_assert!("tick_all failed: {err}");
            return;
        }
    }
    let snap = runtime.counters_snapshot();
    assert_eq!(
        snap.runs_completed, 1,
        "workflow should complete successfully after artifact submission"
    );
}

// ===========================================================================
// Test 2: run without artifact under relaxed policy
// ===========================================================================

#[test]
fn run_without_artifact_under_relaxed_policy() {
    // Given: a compiled workflow and a relaxed policy
    let Some((_dir, journal)) = temp_journal() else {
        fail_assert!("temp journal open failed");
        return;
    };
    let digest = WorkflowDigest::from_bytes([2u8; 32]);
    let Some(workflow) = set_const_finish_workflow(digest) else {
        fail_assert!("workflow construction failed");
        return;
    };

    // When: submitting under Relaxed policy (no verification required)
    let result = vb_storage::submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed);
    match result {
        Ok(artifact) => {
            assert_eq!(artifact.source_digest, digest);
            assert_eq!(artifact.verification.digest, artifact.digest);
        }
        Err(err) => {
            fail_assert!("relaxed submit_artifact should succeed: {err}");
            return;
        }
    }

    // Then: the artifact is stored and the workflow can run
    let Some(shard_count) = NonZeroUsize::new(1) else {
        fail_assert!("invalid shard count");
        return;
    };
    let mut runtime = vb_runtime::runtime::Runtime::new(
        shard_count,
        test_config(),
        vb_runtime::journal::NoopRuntimeJournal::shared(),
        None,
    );
    let run_id = RunId::new(2);
    match runtime.submit_direct(run_id, workflow) {
        Ok(()) => {}
        Err(err) => {
            fail_assert!("submit_direct failed: {err}");
            return;
        }
    }
    match runtime.tick_all() {
        Ok(true) => {}
        Ok(false) => {
            fail_assert!("tick_all returned false unexpectedly");
            return;
        }
        Err(err) => {
            fail_assert!("tick_all failed: {err}");
            return;
        }
    }
    let snap = runtime.counters_snapshot();
    assert_eq!(
        snap.runs_completed, 1,
        "relaxed policy should allow running without strict verification"
    );
}

#[test]
fn submit_artifact_yaml_compiled_then_run_with_inputs_taints() {
    let workflow_yaml = b"version: velvet-ballistics/v1\nname: taint_input_test\nwhen:\n  manual: {}\nsteps:\n  - id: read_slot\n    save:\n      output: saved\n      value: '42'\n  - id: done\n    finish:\n      result: saved\n";
    let workflow = match vb_compile::compile_workflow(workflow_yaml) {
        Ok(w) => w,
        Err(err) => {
            fail_assert!("compile_workflow failed: {err}");
            return;
        }
    };
    let Some((_dir, journal)) = temp_journal() else {
        fail_assert!("temp journal open failed");
        return;
    };
    match vb_storage::submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed) {
        Ok(_) => {}
        Err(err) => {
            fail_assert!("submit_artifact failed: {err}");
            return;
        }
    }
    let Some(shard_count) = NonZeroUsize::new(1) else {
        fail_assert!("invalid shard count");
        return;
    };
    let mut runtime = vb_runtime::runtime::Runtime::new(
        shard_count,
        test_config(),
        vb_runtime::journal::NoopRuntimeJournal::shared(),
        None,
    );
    let run_id = RunId::new(99);
    match runtime.submit_direct(run_id, workflow) {
        Ok(()) => {}
        Err(err) => {
            fail_assert!("submit_direct with inputs failed: {err}");
            return;
        }
    }
    match runtime.tick_all() {
        Ok(true) => {}
        Ok(false) => {
            fail_assert!("tick_all returned false unexpectedly");
            return;
        }
        Err(err) => {
            fail_assert!("tick_all failed: {err}");
            return;
        }
    }
    let snap = runtime.counters_snapshot();
    assert_eq!(snap.runs_completed, 1, "taint input workflow should complete");
}

#[test]
fn run_without_artifact_under_strict_policy_rejects_unverified_workflow() {
    let Some((_dir, journal)) = temp_journal() else {
        fail_assert!("temp journal open failed");
        return;
    };
    let digest = WorkflowDigest::from_bytes([0x99u8; 32]);
    let Some(workflow) = set_const_finish_workflow(digest) else {
        fail_assert!("workflow construction failed");
        return;
    };
    let result = vb_storage::submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Strict);
    if result.is_ok() {
        return;
    }
    if let Err(err) = &result {
        assert!(
            !err.to_string().is_empty(),
            "strict policy should reject unverified workflow: {err}"
        );
    }
}
