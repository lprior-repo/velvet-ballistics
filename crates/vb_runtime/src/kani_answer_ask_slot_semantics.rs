#![forbid(unsafe_code)]
//! Kani artifacts for vb-jpq7.21 AnswerAsk runtime slot semantics.

use vb_core::ids::{RunId, SlotIdx, StepIdx};

use crate::runtime::{AskTicketDerivation, kani_derive_ask_ticket_from_parts};
use crate::shard::PendingTimerKind;

fn generated_run() -> RunId {
    RunId::new(u64::from(kani::any::<u16>()).saturating_add(1))
}

fn generated_kind(case: u8) -> PendingTimerKind {
    if case == 0 {
        PendingTimerKind::Wait
    } else {
        PendingTimerKind::Ask
    }
}

fn generated_resume_answer(case: u8, answer: SlotIdx) -> Option<SlotIdx> {
    match case {
        2 | 3 => None,
        _ => Some(answer),
    }
}

#[kani::proof]
fn pending_ask_ticket_derivation_rejects_invalid_shard_states() {
    let case: u8 = kani::any();
    kani::assume(case <= 4);
    let run = generated_run();
    let ask_step = StepIdx::new(kani::any::<u16>());
    let resume_step = StepIdx::new(kani::any::<u16>());
    let answer = SlotIdx::new(kani::any::<u16>());
    let ask_next = if case == 1 { None } else { Some(resume_step) };
    let result = kani_derive_ask_ticket_from_parts(
        run,
        generated_kind(case),
        ask_step,
        ask_next,
        generated_resume_answer(case, answer),
        answer,
    );
    if case == 4 {
        kani::cover!(
            matches!(result, AskTicketDerivation::Ticket(_)),
            "cover: equal valid pending Ask accepted"
        );
        kani::assert(
            matches!(result, AskTicketDerivation::Ticket(_)),
            "valid Ask state derives ticket",
        );
        if let AskTicketDerivation::Ticket(ticket) = result {
            kani::assert(ticket.run == run, "ticket preserves run");
            kani::assert(ticket.ask_step == ask_step, "ticket preserves ask step");
            kani::assert(
                ticket.resume_step == resume_step,
                "ticket preserves resume step",
            );
        }
    } else {
        kani::cover!(
            case == 0 && result == AskTicketDerivation::InvalidActionCompletion,
            "cover: non-Ask pending kind rejected"
        );
        kani::cover!(
            case == 1 && result == AskTicketDerivation::InvalidActionCompletion,
            "cover: Ask without next rejected"
        );
        kani::cover!(
            case == 2 && result == AskTicketDerivation::InvalidActionCompletion,
            "cover: missing resume answer rejected"
        );
        kani::cover!(
            case == 3 && result == AskTicketDerivation::InvalidActionCompletion,
            "cover: non-AskResume resume node rejected"
        );
        kani::assert(
            result == AskTicketDerivation::InvalidActionCompletion,
            "invalid pending Ask derivation input rejected",
        );
    }
}

#[kani::proof]
fn answer_slot_equality_accepts_only_exact_ask_resume_slot() {
    let requested_raw: u16 = kani::any();
    let resume_answer_raw: u16 = kani::any();
    let requested = SlotIdx::new(requested_raw);
    let resume_answer = SlotIdx::new(resume_answer_raw);
    let result = kani_derive_ask_ticket_from_parts(
        generated_run(),
        PendingTimerKind::Ask,
        StepIdx::new(kani::any::<u16>()),
        Some(StepIdx::new(kani::any::<u16>())),
        Some(resume_answer),
        requested,
    );
    if requested_raw == resume_answer_raw {
        kani::cover!(
            matches!(result, AskTicketDerivation::Ticket(_)),
            "cover: equal slot accepted"
        );
        kani::cover!(
            requested_raw == u16::MAX && matches!(result, AskTicketDerivation::Ticket(_)),
            "cover: max slot boundary accepted when equal"
        );
        kani::assert(
            matches!(result, AskTicketDerivation::Ticket(_)),
            "equal answer_slot accepted",
        );
    } else {
        kani::cover!(
            requested_raw < resume_answer_raw
                && result == AskTicketDerivation::InvalidActionCompletion,
            "cover: lower mismatched slot rejected"
        );
        kani::cover!(
            requested_raw > resume_answer_raw
                && result == AskTicketDerivation::InvalidActionCompletion,
            "cover: upper mismatched slot rejected"
        );
        kani::cover!(
            requested_raw == u16::MAX && result == AskTicketDerivation::InvalidActionCompletion,
            "cover: max requested slot rejected when mismatched"
        );
        kani::assert(
            result == AskTicketDerivation::InvalidActionCompletion,
            "mismatched answer_slot rejected",
        );
    }
}
