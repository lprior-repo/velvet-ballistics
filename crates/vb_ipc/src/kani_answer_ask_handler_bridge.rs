#![forbid(unsafe_code)]
//! Kani artifact for vb-jpq7.21 AnswerAsk IPC scalar decode and runtime bridge.

use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_runtime::runtime::{AskTicketDerivation, kani_derive_ask_ticket_from_parts};
use vb_runtime::shard::PendingTimerKind;

use crate::server::IpcResponse;
use crate::server::handlers::{
    AnswerAskFieldDecision, AnswerAskSlotValueDecode, decide_answer_ask_fields,
    decode_answer_ask_fields_with_slot_value_decoder,
};

#[kani::proof]
fn answer_ask_handler_rejects_malformed_and_routes_valid_bounded_inputs() {
    let case: u8 = kani::any();
    kani::assume(case <= 3);
    let run = RunId::new(u64::from(kani::any::<u16>()).saturating_add(100));
    let resume_answer = SlotIdx::new(kani::any::<u16>());
    let wrong = SlotIdx::new(resume_answer.get().wrapping_add(1));
    let answer_slot = if case == 2 { wrong } else { resume_answer };
    let decoded_value = if case == 0 {
        AnswerAskSlotValueDecode::Invalid
    } else if kani::any::<bool>() {
        AnswerAskSlotValueDecode::Decoded(SlotValue::Bool(kani::any()))
    } else {
        AnswerAskSlotValueDecode::Decoded(SlotValue::I64(i64::from(kani::any::<i16>())))
    };
    let answer_len = if case == 3 { 65_537_usize } else { 2_usize };
    let taint = if case == 1 { None } else { Some(Taint::Secret) };

    let decision = decide_answer_ask_fields(run, answer_slot, answer_len, decoded_value, taint);
    if case == 0 {
        kani::cover!(
            decision == AnswerAskFieldDecision::InvalidAnswerBytes,
            "cover: invalid answer bytes rejected"
        );
        kani::assert(
            decision == AnswerAskFieldDecision::InvalidAnswerBytes,
            "malformed SlotValue decode rejected",
        );
        return;
    }
    if case == 3 {
        kani::cover!(
            decision == AnswerAskFieldDecision::OversizeAnswer,
            "cover: oversize answer rejected"
        );
        kani::assert(
            decision == AnswerAskFieldDecision::OversizeAnswer,
            "oversize answer rejected",
        );
        return;
    }

    let AnswerAskFieldDecision::Decoded(decoded) = decision else {
        kani::assert(false, "valid scalar fields decode");
        return;
    };
    if case == 1 {
        kani::cover!(
            decoded.taint == Taint::Clean,
            "cover: missing taint defaults Clean"
        );
        kani::assert(
            decoded.taint == Taint::Clean,
            "missing taint defaults Clean",
        );
    } else {
        kani::cover!(
            decoded.taint == Taint::Secret,
            "cover: explicit taint preserved"
        );
        kani::assert(
            decoded.taint == Taint::Secret,
            "explicit taint preserved by decode helper",
        );
    }
    kani::assert(
        decoded.encoded_len == 2,
        "bounded answer length converted to u32",
    );

    let routed = kani_derive_ask_ticket_from_parts(
        decoded.run_id,
        PendingTimerKind::Ask,
        StepIdx::new(1),
        Some(StepIdx::new(2)),
        Some(resume_answer),
        decoded.answer_slot,
    );
    if case == 1 {
        kani::cover!(
            matches!(routed, AskTicketDerivation::Ticket(_)),
            "cover: valid decoded fields route"
        );
        kani::assert(
            matches!(routed, AskTicketDerivation::Ticket(_)),
            "valid decoded fields route to Ask helper",
        );
    } else {
        kani::cover!(
            routed == AskTicketDerivation::InvalidActionCompletion,
            "cover: mismatched decoded slot rejected"
        );
        kani::assert(
            routed == AskTicketDerivation::InvalidActionCompletion,
            "mismatched decoded answer_slot rejected",
        );
    }
}

#[kani::proof]
fn answer_ask_boundary_order_rejects_oversize_before_slot_value_decoder() {
    const OVERSIZE_ANSWER: usize = 65_537;

    let run = RunId::new(u64::from(kani::any::<u16>()).saturating_add(100));
    let answer_slot = SlotIdx::new(kani::any::<u16>());
    let answer = [0_u8; OVERSIZE_ANSWER];
    let taint = if kani::any::<bool>() {
        Some(Taint::Secret)
    } else {
        None
    };
    let mut decoder_invoked = false;

    let result = decode_answer_ask_fields_with_slot_value_decoder(
        run,
        answer_slot,
        &answer,
        taint,
        |_bytes| {
            decoder_invoked = true;
            AnswerAskSlotValueDecode::Invalid
        },
    );

    kani::cover!(
        !decoder_invoked,
        "cover: oversize answer bypasses SlotValue decoder"
    );
    kani::assert(
        !decoder_invoked,
        "oversize answer rejected before SlotValue decoder invocation",
    );

    match &result {
        Err(IpcResponse::PayloadError { diagnostic, .. }) => {
            kani::cover!(
                *diagnostic
                    == crate::IpcError::PayloadDecodeFailed
                        .diagnostic_code()
                        .code(),
                "cover: oversize answer returns PayloadDecodeFailed payload error"
            );
            kani::assert(
                *diagnostic
                    == crate::IpcError::PayloadDecodeFailed
                        .diagnostic_code()
                        .code(),
                "oversize answer returns PayloadDecodeFailed diagnostic",
            );
        }
        Err(_) => {
            kani::assert(false, "oversize answer returns payload error");
        }
        Ok(_) => {
            kani::assert(false, "oversize answer returns IPC error response");
        }
    }
    core::mem::forget(result);
}
