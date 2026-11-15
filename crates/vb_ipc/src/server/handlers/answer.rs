#![forbid(unsafe_code)]
//! Answer command handler and pending Ask selector resolution.

use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::{AskAnswer, AskTicket};

use super::{MAX_ANSWER_ASK_BYTES, decode_payload, sanitize_runtime_error};
use crate::server::IpcResponse;

/// Handles answer-ask.
pub fn handle_answer_ask(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let prepared = match prepare_answer(payload) {
        Ok(prepared) => prepared,
        Err(response) => return response,
    };
    let request = prepared.request;
    let Some(ticket) = resolve_answer_ticket(runtime, request.run_id, prepared.selector) else {
        return IpcResponse::BadRequest;
    };
    answer_response(
        runtime,
        request,
        ticket,
        prepared.value,
        prepared.encoded_len,
    )
}

struct PreparedAnswer {
    request: AnswerAskRequest,
    selector: AnswerTicketSelector,
    value: SlotValue,
    encoded_len: u32,
}

fn prepare_answer(payload: &[u8]) -> Result<PreparedAnswer, IpcResponse> {
    let Some(request) = decode_answer_request(payload) else {
        return Err(IpcResponse::BadRequest);
    };
    if let Some(response) = validate_answer_size(&request.answer) {
        return Err(response);
    }
    let Some(selector) = decode_answer_selector(request.ticket) else {
        return Err(IpcResponse::BadRequest);
    };
    let (encoded_len, value) = prepare_answer_content(&request.answer)?;
    Ok(PreparedAnswer {
        request,
        selector,
        value,
        encoded_len,
    })
}

fn prepare_answer_content(answer: &[u8]) -> Result<(u32, SlotValue), IpcResponse> {
    let encoded_len = answer_encoded_len(answer).ok_or_else(|| IpcResponse::RuntimeError {
        message: String::from("answer payload size exceeds u32::MAX"),
    })?;
    let value = decode_answer_value(answer)?;
    Ok((encoded_len, value))
}

struct AnswerAskRequest {
    run_id: RunId,
    ticket: u64,
    answer: Vec<u8>,
    taint: Option<Taint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnswerTicketSelector {
    Step(StepIdx),
    Slot(SlotIdx),
}

fn decode_answer_request(payload: &[u8]) -> Option<AnswerAskRequest> {
    match decode_payload::<crate::IpcPayload>(payload).ok()? {
        crate::IpcPayload::AnswerAsk {
            run_id,
            ticket,
            answer,
            taint,
        } => Some(AnswerAskRequest {
            run_id,
            ticket,
            answer,
            taint,
        }),
        _ => None,
    }
}

fn validate_answer_size(answer: &[u8]) -> Option<IpcResponse> {
    if answer.len() <= MAX_ANSWER_ASK_BYTES {
        return None;
    }
    Some(IpcResponse::PayloadError {
        diagnostic: crate::IpcError::PayloadDecodeFailed
            .diagnostic_code()
            .code(),
        message: String::from("answer payload exceeds maximum allowed size"),
    })
}

fn decode_answer_selector(ticket: u64) -> Option<AnswerTicketSelector> {
    match u16::try_from(ticket) {
        Ok(step) => Some(AnswerTicketSelector::Step(StepIdx::new(step))),
        Err(_) => decode_answer_slot_ticket(ticket).map(AnswerTicketSelector::Slot),
    }
}

fn decode_answer_slot_ticket(ticket: u64) -> Option<SlotIdx> {
    let slot = ticket.checked_sub(SLOT_TICKET_OFFSET)?;
    u16::try_from(slot).ok().map(SlotIdx::new)
}

fn answer_encoded_len(answer: &[u8]) -> Option<u32> {
    u32::try_from(answer.len()).ok()
}

fn decode_answer_value(answer: &[u8]) -> Result<SlotValue, IpcResponse> {
    postcard::from_bytes(answer).map_err(|_| IpcResponse::RuntimeError {
        message: String::from("answer bytes are not valid postcard-encoded SlotValue"),
    })
}

fn resolve_answer_ticket(
    runtime: &Runtime,
    run: RunId,
    selector: AnswerTicketSelector,
) -> Option<AskTicket> {
    match selector {
        AnswerTicketSelector::Step(step) => runtime.lookup_pending_ask_ticket(run, step),
        AnswerTicketSelector::Slot(slot) => lookup_pending_slot_ticket(runtime, run, slot),
    }
}

fn lookup_pending_slot_ticket(runtime: &Runtime, run: RunId, slot: SlotIdx) -> Option<AskTicket> {
    let state = match runtime.inspect_run_state(run) {
        Ok(state) => state,
        Err(_) => return None,
    };
    let mut tickets = (0..state.workflow.node_count())
        .filter_map(|step| runtime.lookup_pending_ask_ticket(run, StepIdx::new(step)))
        .filter(|ticket| ticket.answer_slot() == slot);
    let ticket = tickets.next()?;
    match tickets.next() {
        None => Some(ticket),
        Some(_) => None,
    }
}

fn answer_response(
    runtime: &mut Runtime,
    request: AnswerAskRequest,
    ticket: AskTicket,
    value: SlotValue,
    encoded_len: u32,
) -> IpcResponse {
    let answer = AskAnswer {
        answer_slot: ticket.answer_slot(),
        ticket,
        value,
        taint: request.taint.unwrap_or(Taint::Clean),
        encoded_len,
    };
    match runtime.answer_ask(answer) {
        Ok(()) => IpcResponse::AcceptedRun {
            run_id: request.run_id.get(),
        },
        Err(error) => IpcResponse::RuntimeError {
            message: sanitize_runtime_error(&error),
        },
    }
}

const SLOT_TICKET_OFFSET: u64 = 65_536;

#[cfg(test)]
#[path = "answer_tests.rs"]
mod tests;
