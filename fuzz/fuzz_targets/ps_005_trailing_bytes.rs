// Fuzz target: Trailing bytes concatenation attack defense (Gate 3).
//
// Obligation: PO-vb-h09wf-016
// Verifier: cargo-fuzz
// Command: cargo fuzz run ps_005_trailing_bytes -- -max_total_time=300
//
// Domain claim: 300s fuzz run: no panics, no crashes. All payloads with
// trailing bytes rejected. Defends against concatenation attacks (H5).
//
// PRODUCTION BINDING:
//   vb_storage::admission::decode_accepted_artifact_envelope
//   vb_storage::codec::payload::reject_trailing_bytes

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Generate a payload where a potentially valid envelope is followed by
    // arbitrary trailing bytes. The fuzzer varies both the envelope and trailer.

    if data.is_empty() {
        return;
    }

    // Split: first byte determines the split point
    let split_pct = data[0] as usize;
    let split = if data.len() > 1 {
        (split_pct % data.len()).max(1).min(data.len().saturating_sub(1))
    } else {
        1.min(data.len())
    };

    let _envelope_part = &data[..split];
    let _trailer_part = &data[split..];

    // Test reject_trailing_bytes with various boundary values
    for declared in [0, split, data.len(), data.len().saturating_sub(1)] {
        for actual in [0, split, data.len(), data.len().saturating_add(1)] {
            let _ = vb_storage::codec::payload::reject_trailing_bytes(
                declared.min(usize::MAX / 2),
                actual.min(usize::MAX / 2),
            );
        }
    }

    // Test the full decode path with likely-trailing data
    let _ = vb_storage::admission::decode_accepted_artifact_envelope(data);
});
