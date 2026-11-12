use crate::DurableActionOutcome;
use crate::recovery::types::{RecoveryError, RecoveryResult};
use vb_core::{ActionTicket, RunId};

pub(crate) fn verify_action_ticket_event(run: RunId, ticket: ActionTicket) -> RecoveryResult<()> {
    if ticket.run != run {
        return Err(RecoveryError::ReplayDivergence {
            step: ticket.step,
            detail: String::from("action ticket run mismatch"),
        });
    }
    if ticket.attempt == 0 || ticket.capacity == 0 || ticket.attempt > ticket.capacity {
        return Err(RecoveryError::ReplayDivergence {
            step: ticket.step,
            detail: String::from("action ticket attempt bounds invalid"),
        });
    }
    if !vb_core::action::action_ticket_has_valid_key(ticket) {
        return Err(RecoveryError::ReplayDivergence {
            step: ticket.step,
            detail: String::from("action ticket idempotency key mismatch"),
        });
    }
    Ok(())
}

pub(crate) fn verified_action_envelope_digest(
    run: RunId,
    ticket: ActionTicket,
    outcome: DurableActionOutcome,
    value: &[u8],
    encoded_len: u32,
    expected: [u8; 32],
) -> RecoveryResult<[u8; 32]> {
    verify_action_ticket_event(run, ticket)?;
    match outcome {
        DurableActionOutcome::Ready => {}
    }
    let actual_len = u32::try_from(value.len()).map_err(|_| RecoveryError::ReplayDivergence {
        step: ticket.step,
        detail: String::from("action completion value length exceeds u32"),
    })?;
    if actual_len != encoded_len {
        return Err(RecoveryError::ReplayDivergence {
            step: ticket.step,
            detail: String::from("action completion encoded length mismatch"),
        });
    }

    let found = *blake3::hash(value).as_bytes();
    if found == expected {
        Ok(expected)
    } else {
        Err(RecoveryError::ReplayDivergence {
            step: ticket.step,
            detail: String::from("action completion value digest mismatch"),
        })
    }
}

pub(crate) fn decode_action_envelope_slot(
    ticket: vb_core::ActionTicket,
    output: vb_core::SlotIdx,
    value: &[u8],
) -> RecoveryResult<vb_core::SlotValue> {
    postcard::from_bytes(value).map_err(|_| RecoveryError::ReplayDivergence {
        step: ticket.step,
        detail: format!("slot value decode failed for slot {:?}", output),
    })
}

pub(crate) fn sub_tail_parallel_in_flight(
    frame: &mut vb_core::RunFrame,
    step: vb_core::StepIdx,
) -> RecoveryResult<()> {
    if frame.parallel_in_flight() == 0 {
        return Ok(());
    }
    frame
        .sub_parallel_in_flight(1)
        .map_err(|_e| RecoveryError::ReplayDivergence {
            step,
            detail: "parallel_in_flight underflow".to_owned(),
        })
}
