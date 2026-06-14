#![no_main]

// Cargo-fuzz harness for ActionTicket wire-format round-trip.
//
// Obligation: OBL-013 (serialization), OBL-NEW-PS-013.
// Verifier lane: cargo-fuzz.

use libfuzzer_sys::fuzz_target;
use vb_core::{
    action::{ActionTicket, MockMarker},
    ids::{ActionId, RunId, SeqNo, StepIdx},
};

const RUN_OFFSET: usize = 0;
const STEP_OFFSET: usize = 8;
const SEQ_OFFSET: usize = 10;
const ACTION_OFFSET: usize = 18;
const ATTEMPT_OFFSET: usize = 20;
const KEY_OFFSET: usize = 22;
const CAPACITY_OFFSET: usize = 38;
const MOCK_OFFSET: usize = 40;

fuzz_target!(|data: &[u8]| {
    match ticket_from_bytes(data) {
        Some(ticket) => round_trip_ticket(ticket),
        None => exercise_decode(data),
    }
});

fn ticket_from_bytes(data: &[u8]) -> Option<ActionTicket> {
    Some(ActionTicket {
        run: RunId::new(u64::from_le_bytes(read_array::<8>(data, RUN_OFFSET)?)),
        step: StepIdx::new(u16::from_le_bytes(read_array::<2>(data, STEP_OFFSET)?)),
        seq: SeqNo::new(u64::from_le_bytes(read_array::<8>(data, SEQ_OFFSET)?)),
        action: ActionId::new(u16::from_le_bytes(read_array::<2>(data, ACTION_OFFSET)?)),
        attempt: u16::from_le_bytes(read_array::<2>(data, ATTEMPT_OFFSET)?),
        idempotency_key: u128::from_le_bytes(read_array::<16>(data, KEY_OFFSET)?),
        capacity: u16::from_le_bytes(read_array::<2>(data, CAPACITY_OFFSET)?),
        mock: mock_from_byte(*data.get(MOCK_OFFSET)?),
    })
}

fn read_array<const N: usize>(data: &[u8], offset: usize) -> Option<[u8; N]> {
    data.get(offset..offset.checked_add(N)?)?.try_into().ok()
}

fn mock_from_byte(byte: u8) -> MockMarker {
    match byte.checked_rem(3) {
        Some(0) => MockMarker::GithubIssueCreate,
        Some(1) => MockMarker::AiClassifyTicket,
        _ => MockMarker::HttpGet,
    }
}

fn round_trip_ticket(ticket: ActionTicket) {
    let Ok(serialized) = postcard::to_allocvec(&ticket) else {
        return;
    };
    let Ok(deserialized) = postcard::from_bytes::<ActionTicket>(&serialized) else {
        return;
    };

    assert_eq!(deserialized, ticket, "ActionTicket round-trip mismatch");
}

fn exercise_decode(data: &[u8]) {
    let result = postcard::from_bytes::<ActionTicket>(data);
    std::mem::drop(result);
}
