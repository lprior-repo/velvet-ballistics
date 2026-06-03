#![forbid(unsafe_code)]
#[cfg(test)]
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
mod recover_tests {
    use crate::recovery::{
        RecoveryError, check_action_abi_digest, check_action_abi_digests, check_compiled_ir_digest,
        check_policy_digest, check_policy_digests,
    };
    use vb_core::{ActionId, StepIdx, WorkflowDigest};

    fn digest(byte: u8) -> WorkflowDigest {
        WorkflowDigest::from_bytes([byte; 32])
    }

    #[test]
    fn check_compiled_ir_digest_accepts_match() {
        let d = digest(0x11);
        let result = check_compiled_ir_digest(d, d);
        assert!(
            result.is_ok(),
            "matching digest should succeed, got {:?}",
            result
        );
    }

    #[test]
    fn check_compiled_ir_digest_rejects_mismatch() {
        let expected = digest(0x11);
        let found = digest(0x22);
        let result = check_compiled_ir_digest(expected, found);
        assert!(
            matches!(result, Err(RecoveryError::CompiledIrDigestMismatch { expected: e, found: f })
                if e == expected && f == found),
            "should report mismatch, got {:?}",
            result
        );
    }

    #[test]
    fn check_action_abi_digest_accepts_match() {
        let d = digest(0x33);
        let result = check_action_abi_digest(ActionId::new(1), d, d);
        assert!(
            result.is_ok(),
            "matching ABI digest should succeed, got {:?}",
            result
        );
    }

    #[test]
    fn check_action_abi_digest_rejects_mismatch() {
        let action_id = ActionId::new(7);
        let result = check_action_abi_digest(action_id, digest(0xAA), digest(0xBB));
        assert!(
            matches!(result, Err(RecoveryError::ActionAbiMismatch { action_id: a, .. }) if a == action_id),
            "should report ABI mismatch, got {:?}",
            result
        );
    }

    #[test]
    fn check_policy_digest_accepts_match() {
        let d = digest(0x44);
        let result = check_policy_digest(StepIdx::new(0), d, d);
        assert!(
            result.is_ok(),
            "matching policy digest should succeed, got {:?}",
            result
        );
    }

    #[test]
    fn check_policy_digest_rejects_mismatch() {
        let step = StepIdx::new(5);
        let result = check_policy_digest(step, digest(0xCC), digest(0xDD));
        assert!(
            matches!(result, Err(RecoveryError::PolicyDigestMismatch { step: s, .. }) if s == step),
            "should report policy mismatch, got {:?}",
            result
        );
    }

    #[test]
    fn check_action_abi_digests_accepts_all_matching() {
        let entries = vec![
            (ActionId::new(1), digest(0x11), digest(0x11)),
            (ActionId::new(2), digest(0x22), digest(0x22)),
        ];
        let result = check_action_abi_digests(&entries);
        assert!(
            result.is_ok(),
            "all matching should succeed, got {:?}",
            result
        );
    }

    #[test]
    fn check_action_abi_digests_accepts_empty_entries() {
        let result = check_action_abi_digests(&[]);
        assert!(
            result.is_ok(),
            "empty entries should succeed, got {:?}",
            result
        );
    }

    #[test]
    fn check_action_abi_digests_rejects_first_mismatch() {
        let entries = vec![
            (ActionId::new(1), digest(0x11), digest(0x11)),
            (ActionId::new(2), digest(0x22), digest(0x33)),
            (ActionId::new(3), digest(0x44), digest(0x44)),
        ];
        let result = check_action_abi_digests(&entries);
        assert!(
            matches!(result, Err(RecoveryError::ActionAbiMismatch { action_id, .. }) if action_id == ActionId::new(2)),
            "should report first mismatch, got {:?}",
            result
        );
    }

    #[test]
    fn check_policy_digests_accepts_all_matching() {
        let entries = vec![
            (StepIdx::new(0), digest(0x55), digest(0x55)),
            (StepIdx::new(1), digest(0x66), digest(0x66)),
        ];
        let result = check_policy_digests(&entries);
        assert!(
            result.is_ok(),
            "all matching should succeed, got {:?}",
            result
        );
    }

    #[test]
    fn check_policy_digests_accepts_empty_entries() {
        let result = check_policy_digests(&[]);
        assert!(
            result.is_ok(),
            "empty entries should succeed, got {:?}",
            result
        );
    }

    #[test]
    fn check_policy_digests_rejects_first_mismatch() {
        let entries = vec![
            (StepIdx::new(0), digest(0x55), digest(0x55)),
            (StepIdx::new(3), digest(0x77), digest(0x88)),
        ];
        let result = check_policy_digests(&entries);
        assert!(
            matches!(result, Err(RecoveryError::PolicyDigestMismatch { step, .. }) if step == StepIdx::new(3)),
            "should report first policy mismatch, got {:?}",
            result
        );
    }
}
