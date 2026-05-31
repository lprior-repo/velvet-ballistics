//! Workflow run command and artifact storage.
    workflow: &std::path::Path,
    input_bin: &std::path::Path,
    durability: DurabilityMode,
    db: Option<&std::path::Path>,
    output: OutputFormat,
) -> ExitCode {
    let input_data = match read_file(input_bin, output, CliExitCode::ValidationFailed) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let bytes = match read_file(workflow, output, CliExitCode::ValidationFailed) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let compiled = match vb_compile::compile_workflow(&bytes) {
        Ok(c) => c,
        Err(errors) => {
            if output != OutputFormat::Text {
                write_failure_message(
                    &compile_errors_message(&errors.0),
                    output,
                    CliExitCode::CompileFailed,
                );
            } else {
                for err in &errors.0 {
                    errln!("compile error: {err}");
                }
            }
            return CliExitCode::CompileFailed.into();
        }
    };

    let inputs = match map_runtime_inputs(&compiled, &input_data) {
        Ok(inputs) => inputs,
        Err(error) => {
            if output != OutputFormat::Text {
                write_failure_message(&error.to_string(), output, CliExitCode::RuntimeFailed);
            } else {
                errln!("{error}");
            }
            return CliExitCode::RuntimeFailed.into();
        }
    };

    match durability {
        DurabilityMode::None => {}
        _ => {
            if let Err(code) = store_workflow_artifacts(&compiled, &bytes, db, output) {
                return code;
            }
        }
    }

    run_compiled_workflow(&compiled, inputs, durability, db, output)
}

pub(crate) fn store_workflow_artifacts(
    compiled: &vb_core::CompiledWorkflow,
    source: &[u8],
    db: Option<&std::path::Path>,
    output: OutputFormat,
) -> Result<(), ExitCode> {
    let Some(db) = db else {
        return Ok(());
    };
    let parts = compiled.to_parts();
    let ir_bytes = match postcard::to_allocvec(&parts) {
        Ok(ir) => ir,
        Err(e) => {
            report_compiled_ir_store_error(format_args!("compiled IR encode error: {e}"), output);
            return Err(CliExitCode::StorageError.into());
        }
    };
    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(journal) => journal,
        Err(e) => {
            report_compiled_ir_store_error(
                format_args!("error opening journal at {}: {e}", db.display()),
                output,
            );
            return Err(CliExitCode::StorageError.into());
        }
    };
    let source_record = vb_storage::WorkflowSourceRecord {
        digest: vb_core::WorkflowDigest::from_bytes(blake3::hash(source).into()),
        source: source.to_vec(),
    };
    if let Err(e) = journal.put_workflow_source(&source_record) {
        report_compiled_ir_store_error(format_args!("workflow source write error: {e}"), output);
        return Err(CliExitCode::StorageError.into());
    }
    let proof = vb_storage::admission::VerificationProof::new(
        compiled.digest(),
        vb_runtime::admission::REQUIRED_GATE_COUNT,
        true,
    );
    let artifact = vb_storage::admission::AcceptedArtifact {
        digest: compiled.digest(),
        source_digest: compiled.digest(),
        policy_digest: vb_storage::admission::compute_policy_digest(compiled),
        ir: ir_bytes,
        verification: proof,
        accepted_at_seq: vb_storage::EventSeq::new(0),
        required_capabilities: Box::new([]),
    };
    let artifact_bytes = match postcard::to_allocvec(&artifact) {
        Ok(bytes) => bytes,
        Err(e) => {
            report_compiled_ir_store_error(format_args!("artifact encode error: {e}"), output);
            return Err(CliExitCode::StorageError.into());
        }
    };
    let record = vb_storage::CompiledIrRecord {
        digest: compiled.digest(),
        ir: artifact_bytes,
    };
    journal.put_compiled_ir(&record).map_err(|e| {
        report_compiled_ir_store_error(format_args!("compiled IR write error: {e}"), output);
        CliExitCode::StorageError.into()
    })
}

pub(crate) fn report_compiled_ir_store_error(args: std::fmt::Arguments<'_>, output: OutputFormat) {
    if output != OutputFormat::Text {
        write_failure_message(&args.to_string(), output, CliExitCode::StorageError);
    } else {
        errln!("{args}");
    }
}

pub(crate) fn cmd_submit(
