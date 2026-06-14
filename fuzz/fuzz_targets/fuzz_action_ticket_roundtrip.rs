//! Cargo-fuzz harness for ActionTicket 7-field wire format round-trip.
//!
//! Obligation: OBL-013 (serialization), OBL-NEW-PS-013
//! Verifier lane: cargo-fuzz
//!
/// This fuzz target tests that ActionTicket serializes and deserializes
/// correctly in postcard wire format. It covers the 7-field structure
/// (run, step, seq, action, attempt, idempotency_key, capacity).
///
/// NOTE: The 8-field format (with mock field) will be tested once
/// MockMarker is added to production code.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: Vec<u8>| {
    // Construct an ActionTicket from raw bytes, then serialize and
    // round-trip to verify the wire format.

    if data.len() >= 48 {
        let run = u64::from_le_bytes(data[0..8].try_into().unwrap());
        let step = u64::from_le_bytes(data[8..16].try_into().unwrap());
        let seq = u64::from_le_bytes(data[16..24].try_into().unwrap());
        let action = u64::from_le_bytes(data[24..32].try_into().unwrap());
        let attempt = u16::from_le_bytes(data[32..34].try_into().unwrap());
        let idempotency_key = u128::from_le_bytes(data[34..46].try_into().unwrap());
        let capacity = u16::from_le_bytes(data[46..48].try_into().unwrap());

        let ticket = vb_core::action::ActionTicket {
            run: vb_core::ids::RunId::new(run),
            step: vb_core::ids::StepIdx::new(step),
            seq: vb_core::ids::SeqNo::new(seq),
            action: vb_core::ids::ActionId::new(action),
            attempt,
            idempotency_key,
            capacity,
        };

        // Serialize to postcard (7-field wire format).
        let serialized = postcard::to_allocvec(&ticket).expect("ActionTicket serialization must succeed");

        // Deserialize back.
        let deserialized: vb_core::action::ActionTicket =
            postcard::from_bytes(&serialized).expect("ActionTicket deserialization must succeed");

        // Verify all 7 fields round-trip correctly.
        assert_eq!(deserialized.run.get(), ticket.run.get(), "run field mismatch");
        assert_eq!(deserialized.step.get(), ticket.step.get(), "step field mismatch");
        assert_eq!(deserialized.seq.get(), ticket.seq.get(), "seq field mismatch");
        assert_eq!(deserialized.action.get(), ticket.action.get(), "action field mismatch");
        assert_eq!(deserialized.attempt, ticket.attempt, "attempt field mismatch");
        assert_eq!(deserialized.idempotency_key, ticket.idempotency_key, "idempotency_key field mismatch");
        assert_eq!(deserialized.capacity, ticket.capacity, "capacity field mismatch");
    } else {
        // If input is too short, pad and try deserialize.
        // Goal: test that deserialization doesn't panic on malformed input.
        let mut padded = data.clone();
        while padded.len() < 49 {
            padded.push(0);
        }
        let _result: Result<vb_core::action::ActionTicket, _> = postcard::from_bytes(&padded);
    }
});
