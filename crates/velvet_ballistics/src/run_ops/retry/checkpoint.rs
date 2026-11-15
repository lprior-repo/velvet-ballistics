struct RetryCheckpointFields<'a> {
    run: vb_core::RunId,
    step: vb_core::StepIdx,
    digest: &'a [u8; 32],
    payload: &'a [u8],
}

pub(super) fn load_latest_retry_checkpoint(
    events: &[vb_storage::JournalEvent],
    run: vb_core::RunId,
    requested_step: u16,
) -> Result<vb_runtime::run_checkpoint::RunCheckpointV1, String> {
    let latest = events
        .iter()
        .rev()
        .find(|event| matches!(event, vb_storage::JournalEvent::RetryCheckpoint { .. }));
    let fields = retry_checkpoint_fields(latest)?;
    validate_checkpoint_run(fields.run, run)?;
    validate_checkpoint_step(fields.step, requested_step)?;
    decode_retry_checkpoint(fields.payload, fields.digest, run, requested_step)
}

fn retry_checkpoint_fields(
    event: Option<&vb_storage::JournalEvent>,
) -> Result<RetryCheckpointFields<'_>, String> {
    match event {
        Some(vb_storage::JournalEvent::RetryCheckpoint {
            run,
            step,
            payload_digest,
            payload,
            ..
        }) => Ok(RetryCheckpointFields {
            run: *run,
            step: *step,
            digest: payload_digest,
            payload,
        }),
        _ => Err("run has no durable retry checkpoint".to_string()),
    }
}

fn validate_checkpoint_run(
    observed: vb_core::RunId,
    expected: vb_core::RunId,
) -> Result<(), String> {
    if observed == expected {
        return Ok(());
    }
    Err(format!(
        "retry checkpoint belongs to run {}, expected {}",
        observed.get(),
        expected.get()
    ))
}

fn validate_checkpoint_step(observed: vb_core::StepIdx, expected: u16) -> Result<(), String> {
    if observed.get() == expected {
        return Ok(());
    }
    Err(format!(
        "retry checkpoint step {} does not match requested step {expected}",
        observed.get()
    ))
}

fn decode_retry_checkpoint(
    payload: &[u8],
    expected_digest: &[u8; 32],
    run: vb_core::RunId,
    requested_step: u16,
) -> Result<vb_runtime::run_checkpoint::RunCheckpointV1, String> {
    validate_checkpoint_digest(payload, expected_digest)?;
    let checkpoint = postcard::from_bytes(payload)
        .map_err(|error| format!("retry checkpoint decode failed: {error}"))?;
    validate_checkpoint_payload(&checkpoint, run, requested_step)?;
    Ok(checkpoint)
}

fn validate_checkpoint_digest(payload: &[u8], expected: &[u8; 32]) -> Result<(), String> {
    if blake3::hash(payload).as_bytes() == expected {
        return Ok(());
    }
    Err("retry checkpoint payload digest mismatch".to_string())
}

fn validate_checkpoint_payload(
    checkpoint: &vb_runtime::run_checkpoint::RunCheckpointV1,
    run: vb_core::RunId,
    requested_step: u16,
) -> Result<(), String> {
    if checkpoint.run_id != run {
        return Err(format!(
            "retry checkpoint payload belongs to run {}, expected {}",
            checkpoint.run_id.get(),
            run.get()
        ));
    }
    if checkpoint.retry_step.get() != requested_step {
        return Err(format!(
            "retry checkpoint payload step {} does not match requested step {requested_step}",
            checkpoint.retry_step.get()
        ));
    }
    Ok(())
}
