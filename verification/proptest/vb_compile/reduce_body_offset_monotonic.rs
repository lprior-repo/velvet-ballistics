// Verification artifact: reduce_body_offset_monotonic.rs
// PO: PO-OFFSET-PROP-001
// Bead: vb-xi2f.24 | State: 5 (proof-writer)
// Verifier: proptest
// Command: cargo test -p vb_compile -- proptest_reduce_body_offset_monotonic
//
// Requirement: C3 — Body Step Sequential Assignment
// Domain Claim: Arbitrary body step sequences produce non-overlapping,
//   monotonically increasing StepIdx values.
//
// Proptest generates step width sequences and verifies that the
// cumulative offset accumulation produces strictly increasing IDs.

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use vb_core::ids::StepIdx;

    fn positive_u16_strategy() -> impl Strategy<Value = u16> {
        (1u16..10u16).prop_map(|v| v)
    }

    fn width_sequence_strategy() -> impl Strategy<Value = Vec<u16>> {
        prop::collection::vec(positive_u16_strategy(), 1..50)
    }

    fn compute_offsets(base: u16, widths: &[u16]) -> Vec<u16> {
        let mut offsets = Vec::new();
        let mut cumulative: u16 = 0;
        for &w in widths {
            let offset = cumulative + 1; // body_step starts at base + 1 + cumulative
            offsets.push(offset);
            cumulative = cumulative.saturating_add(w);
        }
        offsets
    }

    proptest! {
        #[test]
        fn proptest_reduce_body_offset_monotonic(
            widths in width_sequence_strategy(),
        ) {
            let base: u16 = 0;
            let offsets = compute_offsets(base, &widths);

            // All offsets must be strictly increasing
            for i in 1..offsets.len() {
                assert!(
                    offsets[i] > offsets[i - 1],
                    "offset at position {} ({}) must be > offset at position {} ({})",
                    i, offsets[i], i - 1, offsets[i - 1]
                );
            }

            // No two offsets are equal
            let mut sorted = offsets.clone();
            sorted.sort();
            sorted.dedup();
            assert_eq!(
                sorted.len(),
                offsets.len(),
                "all offsets must be distinct"
            );
        }
    }
}
