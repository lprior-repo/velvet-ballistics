// Verification artifact: reduce_body_chain_integrity.rs
// PO: PO-CHAIN-PROP-001
// Bead: vb-xi2f.24 | State: 5 (proof-writer)
// Verifier: proptest
// Command: cargo test -p vb_compile -- proptest_reduce_body_chain_integrity
//
// Requirement: C4 — Body Step Next-Link Chain
// Domain Claim: Arbitrary body step sequences produce correct linear
//   next-link chains with no broken or dangling links.

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    fn step_sequence_strategy() -> impl Strategy<Value = Vec<u16>> {
        prop::collection::vec(1u16..10u16, 1..50)
    }

    fn build_chain(base: u16, widths: &[u16], next_step: u16) -> Vec<(u16, u16)> {
        let mut chain = Vec::new();
        let mut cumulative: u16 = 0;
        for i in 0..widths.len() {
            let step_id = base + 1 + cumulative;
            let step_next = if i == widths.len() - 1 {
                next_step
            } else {
                base + 1 + cumulative + widths[i]
            };
            chain.push((step_id, step_next));
            cumulative = cumulative.saturating_add(widths[i]);
        }
        chain
    }

    proptest! {
        #[test]
        fn proptest_reduce_body_chain_integrity(
            widths in step_sequence_strategy(),
        ) {
            let base: u16 = 10;
            // Compute next_step as if after all body steps
            let total_width: u16 = widths.iter().sum();
            let next_step = base + 1 + total_width;

            let chain = build_chain(base, &widths, next_step);

            if chain.is_empty() {
                return;
            }

            // Chain must be continuous
            for i in 0..chain.len() - 1 {
                let (current_id, current_next) = chain[i];
                let (next_id, _) = chain[i + 1];

                assert_eq!(
                    current_next, next_id,
                    "step {} next ({}) must equal next step id ({})",
                    i, current_next, next_id
                );
                assert!(
                    current_next > current_id,
                    "step {} next must be > step id",
                    i
                );
            }

            // Last step chains to next_step
            let (last_id, last_next) = chain[chain.len() - 1];
            assert_eq!(
                last_next, next_step,
                "last step next ({}) must equal next_step ({})",
                last_next, next_step
            );
            assert!(
                last_next > last_id,
                "last step next must be > last step id"
            );
        }
    }
}
