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
        let expected = digest(0xAA);
        let found = digest(0xBB);
        let result = check_action_abi_digest(action_id, expected, found);
        let Err(RecoveryError::ActionAbiMismatch {
            action_id: reported_action,
            expected: reported_expected,
            found: reported_found,
        }) = result
        else {
            panic!("should report ABI mismatch, got {result:?}");
        };
        assert_eq!(reported_action, action_id);
        assert_eq!(reported_expected, expected);
        assert_eq!(reported_found, found);
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
        let expected = digest(0xCC);
        let found = digest(0xDD);
        let result = check_policy_digest(step, expected, found);
        let Err(RecoveryError::PolicyDigestMismatch {
            step: reported_step,
            expected: reported_expected,
            found: reported_found,
        }) = result
        else {
            panic!("should report policy mismatch, got {result:?}");
        };
        assert_eq!(reported_step, step);
        assert_eq!(reported_expected, expected);
        assert_eq!(reported_found, found);
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
        let Err(RecoveryError::ActionAbiMismatch {
            action_id,
            expected,
            found,
        }) = result
        else {
            panic!("should report first mismatch, got {result:?}");
        };
        assert_eq!(action_id, ActionId::new(2));
        assert_eq!(expected, digest(0x22));
        assert_eq!(found, digest(0x33));
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
        let Err(RecoveryError::PolicyDigestMismatch {
            step,
            expected,
            found,
        }) = result
        else {
            panic!("should report first policy mismatch, got {result:?}");
        };
        assert_eq!(step, StepIdx::new(3));
        assert_eq!(expected, digest(0x77));
        assert_eq!(found, digest(0x88));
    }
}
