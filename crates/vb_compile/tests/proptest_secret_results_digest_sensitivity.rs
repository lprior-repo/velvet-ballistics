#![allow(clippy::expect_used)]
// Verification artifact: proptest_secret_results_digest_sensitivity.rs
// PO: PO-P03
// Bead: vb-xi2f.35
// Verifier: proptest
// Command: cargo test proptest_secret_results_digest_sensitivity -- --nocapture
// Workdir: crates/vb_compile
//
// Proof obligation: allows_secret_results digest sensitivity at scale.
//
// GOD RULE: Calls actual production encode_contract_bytes.

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use vb_core::contract_encoding::encode_contract_bytes;
    use vb_core::workflow::ResourceContract;

    // PO-P03: Toggling allows_secret_results changes encoding at scale.
    proptest! {
        #[test]
        fn proptest_secret_results_digest_sensitivity(
            max_steps in 1u16..=200u16,
            max_slots in 1u16..=64u16,
        ) {
            let mut contract = ResourceContract::DEFAULT;
            contract.max_steps = max_steps;
            contract.max_slots = max_slots;

            let mut contract_true = contract;
            contract_true.allows_secret_results = true;

            let mut contract_false = contract;
            contract_false.allows_secret_results = false;

            let enc_true = encode_contract_bytes(&contract_true);
            let enc_false = encode_contract_bytes(&contract_false);
            prop_assert_ne!(enc_true, enc_false,
                "bool allows_secret_results toggle must produce different encodings");
        }
    }
}
