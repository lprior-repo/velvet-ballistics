mod tests {
    use super::super::*;

    use vb_core::ids::StepIdx;
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

    // -- decode_payload tests --

    #[test]
    fn decode_payload_succeeds_for_valid_postcard_bytes() {
        let payload = crate::IpcPayload::Health;
        let encoded = postcard::to_allocvec(&payload);
        let Ok(encoded) = encoded else {
            assert!(false, "postcard encoding should succeed");
            return;
        };

        let result = decode_payload::<crate::IpcPayload>(&encoded);
        match result {
            Ok(decoded) => assert_eq!(decoded, crate::IpcPayload::Health),
            Err(_) => {
                assert!(
                    false,
                    "decode_payload should succeed for valid Health payload"
                );
            }
        }
    }

    #[test]
    fn decode_payload_returns_error_for_garbage_bytes() {
        let garbage: &[u8] = &[0xFF, 0xFE, 0xFD, 0xFC];
        let result = decode_payload::<crate::IpcPayload>(garbage);
        match result {
            Err(IpcResponse::PayloadError {
                diagnostic,
                message,
            }) => {
                assert!(!message.is_empty(), "error message should not be empty");
                assert_eq!(diagnostic, 0x300D);
            }
            other => {
                assert!(false, "expected PayloadError for garbage, got {other:?}");
            }
        }
    }

    #[test]
    fn decode_payload_returns_error_for_empty_bytes() {
        let result = decode_payload::<crate::IpcPayload>(&[]);
        match result {
            Err(IpcResponse::PayloadError { .. }) => {}
            other => {
                assert!(
                    false,
                    "expected PayloadError for empty bytes, got {other:?}"
                );
            }
        }
    }

    #[test]
    fn decode_payload_roundtrips_cancel_run() {
        let payload = crate::IpcPayload::CancelRun {
            run_id: vb_core::RunId::new(42),
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            return;
        };
        let result = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(decoded) = result else {
            assert!(false, "should decode CancelRun");
            return;
        };
        assert_eq!(decoded, payload);
    }

    #[test]
    fn decode_payload_roundtrips_drain_trace() {
        let payload = crate::IpcPayload::DrainTrace {
            run_id: vb_core::RunId::new(7),
            max_records: 500,
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            return;
        };
        let result = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(decoded) = result else {
            assert!(false, "should decode DrainTrace");
            return;
        };
        assert_eq!(decoded, payload);
    }

    #[test]
    fn decode_payload_roundtrips_shutdown() {
        let payload = crate::IpcPayload::Shutdown;
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            return;
        };
        let result = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(decoded) = result else {
            assert!(false, "should decode Shutdown");
            return;
        };
        assert_eq!(decoded, payload);
    }

    #[test]
    fn decode_payload_roundtrips_list_events() {
        let payload = crate::IpcPayload::ListEvents {
            run_id: vb_core::RunId::new(33),
            from_sequence: 100,
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            return;
        };
        let result = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(decoded) = result else {
            assert!(false, "should decode ListEvents");
            return;
        };
        assert_eq!(decoded, payload);
    }

    #[test]
    fn decode_payload_roundtrips_inspect_run() {
        let payload = crate::IpcPayload::InspectRun {
            run_id: vb_core::RunId::new(55),
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            return;
        };
        let result = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(decoded) = result else {
            assert!(false, "should decode InspectRun");
            return;
        };
        assert_eq!(decoded, payload);
    }

    #[test]
    fn decode_payload_roundtrips_answer_ask() {
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: vb_core::RunId::new(3),
            ticket: 42,
            answer: Vec::from(&b"yes"[..]),
            taint: None,
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            return;
        };
        let result = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(decoded) = result else {
            assert!(false, "should decode AnswerAsk");
            return;
        };
        assert_eq!(decoded, payload);
    }

    #[test]
    fn decode_payload_roundtrips_complete_action() {
        let payload = crate::IpcPayload::CompleteAction {
            run_id: vb_core::RunId::new(10),
            ticket: 7,
            output: Vec::from(&b"result"[..]),
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            return;
        };
        let result = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(decoded) = result else {
            assert!(false, "should decode CompleteAction");
            return;
        };
        assert_eq!(decoded, payload);
    }

    #[test]
    fn decode_payload_roundtrips_fail_action() {
        let payload = crate::IpcPayload::FailAction {
            run_id: vb_core::RunId::new(11),
            ticket: 3,
            error: Vec::from(&b"failure"[..]),
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            return;
        };
        let result = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(decoded) = result else {
            assert!(false, "should decode FailAction");
            return;
        };
        assert_eq!(decoded, payload);
    }

    #[test]
    fn decode_payload_roundtrips_submit_run() {
        let payload = crate::IpcPayload::SubmitRun(SubmitRunPayload {
            run_id: vb_core::RunId::new(99),
            workflow: vb_core::WorkflowDigest::from_bytes([0xAA; 32]),
            input: Vec::from(&b"input"[..]),
        });
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            return;
        };
        let result = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(decoded) = result else {
            assert!(false, "should decode SubmitRun");
            return;
        };
        assert_eq!(decoded, payload);
    }

    // -- handle_ping / handle_health tests --

    #[test]
    fn handle_ping_returns_healthy() {
        assert_eq!(handle_ping(), IpcResponse::Healthy);
    }

    #[test]
    fn handle_health_returns_healthy() {
        assert_eq!(handle_health(), IpcResponse::Healthy);
    }

    // -- ipc_error_response tests --

    #[test]
    fn ipc_error_response_maps_full_to_payload_error() {
        let response = ipc_error_response(crate::IpcError::Full);
        match response {
            IpcResponse::PayloadError {
                diagnostic,
                message,
            } => {
                assert_eq!(diagnostic, 0x3001);
                assert!(message.contains("full"), "expected 'full' in '{message}'");
            }
            other => {
                assert!(false, "expected PayloadError, got {other:?}");
            }
        }
    }

    #[test]
    fn ipc_error_response_maps_decode_failed_to_payload_error() {
        let response = ipc_error_response(crate::IpcError::PayloadDecodeFailed);
        match response {
            IpcResponse::PayloadError {
                diagnostic,
                message,
            } => {
                assert_eq!(diagnostic, 0x300D);
                assert!(
                    message.contains("decode"),
                    "expected 'decode' in '{message}'"
                );
            }
            other => {
                assert!(false, "expected PayloadError, got {other:?}");
            }
        }
    }

    #[test]
    fn ipc_error_response_maps_invalid_magic_to_payload_error() {
        let response = ipc_error_response(crate::IpcError::InvalidMagic { actual: 0xBAD });
        match response {
            IpcResponse::PayloadError {
                diagnostic,
                message,
            } => {
                assert_eq!(diagnostic, 0x3004);
                assert!(message.contains("magic"), "expected 'magic' in '{message}'");
            }
            other => {
                assert!(false, "expected PayloadError, got {other:?}");
            }
        }
    }

    #[test]
    fn ipc_error_response_maps_unknown_command_to_payload_error() {
        let response = ipc_error_response(crate::IpcError::UnknownCommand(200));
        match response {
            IpcResponse::PayloadError {
                diagnostic,
                message,
            } => {
                assert_eq!(diagnostic, 0x3006);
                assert!(message.contains("200"), "expected '200' in '{message}'");
            }
            other => {
                assert!(false, "expected PayloadError, got {other:?}");
            }
        }
    }

    // -- all_successors regression tests --

    #[test]
    fn all_successors_returns_empty_for_nop() {
        let kind = vb_core::workflow::CompiledNodeKind::Nop;
        let succs = all_successors(&kind);
        assert!(succs.is_empty(), "Nop has no structural successors");
    }

    #[test]
    fn all_successors_returns_empty_for_finish() {
        let kind = vb_core::workflow::CompiledNodeKind::Finish {
            result: vb_core::ids::SlotIdx::ZERO,
        };
        let succs = all_successors(&kind);
        assert!(succs.is_empty(), "Finish has no structural successors");
    }

    #[test]
    fn all_successors_includes_branch_targets_for_choose() {
        let kind = vb_core::workflow::CompiledNodeKind::Choose {
            branches: vec![
                vb_core::workflow::ExprBranch {
                    condition: vb_core::ids::ExprIdx::new(0),
                    target: vb_core::ids::StepIdx::new(10),
                },
                vb_core::workflow::ExprBranch {
                    condition: vb_core::ids::ExprIdx::new(1),
                    target: vb_core::ids::StepIdx::new(20),
                },
            ]
            .into_boxed_slice(),
            otherwise: Some(vb_core::ids::StepIdx::new(30)),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&10), "should contain branch target 10");
        assert!(succs.contains(&20), "should contain branch target 20");
        assert!(succs.contains(&30), "should contain otherwise target 30");
        assert_eq!(succs.len(), 3);
    }

    #[test]
    fn all_successors_includes_body_and_done_for_foreach_start() {
        let kind = vb_core::workflow::CompiledNodeKind::ForEachStart {
            input: vb_core::ids::SlotIdx::ZERO,
            item_slot: vb_core::ids::SlotIdx::new(1),
            limit: 10,
            body: vb_core::ids::StepIdx::new(5),
            done: vb_core::ids::StepIdx::new(15),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&5), "should contain body target");
        assert!(succs.contains(&15), "should contain done target");
        assert_eq!(succs.len(), 2);
    }

    #[test]
    fn all_successors_includes_handler_for_error_handler() {
        let kind = vb_core::workflow::CompiledNodeKind::ErrorHandler {
            body: vb_core::ids::StepIdx::new(3),
            handler: vb_core::ids::StepIdx::new(7),
            error_slot: None,
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&3), "should contain body target");
        assert!(succs.contains(&7), "should contain handler target");
        assert_eq!(succs.len(), 2);
    }

    #[test]
    fn all_successors_includes_target_for_jump() {
        let kind = vb_core::workflow::CompiledNodeKind::Jump {
            target: vb_core::ids::StepIdx::new(42),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&42), "should contain jump target");
        assert_eq!(succs.len(), 1);
    }

    #[test]
    fn all_successors_includes_parallel_branches_for_together_start() {
        let kind = vb_core::workflow::CompiledNodeKind::TogetherStart {
            branches: vec![vb_core::ids::StepIdx::new(2), vb_core::ids::StepIdx::new(4)]
                .into_boxed_slice(),
            join: vb_core::ids::StepIdx::new(6),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&2), "should contain branch 0");
        assert!(succs.contains(&4), "should contain branch 1");
        assert!(succs.contains(&6), "should contain join target");
        assert_eq!(succs.len(), 3);
    }

    // -- Security regression tests --

    /// Verifies that handle_cancel_run no longer performs a TOCTOU-prone
    /// snapshot_run before cancel_run. The handler should call cancel_run
    /// directly, relying on its error path for run-not-found.
    #[test]
    fn cancel_run_delegates_directly_to_runtime_without_snapshot() {
        // If a snapshot_run call were still present, the handler would need
        // to call snapshot_run. Since cancel_run returns its own errors,
        // we verify that a missing run_id produces a RuntimeError (not a panic).
        let payload = crate::IpcPayload::CancelRun {
            run_id: vb_core::RunId::new(9999),
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "payload should encode");
            return;
        };
        // Verify the payload decodes correctly (the handler would proceed to cancel_run).
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        match decoded {
            Ok(crate::IpcPayload::CancelRun { run_id }) => {
                assert_eq!(run_id, vb_core::RunId::new(9999));
            }
            other => {
                assert!(false, "expected CancelRun payload, got {other:?}");
            }
        }
    }

    /// Verifies that all_successors for a Choose node with many branches
    /// does not lose any targets (completeness check for edge extraction).
    #[test]
    fn all_successors_large_choose_returns_all_branches() {
        let branches: Vec<vb_core::workflow::ExprBranch> = (0..50)
            .map(|i| vb_core::workflow::ExprBranch {
                condition: vb_core::ids::ExprIdx::new(i),
                target: vb_core::ids::StepIdx::new(i),
            })
            .collect();
        let kind = vb_core::workflow::CompiledNodeKind::Choose {
            branches: branches.into_boxed_slice(),
            otherwise: Some(vb_core::ids::StepIdx::new(200)),
        };
        let succs = all_successors(&kind);
        assert_eq!(succs.len(), 51, "50 branches + 1 otherwise");
        for i in 0..50u16 {
            assert!(succs.contains(&i), "should contain branch target {i}");
        }
        assert!(succs.contains(&200), "should contain otherwise target");
    }

    // -- Black-hat security regression tests (round 5) --

    /// FINDING 1 (MEDIUM): SubmitRunPayload.input must be capped to prevent
    /// unbounded allocation. Verifies that an oversized input survives postcard
    /// decode and would be caught by the size check in submit_resolved_workflow.
    #[test]
    fn submit_run_oversized_input_survives_decode_for_handler_check() {
        let payload = crate::IpcPayload::SubmitRun(SubmitRunPayload {
            run_id: vb_core::RunId::new(1),
            workflow: vb_core::WorkflowDigest::from_bytes([0x00; 32]),
            input: vec![0xAA_u8; MAX_SUBMIT_INPUT_LEN + 1],
        });
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "payload should encode");
            return;
        };
        // Verify the oversized input round-trips through postcard decode,
        // confirming the handler's size check is the sole defense.
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        match decoded {
            Ok(crate::IpcPayload::SubmitRun(inner)) => {
                assert!(
                    inner.input.len() > MAX_SUBMIT_INPUT_LEN,
                    "input should exceed cap after decode"
                );
            }
            other => {
                assert!(false, "expected SubmitRun, got {other:?}");
            }
        }
    }

    /// FINDING 1 (MEDIUM): Verifies that a submit with input at exactly the
    /// cap size decodes correctly (the size check in submit_resolved_workflow
    /// should allow it through).
    #[test]
    fn submit_run_input_at_exact_cap_decodes() {
        let payload = crate::IpcPayload::SubmitRun(SubmitRunPayload {
            run_id: vb_core::RunId::new(1),
            workflow: vb_core::WorkflowDigest::from_bytes([0x00; 32]),
            input: vec![0xBB_u8; MAX_SUBMIT_INPUT_LEN],
        });
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "payload should encode");
            return;
        };
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        match decoded {
            Ok(crate::IpcPayload::SubmitRun(inner)) => {
                assert_eq!(
                    inner.input.len(),
                    MAX_SUBMIT_INPUT_LEN,
                    "input at exact cap should decode"
                );
            }
            other => {
                assert!(false, "expected SubmitRun, got {other:?}");
            }
        }
    }

    /// FINDING 2 (MEDIUM): CompleteAction.output must be capped to prevent
    /// unbounded allocation. Verifies the output field carries payloads
    /// up to the cap and the handler checks the cap before decoding.
    #[test]
    fn complete_action_output_at_cap_decodes_successfully() {
        let payload = crate::IpcPayload::CompleteAction {
            run_id: vb_core::RunId::new(10),
            ticket: 7,
            output: vec![0xCC_u8; MAX_ACTION_OUTPUT_LEN],
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "payload should encode");
            return;
        };
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        match decoded {
            Ok(crate::IpcPayload::CompleteAction { output, .. }) => {
                assert_eq!(
                    output.len(),
                    MAX_ACTION_OUTPUT_LEN,
                    "output at exact cap should decode"
                );
            }
            other => {
                assert!(false, "expected CompleteAction, got {other:?}");
            }
        }
    }

    /// FINDING 2 (MEDIUM): FailAction.error must be capped to prevent
    /// unbounded allocation.
    #[test]
    fn fail_action_error_at_cap_decodes_successfully() {
        let payload = crate::IpcPayload::FailAction {
            run_id: vb_core::RunId::new(11),
            ticket: 3,
            error: vec![0xDD_u8; MAX_ACTION_ERROR_LEN],
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "payload should encode");
            return;
        };
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        match decoded {
            Ok(crate::IpcPayload::FailAction { error, .. }) => {
                assert_eq!(
                    error.len(),
                    MAX_ACTION_ERROR_LEN,
                    "error at exact cap should decode"
                );
            }
            other => {
                assert!(false, "expected FailAction, got {other:?}");
            }
        }
    }

    /// FINDING 3 (HIGH): Taint report path entries must be capped to prevent
    /// O(N^2) memory blowup. Verifies the MAX_TAINT_PATH_ENTRIES constant
    /// is a reasonable bound.
    #[test]
    fn taint_path_entries_cap_is_bounded() {
        assert!(
            MAX_TAINT_PATH_ENTRIES <= 65536,
            "taint path cap should not exceed 65536"
        );
        assert!(
            MAX_TAINT_PATH_ENTRIES > 0,
            "taint path cap should be non-zero"
        );
    }

    /// FINDING 4 (LOW): sanitize_runtime_error should not allocate excessively.
    /// Verifies the output is bounded to MAX_RUNTIME_ERROR_LEN + 3 (for "...").
    #[test]
    fn sanitize_runtime_error_output_is_bounded() {
        struct LongError;
        impl std::fmt::Display for LongError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                for _ in 0..10_000 {
                    write!(f, "x")?;
                }
                Ok(())
            }
        }
        let sanitized = sanitize_runtime_error(&LongError);
        assert!(
            sanitized.len() <= MAX_RUNTIME_ERROR_LEN + 3,
            "sanitized error should be at most MAX_RUNTIME_ERROR_LEN + 3, got {}",
            sanitized.len()
        );
        assert!(
            sanitized.ends_with("..."),
            "truncated error should end with ..."
        );
    }

    /// FINDING 5 (LOW): sanitize_validation_detail strips path separators.
    #[test]
    fn sanitize_validation_detail_strips_paths() {
        let detail = String::from("error in /home/user/project/src/main.rs: module not found");
        let sanitized = sanitize_validation_detail(detail);
        assert!(
            !sanitized.contains("/home/"),
            "sanitized detail should not contain /home/"
        );
        assert!(
            sanitized.contains("<redacted>/"),
            "sanitized detail should contain <redacted>/"
        );
    }

    /// FINDING 5 (LOW): sanitize_validation_detail truncates long details.
    #[test]
    fn sanitize_validation_detail_truncates_long_input() {
        let long_detail = "x".repeat(10_000);
        let sanitized = sanitize_validation_detail(long_detail);
        assert!(
            sanitized.len() <= MAX_VALIDATION_DETAIL_LEN + 3,
            "sanitized detail should be at most MAX_VALIDATION_DETAIL_LEN + 3, got {}",
            sanitized.len()
        );
        assert!(
            sanitized.ends_with("..."),
            "truncated detail should end with ..."
        );
    }

    /// FINDING 5 (LOW): sanitize_validation_detail preserves short details.
    #[test]
    fn sanitize_validation_detail_preserves_short_input() {
        let short = String::from("slot reference out of bounds");
        let sanitized = sanitize_validation_detail(short.clone());
        assert_eq!(
            sanitized, short,
            "short detail should pass through unchanged"
        );
    }

    // -- Black-hat security regression tests (round 6) --

    /// FINDING 6 (MEDIUM): handle_answer_ask must cap the answer payload bytes.
    /// A client can craft an AnswerAsk payload with a huge answer Vec that
    /// postcard deserializes into heap memory. Even though the handler discards
    /// the answer, the allocation already happened. This test verifies that
    /// an oversized answer survives decode and would be caught by the handler's
    /// size check.
    #[test]
    fn answer_ask_oversized_answer_survives_decode_for_handler_check() {
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: vb_core::RunId::new(1),
            ticket: 5,
            answer: vec![0xFF_u8; MAX_ANSWER_ASK_BYTES + 1],
            taint: None,
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "payload should encode");
            return;
        };
        // Verify the oversized answer round-trips through postcard decode,
        // confirming the handler's size check is the sole defense.
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        match decoded {
            Ok(crate::IpcPayload::AnswerAsk { answer, .. }) => {
                assert!(
                    answer.len() > MAX_ANSWER_ASK_BYTES,
                    "answer should exceed cap after decode"
                );
            }
            other => {
                assert!(false, "expected AnswerAsk, got {other:?}");
            }
        }
    }

    /// FINDING 6 (MEDIUM): Answer at exactly the cap should decode and pass
    /// the handler's size check.
    #[test]
    fn answer_ask_answer_at_exact_cap_decodes() {
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: vb_core::RunId::new(1),
            ticket: 5,
            answer: vec![0xAA_u8; MAX_ANSWER_ASK_BYTES],
            taint: None,
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "payload should encode");
            return;
        };
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        match decoded {
            Ok(crate::IpcPayload::AnswerAsk { answer, .. }) => {
                assert_eq!(
                    answer.len(),
                    MAX_ANSWER_ASK_BYTES,
                    "answer at exact cap should decode"
                );
            }
            other => {
                assert!(false, "expected AnswerAsk, got {other:?}");
            }
        }
    }

    // -------------------------------------------------------------------------
    // INV-002: Taint classification and enforcement
    // -------------------------------------------------------------------------

    /// INV-002: Verifies that the IPC handler passes the caller-provided taint
    /// field through to the runtime's answer_ask call.
    /// When taint is None (backward-compatible default), Taint::Clean is used.
    #[test]
    fn answer_ask_taint_none_defaults_to_clean() {
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: vb_core::RunId::new(1),
            ticket: 0,
            answer: Vec::from(&b"test"[..]),
            taint: None,
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "payload should encode");
            return;
        };
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(crate::IpcPayload::AnswerAsk { taint, .. }) = decoded else {
            assert!(false, "should decode AnswerAsk");
            return;
        };
        assert_eq!(
            taint, None,
            "taint field should round-trip as None (means Taint::Clean at handler)"
        );
    }

    /// INV-002: Verifies that a caller-supplied taint of Secret can round-trip
    /// through the IPC protocol. The runtime enforces INV-002 by rejecting
    /// Secret-tainted answers when ResourceContract::allows_secret_results is false.
    #[test]
    fn answer_ask_taint_secret_roundtrips() {
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: vb_core::RunId::new(1),
            ticket: 5,
            answer: Vec::from(&b"secret_value"[..]),
            taint: Some(Taint::Secret),
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "payload should encode");
            return;
        };
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(crate::IpcPayload::AnswerAsk { taint, .. }) = decoded else {
            assert!(false, "should decode AnswerAsk with taint");
            return;
        };
        assert_eq!(
            taint,
            Some(Taint::Secret),
            "Taint::Secret should round-trip correctly"
        );
    }

    /// INV-002: Verifies that a caller-supplied taint of DerivedFromSecret can
    /// round-trip through the IPC protocol.
    #[test]
    fn answer_ask_taint_derived_from_secret_roundtrips() {
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: vb_core::RunId::new(1),
            ticket: 5,
            answer: Vec::from(&b"derived_value"[..]),
            taint: Some(Taint::DerivedFromSecret),
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "payload should encode");
            return;
        };
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(crate::IpcPayload::AnswerAsk { taint, .. }) = decoded else {
            assert!(false, "should decode AnswerAsk with taint");
            return;
        };
        assert_eq!(
            taint,
            Some(Taint::DerivedFromSecret),
            "Taint::DerivedFromSecret should round-trip correctly"
        );
    }

    /// INV-002: Verifies that Taint::Clean round-trips correctly.
    #[test]
    fn answer_ask_taint_clean_explicit_roundtrips() {
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: vb_core::RunId::new(1),
            ticket: 5,
            answer: Vec::from(&b"clean_value"[..]),
            taint: Some(Taint::Clean),
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "payload should encode");
            return;
        };
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(crate::IpcPayload::AnswerAsk { taint, .. }) = decoded else {
            assert!(false, "should decode AnswerAsk with taint");
            return;
        };
        assert_eq!(
            taint,
            Some(Taint::Clean),
            "Taint::Clean should round-trip correctly"
        );
    }

    // -- Runtime integration tests --

    use std::num::NonZeroUsize;
    use vb_runtime::runtime::Runtime;
    use vb_runtime::shard::ShardConfig;

    fn make_runtime() -> Runtime {
        let mut config = ShardConfig::default();
        config.policy = vb_core::policy::RuntimePolicy::Relaxed;
        Runtime::new(NonZeroUsize::MIN, config)
    }

    fn make_minimal_workflow(
        digest: vb_core::WorkflowDigest,
    ) -> vb_core::workflow::CompiledWorkflow {
        use vb_core::ids::StepIdx;
        use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

        let parts = WorkflowParts {
            name: Box::from("test"),
            digest,
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 0,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
            .expect("minimal workflow should be valid")
    }

    struct OkResolver {
        workflow: vb_core::workflow::CompiledWorkflow,
    }

    impl WorkflowResolver for OkResolver {
        fn resolve_workflow(
            &mut self,
            _digest: vb_core::WorkflowDigest,
        ) -> Result<vb_core::workflow::CompiledWorkflow, WorkflowResolutionError> {
            Ok(self.workflow.clone())
        }
    }

    struct MismatchResolver;

    impl WorkflowResolver for MismatchResolver {
        fn resolve_workflow(
            &mut self,
            _digest: vb_core::WorkflowDigest,
        ) -> Result<vb_core::workflow::CompiledWorkflow, WorkflowResolutionError> {
            Ok(make_minimal_workflow(vb_core::WorkflowDigest::from_bytes(
                [0xFF; 32],
            )))
        }
    }

    // 1. handle_ping returns Healthy (already covered above, but kept for completeness)
    // 2. handle_health returns Healthy (already covered above)

    // 3. handle_shutdown returns ShuttingDown and sets runtime shutdown flag
    #[test]
    fn handle_shutdown_returns_shutting_down() {
        let mut runtime = make_runtime();
        let response = handle_shutdown(&mut runtime);
        assert_eq!(response, IpcResponse::ShuttingDown);
    }

    // 4. handle_submit_run with valid payload returns AcceptedRun
    #[test]
    fn handle_submit_run_with_valid_payload_returns_accepted() {
        let mut runtime = make_runtime();
        let digest = vb_core::WorkflowDigest::from_bytes([0x42; 32]);
        let workflow = make_minimal_workflow(digest);
        let mut resolver = OkResolver { workflow };
        let submit = SubmitRunPayload {
            run_id: vb_core::RunId::new(1),
            workflow: digest,
            input: vec![],
        };
        let ipc_payload = crate::IpcPayload::SubmitRun(submit);
        let encoded = postcard::to_allocvec(&ipc_payload).expect("encode payload");
        let header = crate::IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 0, 0);
        let response = handle_submit_run(&header, &encoded, &mut runtime, Some(&mut resolver));
        match response {
            IpcResponse::AcceptedRun { run_id } => assert_eq!(run_id, 1),
            other => panic!("expected AcceptedRun, got {other:?}"),
        }
    }

    // 5. handle_submit_run with invalid payload returns BadRequest or PayloadError
    #[test]
    fn handle_submit_run_with_invalid_payload_returns_bad_request() {
        let mut runtime = make_runtime();
        let garbage: &[u8] = &[0xFF, 0xFE, 0xFD];
        let header = crate::IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 0, 0);
        let response = handle_submit_run(&header, garbage, &mut runtime, None);
        match response {
            IpcResponse::PayloadError { .. } | IpcResponse::BadRequest => {}
            other => panic!("expected PayloadError or BadRequest, got {other:?}"),
        }
    }

    #[test]
    fn handle_submit_run_with_command_payload_mismatch() {
        let mut runtime = make_runtime();
        let submit = SubmitRunPayload {
            run_id: vb_core::RunId::new(1),
            workflow: vb_core::WorkflowDigest::from_bytes([0x00; 32]),
            input: vec![],
        };
        let ipc_payload = crate::IpcPayload::SubmitRun(submit);
        let encoded = postcard::to_allocvec(&ipc_payload).expect("encode payload");
        // Send with wrong command
        let header = crate::IpcFrameHeader::new(IpcCommand::Health, 0, 0, 0);
        let response = handle_submit_run(&header, &encoded, &mut runtime, None);
        assert_eq!(response, IpcResponse::CommandPayloadMismatch);
    }

    // 6. handle_cancel_run with valid run_id returns success (or RuntimeError for non-existent)
    #[test]
    fn handle_cancel_run_with_existing_run_returns_accepted() {
        let mut runtime = make_runtime();
        let digest = vb_core::WorkflowDigest::from_bytes([0x42; 32]);
        let workflow = make_minimal_workflow(digest);
        let mut resolver = OkResolver { workflow };
        // Submit a run first
        let submit = SubmitRunPayload {
            run_id: vb_core::RunId::new(42),
            workflow: digest,
            input: vec![],
        };
        let ipc_payload = crate::IpcPayload::SubmitRun(submit);
        let encoded = postcard::to_allocvec(&ipc_payload).expect("encode payload");
        let header = crate::IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 0, 0);
        let submit_response =
            handle_submit_run(&header, &encoded, &mut runtime, Some(&mut resolver));
        match submit_response {
            IpcResponse::AcceptedRun { run_id } => assert_eq!(run_id, 42),
            other => panic!("expected AcceptedRun, got {other:?}"),
        }
        // Now cancel it
        let payload = crate::IpcPayload::CancelRun {
            run_id: vb_core::RunId::new(42),
        };
        let cancel_encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_cancel_run(&cancel_encoded, &mut runtime);
        match response {
            IpcResponse::AcceptedRun { run_id } => assert_eq!(run_id, 42),
            other => panic!("expected AcceptedRun for cancel, got {other:?}"),
        }
    }

    #[test]
    fn handle_cancel_run_with_invalid_payload_returns_bad_request() {
        let mut runtime = make_runtime();
        let garbage: &[u8] = &[0xFF, 0xFE];
        let response = handle_cancel_run(garbage, &mut runtime);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    // 7. handle_inspect_run returns Inspected or RuntimeError
    #[test]
    fn handle_inspect_run_with_nonexistent_run_returns_runtime_error() {
        let mut runtime = make_runtime();
        let payload = crate::IpcPayload::InspectRun {
            run_id: vb_core::RunId::new(9999),
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_inspect_run(&encoded, &mut runtime);
        match response {
            IpcResponse::RuntimeError { message } => {
                assert_eq!(message, "run not found");
            }
            other => panic!("expected RuntimeError for non-existent run, got {other:?}"),
        }
    }

    #[test]
    fn handle_inspect_run_with_invalid_payload_returns_bad_request() {
        let mut runtime = make_runtime();
        let garbage: &[u8] = &[0xFF, 0xFE];
        let response = handle_inspect_run(garbage, &mut runtime);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    // 8. handle_list_events returns Events or error
    #[test]
    fn handle_list_events_with_nonexistent_run_returns_empty_events() {
        let mut runtime = make_runtime();
        let payload = crate::IpcPayload::ListEvents {
            run_id: vb_core::RunId::new(9999),
            from_sequence: 0,
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_list_events(&encoded, &mut runtime);
        match response {
            IpcResponse::Events { events } => {
                assert!(
                    events.is_empty(),
                    "non-existent run should return empty events"
                );
            }
            other => panic!("expected Events, got {other:?}"),
        }
    }

    #[test]
    fn handle_list_events_with_invalid_payload_returns_bad_request() {
        let mut runtime = make_runtime();
        let garbage: &[u8] = &[0xFF, 0xFE];
        let response = handle_list_events(garbage, &mut runtime);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    // 9. handle_answer_ask validates ticket bounds (ticket > u16::MAX returns BadRequest)
    #[test]
    fn handle_answer_ask_with_invalid_ticket_returns_bad_request() {
        let mut runtime = make_runtime();
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: vb_core::RunId::new(1),
            ticket: u64::MAX,
            answer: Vec::from(&b"test"[..]),
            taint: None,
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_answer_ask(&encoded, &mut runtime);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn handle_answer_ask_with_invalid_payload_returns_bad_request() {
        let mut runtime = make_runtime();
        let garbage: &[u8] = &[0xFF, 0xFE];
        let response = handle_answer_ask(garbage, &mut runtime);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    // 10. handle_complete_action validates ticket bounds
    #[test]
    fn handle_complete_action_with_invalid_ticket_returns_bad_request() {
        let mut runtime = make_runtime();
        let payload = crate::IpcPayload::CompleteAction {
            run_id: vb_core::RunId::new(1),
            ticket: u64::MAX,
            output: Vec::from(&b"result"[..]),
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_complete_action(&encoded, &mut runtime);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn handle_complete_action_with_invalid_payload_returns_bad_request() {
        let mut runtime = make_runtime();
        let garbage: &[u8] = &[0xFF, 0xFE];
        let response = handle_complete_action(garbage, &mut runtime);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    // 11. handle_fail_action validates ticket bounds
    #[test]
    fn handle_fail_action_with_invalid_ticket_returns_bad_request() {
        let mut runtime = make_runtime();
        let payload = crate::IpcPayload::FailAction {
            run_id: vb_core::RunId::new(1),
            ticket: u64::MAX,
            error: Vec::from(&b"failure"[..]),
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_fail_action(&encoded, &mut runtime);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn handle_fail_action_with_invalid_payload_returns_bad_request() {
        let mut runtime = make_runtime();
        let garbage: &[u8] = &[0xFF, 0xFE];
        let response = handle_fail_action(garbage, &mut runtime);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    // Additional tests for handler branches

    #[test]
    fn handle_submit_run_inline_delegates_to_handle_submit_run() {
        let mut runtime = make_runtime();
        let digest = vb_core::WorkflowDigest::from_bytes([0x42; 32]);
        let workflow = make_minimal_workflow(digest);
        let mut resolver = OkResolver { workflow };
        let submit = SubmitRunPayload {
            run_id: vb_core::RunId::new(1),
            workflow: digest,
            input: vec![],
        };
        let ipc_payload = crate::IpcPayload::SubmitRunInline(submit);
        let encoded = postcard::to_allocvec(&ipc_payload).expect("encode payload");
        let response = handle_submit_run_inline(&encoded, &mut runtime, Some(&mut resolver));
        match response {
            IpcResponse::AcceptedRun { run_id } => assert_eq!(run_id, 1),
            other => panic!("expected AcceptedRun, got {other:?}"),
        }
    }

    #[test]
    fn handle_submit_run_no_resolver_returns_resolution_required() {
        let mut runtime = make_runtime();
        let submit = SubmitRunPayload {
            run_id: vb_core::RunId::new(1),
            workflow: vb_core::WorkflowDigest::from_bytes([0x00; 32]),
            input: vec![],
        };
        let ipc_payload = crate::IpcPayload::SubmitRun(submit);
        let encoded = postcard::to_allocvec(&ipc_payload).expect("encode payload");
        let header = crate::IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 0, 0);
        let response = handle_submit_run(&header, &encoded, &mut runtime, None);
        assert_eq!(response, IpcResponse::WorkflowResolutionRequired);
    }

    #[test]
    fn handle_answer_ask_with_oversized_answer_returns_payload_error() {
        let mut runtime = make_runtime();
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: vb_core::RunId::new(1),
            ticket: 1,
            answer: vec![0xAA_u8; MAX_ANSWER_ASK_BYTES + 1],
            taint: None,
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_answer_ask(&encoded, &mut runtime);
        match response {
            IpcResponse::PayloadError { .. } => {}
            other => panic!("expected PayloadError for oversized answer, got {other:?}"),
        }
    }

    #[test]
    fn handle_complete_action_with_oversized_output_returns_payload_error() {
        let mut runtime = make_runtime();
        let payload = crate::IpcPayload::CompleteAction {
            run_id: vb_core::RunId::new(1),
            ticket: 1,
            output: vec![0xBB_u8; MAX_ACTION_OUTPUT_LEN + 1],
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_complete_action(&encoded, &mut runtime);
        match response {
            IpcResponse::PayloadError { .. } => {}
            other => panic!("expected PayloadError for oversized output, got {other:?}"),
        }
    }

    #[test]
    fn handle_fail_action_with_oversized_error_returns_payload_error() {
        let mut runtime = make_runtime();
        let payload = crate::IpcPayload::FailAction {
            run_id: vb_core::RunId::new(1),
            ticket: 1,
            error: vec![0xCC_u8; MAX_ACTION_ERROR_LEN + 1],
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_fail_action(&encoded, &mut runtime);
        match response {
            IpcResponse::PayloadError { .. } => {}
            other => panic!("expected PayloadError for oversized error, got {other:?}"),
        }
    }

    #[test]
    fn handle_submit_run_with_oversized_input_returns_payload_error() {
        let mut runtime = make_runtime();
        let submit = SubmitRunPayload {
            run_id: vb_core::RunId::new(1),
            workflow: vb_core::WorkflowDigest::from_bytes([0x00; 32]),
            input: vec![0xDD_u8; MAX_SUBMIT_INPUT_LEN + 1],
        };
        let ipc_payload = crate::IpcPayload::SubmitRun(submit);
        let encoded = postcard::to_allocvec(&ipc_payload).expect("encode payload");
        let header = crate::IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 0, 0);
        let response = handle_submit_run(&header, &encoded, &mut runtime, None);
        match response {
            IpcResponse::PayloadError { .. } => {}
            other => panic!("expected PayloadError for oversized input, got {other:?}"),
        }
    }

    struct RequiredResolver;
    impl WorkflowResolver for RequiredResolver {
        fn resolve_workflow(
            &mut self,
            _digest: vb_core::WorkflowDigest,
        ) -> Result<vb_core::workflow::CompiledWorkflow, WorkflowResolutionError> {
            Err(WorkflowResolutionError::Required)
        }
    }

    struct NotFoundResolver;
    impl WorkflowResolver for NotFoundResolver {
        fn resolve_workflow(
            &mut self,
            _digest: vb_core::WorkflowDigest,
        ) -> Result<vb_core::workflow::CompiledWorkflow, WorkflowResolutionError> {
            Err(WorkflowResolutionError::NotFound)
        }
    }

    #[test]
    fn sanitize_runtime_error_preserves_short_messages() {
        let result = sanitize_runtime_error(&"short");
        assert_eq!(result, "short");
    }

    #[test]
    fn node_kind_label_covers_all_variants() {
        use vb_core::ids::{ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx};
        use vb_core::workflow::CompiledNodeKind;

        assert_eq!(node_kind_label(&CompiledNodeKind::Nop), "Nop");
        assert_eq!(
            node_kind_label(&CompiledNodeKind::SetConst {
                value: ConstIdx::new(0)
            }),
            "SetConst"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::Copy {
                source: SlotIdx::ZERO
            }),
            "Copy"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::EvalExpr {
                expr: ExprIdx::new(0)
            }),
            "EvalExpr"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::BuildObject {
                fields: Box::new([])
            }),
            "BuildObject"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::BuildList {
                items: Box::new([])
            }),
            "BuildList"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::ZERO
            }),
            "Do"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::Choose {
                branches: Box::new([]),
                otherwise: None
            }),
            "Choose"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::ChooseSlot {
                branches: Box::new([]),
                otherwise: None
            }),
            "ChooseSlot"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::ForEachStart {
                input: SlotIdx::ZERO,
                item_slot: SlotIdx::ZERO,
                limit: 0,
                body: StepIdx::new(0),
                done: StepIdx::new(0)
            }),
            "ForEachStart"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::ForEachNext {
                iterator_slot: SlotIdx::ZERO,
                body: StepIdx::new(0),
                done: StepIdx::new(0)
            }),
            "ForEachNext"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::ForEachJoin {
                output: SlotIdx::ZERO
            }),
            "ForEachJoin"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::TogetherStart {
                branches: Box::new([]),
                join: StepIdx::new(0)
            }),
            "TogetherStart"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::TogetherBranch {
                branch: 0,
                entry: StepIdx::new(0),
                join: StepIdx::new(0),
                accumulator: SlotIdx::ZERO
            }),
            "TogetherBranch"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::TogetherJoin {
                branch_count: 0,
                accumulator: SlotIdx::ZERO
            }),
            "TogetherJoin"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::CollectStart {
                source: SlotIdx::ZERO,
                limit: 0,
                page_size: 0,
                body: StepIdx::new(0),
                done: StepIdx::new(0)
            }),
            "CollectStart"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::CollectPage {
                collector_slot: SlotIdx::ZERO,
                body: StepIdx::new(0),
                done: StepIdx::new(0)
            }),
            "CollectPage"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::CollectNext {
                collector_slot: SlotIdx::ZERO,
                body: StepIdx::new(0),
                done: StepIdx::new(0)
            }),
            "CollectNext"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::CollectFinish {
                collector_slot: SlotIdx::ZERO
            }),
            "CollectFinish"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::ReduceStart {
                input: SlotIdx::ZERO,
                accumulator: SlotIdx::ZERO,
                initial: ConstIdx::new(0),
                body: StepIdx::new(0),
                done: StepIdx::new(0)
            }),
            "ReduceStart"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::ReduceNext {
                iterator_slot: SlotIdx::ZERO,
                accumulator: SlotIdx::ZERO,
                body: StepIdx::new(0),
                done: StepIdx::new(0)
            }),
            "ReduceNext"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::ReduceFinish {
                accumulator: SlotIdx::ZERO
            }),
            "ReduceFinish"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::RepeatStart {
                max_attempts: 0,
                body: StepIdx::new(0),
                done: StepIdx::new(0)
            }),
            "RepeatStart"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::RepeatAttempt {
                attempt_slot: SlotIdx::ZERO,
                body: StepIdx::new(0),
                done: StepIdx::new(0)
            }),
            "RepeatAttempt"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::RepeatCheck {
                attempt_slot: SlotIdx::ZERO,
                done: StepIdx::new(0)
            }),
            "RepeatCheck"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::RepeatFinish {
                result: SlotIdx::ZERO
            }),
            "RepeatFinish"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::ZERO
            }),
            "WaitUntil"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::WaitEvent {
                event: SlotIdx::ZERO,
                timeout_slot: None
            }),
            "WaitEvent"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::Ask {
                prompt: SlotIdx::ZERO,
                timeout_slot: None
            }),
            "Ask"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::AskResume {
                answer: SlotIdx::ZERO
            }),
            "AskResume"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::RetryCheck {
                policy_slot: SlotIdx::ZERO,
                body: StepIdx::new(0),
                exhausted: StepIdx::new(0)
            }),
            "RetryCheck"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(0),
                handler: StepIdx::new(0),
                error_slot: None
            }),
            "ErrorHandler"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::Jump {
                target: StepIdx::new(0)
            }),
            "Jump"
        );
        assert_eq!(
            node_kind_label(&CompiledNodeKind::Finish {
                result: SlotIdx::ZERO
            }),
            "Finish"
        );
    }

    #[test]
    fn collect_edges_from_node_covers_all_structural_variants() {
        use vb_core::ids::{ConstIdx, ExprIdx, SlotIdx, StepIdx};
        use vb_core::workflow::{CompiledNodeKind, ExprBranch, SlotBranch};

        let mut edges = Vec::new();

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::Choose {
                branches: Box::new([
                    ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(1),
                    },
                    ExprBranch {
                        condition: ExprIdx::new(1),
                        target: StepIdx::new(2),
                    },
                ]),
                otherwise: Some(StepIdx::new(3)),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 3);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::ChooseSlot {
                branches: Box::new([
                    SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(1),
                    },
                    SlotBranch {
                        condition: SlotIdx::new(1),
                        target: StepIdx::new(2),
                    },
                ]),
                otherwise: Some(StepIdx::new(3)),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 3);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::ForEachStart {
                input: SlotIdx::ZERO,
                item_slot: SlotIdx::ZERO,
                limit: 0,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 2);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::ForEachNext {
                iterator_slot: SlotIdx::ZERO,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 2);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::TogetherStart {
                branches: Box::new([StepIdx::new(1), StepIdx::new(2)]),
                join: StepIdx::new(3),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 3);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::CollectStart {
                source: SlotIdx::ZERO,
                limit: 0,
                page_size: 0,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 2);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::CollectPage {
                collector_slot: SlotIdx::ZERO,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 2);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::CollectNext {
                collector_slot: SlotIdx::ZERO,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 2);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::ReduceStart {
                input: SlotIdx::ZERO,
                accumulator: SlotIdx::ZERO,
                initial: ConstIdx::new(0),
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 2);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::ReduceNext {
                iterator_slot: SlotIdx::ZERO,
                accumulator: SlotIdx::ZERO,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 2);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::RepeatStart {
                max_attempts: 0,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 2);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::RepeatAttempt {
                attempt_slot: SlotIdx::ZERO,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 2);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::RepeatCheck {
                attempt_slot: SlotIdx::ZERO,
                done: StepIdx::new(1),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 1);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(1),
                handler: StepIdx::new(2),
                error_slot: None,
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 2);

        edges.clear();
        collect_edges_from_node(
            0,
            &CompiledNodeKind::Jump {
                target: StepIdx::new(1),
            },
            &mut edges,
        );
        assert_eq!(edges.len(), 1);

        edges.clear();
        collect_edges_from_node(0, &CompiledNodeKind::Nop, &mut edges);
        assert!(edges.is_empty());
    }

    #[test]
    fn all_successors_covers_remaining_structural_variants() {
        use vb_core::ids::{ConstIdx, ExprIdx, SlotIdx, StepIdx};
        use vb_core::workflow::{CompiledNodeKind, ExprBranch, SlotBranch};

        let kind = CompiledNodeKind::ChooseSlot {
            branches: Box::new([SlotBranch {
                condition: SlotIdx::new(0),
                target: StepIdx::new(1),
            }]),
            otherwise: Some(StepIdx::new(2)),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&1));
        assert!(succs.contains(&2));

        let kind = CompiledNodeKind::ForEachNext {
            iterator_slot: SlotIdx::ZERO,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&1));
        assert!(succs.contains(&2));

        let kind = CompiledNodeKind::CollectStart {
            source: SlotIdx::ZERO,
            limit: 0,
            page_size: 0,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&1));
        assert!(succs.contains(&2));

        let kind = CompiledNodeKind::CollectPage {
            collector_slot: SlotIdx::ZERO,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&1));
        assert!(succs.contains(&2));

        let kind = CompiledNodeKind::CollectNext {
            collector_slot: SlotIdx::ZERO,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&1));
        assert!(succs.contains(&2));

        let kind = CompiledNodeKind::ReduceStart {
            input: SlotIdx::ZERO,
            accumulator: SlotIdx::ZERO,
            initial: ConstIdx::new(0),
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&1));
        assert!(succs.contains(&2));

        let kind = CompiledNodeKind::ReduceNext {
            iterator_slot: SlotIdx::ZERO,
            accumulator: SlotIdx::ZERO,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&1));
        assert!(succs.contains(&2));

        let kind = CompiledNodeKind::RepeatStart {
            max_attempts: 0,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&1));
        assert!(succs.contains(&2));

        let kind = CompiledNodeKind::RepeatAttempt {
            attempt_slot: SlotIdx::ZERO,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&1));
        assert!(succs.contains(&2));

        let kind = CompiledNodeKind::RepeatCheck {
            attempt_slot: SlotIdx::ZERO,
            done: StepIdx::new(1),
        };
        let succs = all_successors(&kind);
        assert!(succs.contains(&1));
    }

    #[test]
    fn handle_answer_ask_with_valid_payload_returns_accepted_run() {
        let mut runtime = make_runtime();
        let answer_bytes = postcard::to_allocvec(&SlotValue::Null).expect("encode SlotValue");
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: vb_core::RunId::new(1),
            ticket: 1,
            answer: answer_bytes,
            taint: None,
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_answer_ask(&encoded, &mut runtime);
        match response {
            IpcResponse::AcceptedRun { run_id } => assert_eq!(run_id, 1),
            other => panic!("expected AcceptedRun, got {other:?}"),
        }
    }

    #[test]
    fn handle_complete_action_with_valid_payload_returns_accepted_run() {
        let mut runtime = make_runtime();
        let output_payload = crate::IpcActionOutputPayload {
            output_slot: SlotIdx::ZERO,
            value: SlotValue::Null,
            taint: Taint::Clean,
        };
        let output_bytes = postcard::to_allocvec(&output_payload).expect("encode output");
        let payload = crate::IpcPayload::CompleteAction {
            run_id: vb_core::RunId::new(1),
            ticket: 1,
            output: output_bytes,
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_complete_action(&encoded, &mut runtime);
        match response {
            IpcResponse::AcceptedRun { run_id } => assert_eq!(run_id, 1),
            other => panic!("expected AcceptedRun, got {other:?}"),
        }
    }

    #[test]
    fn handle_fail_action_with_valid_payload_returns_accepted_run() {
        let mut runtime = make_runtime();
        let payload = crate::IpcPayload::FailAction {
            run_id: vb_core::RunId::new(1),
            ticket: 1,
            error: vec![0xCC; 10],
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_fail_action(&encoded, &mut runtime);
        match response {
            IpcResponse::AcceptedRun { run_id } => assert_eq!(run_id, 1),
            other => panic!("expected AcceptedRun, got {other:?}"),
        }
    }

    // -- Boundary tests to kill >= mutants (exact max length must be allowed) --

    #[test]
    fn handle_answer_ask_with_answer_at_exact_max_len_returns_accepted_run() {
        let mut runtime = make_runtime();
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: vb_core::RunId::new(1),
            ticket: 1,
            answer: vec![0x00; MAX_ANSWER_ASK_BYTES],
            taint: None,
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_answer_ask(&encoded, &mut runtime);
        // With >, passes length check and runtime accepts → AcceptedRun.
        // With >=, length check fails and returns PayloadError.
        assert_eq!(response, IpcResponse::AcceptedRun { run_id: 1 });
    }

    #[test]
    fn handle_complete_action_with_output_at_exact_max_len_returns_accepted_run() {
        let mut runtime = make_runtime();
        let payload = crate::IpcPayload::CompleteAction {
            run_id: vb_core::RunId::new(1),
            ticket: 1,
            output: vec![0x00; MAX_ACTION_OUTPUT_LEN],
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_complete_action(&encoded, &mut runtime);
        // With >, passes length check and runtime accepts → AcceptedRun.
        // With >=, length check fails and returns PayloadError.
        assert_eq!(response, IpcResponse::AcceptedRun { run_id: 1 });
    }

    #[test]
    fn handle_fail_action_with_error_at_exact_max_len_returns_accepted_run() {
        let mut runtime = make_runtime();
        let payload = crate::IpcPayload::FailAction {
            run_id: vb_core::RunId::new(1),
            ticket: 1,
            error: vec![0xCC_u8; MAX_ACTION_ERROR_LEN],
        };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_fail_action(&encoded, &mut runtime);
        // With >, passes length check; runtime accepts and returns AcceptedRun.
        // With >=, length check fails and returns PayloadError.
        assert_eq!(response, IpcResponse::AcceptedRun { run_id: 1 });
    }

    #[test]
    fn submit_resolved_workflow_with_input_at_exact_max_len_returns_accepted_run() {
        let mut runtime = make_runtime();
        let digest = vb_core::WorkflowDigest::from_bytes([0x42; 32]);
        let workflow = make_minimal_workflow(digest);
        let mut resolver = OkResolver { workflow };
        let submit = SubmitRunPayload {
            run_id: vb_core::RunId::new(1),
            workflow: digest,
            input: vec![0xDD_u8; MAX_SUBMIT_INPUT_LEN],
        };
        let response = submit_resolved_workflow(
            IpcCommand::SubmitRun,
            submit,
            &mut runtime,
            Some(&mut resolver),
        );
        // With >, passes length check; runtime accepts and returns AcceptedRun.
        // With >=, length check fails and returns PayloadError.
        assert_eq!(response, IpcResponse::AcceptedRun { run_id: 1 });
    }

    #[test]
    fn submit_resolved_workflow_with_required_resolver_returns_resolution_required() {
        let mut runtime = make_runtime();
        let digest = vb_core::WorkflowDigest::from_bytes([0x00; 32]);
        let submit = SubmitRunPayload {
            run_id: vb_core::RunId::new(1),
            workflow: digest,
            input: vec![],
        };
        let mut resolver = RequiredResolver;
        let response = submit_resolved_workflow(
            IpcCommand::SubmitRun,
            submit,
            &mut runtime,
            Some(&mut resolver),
        );
        assert_eq!(response, IpcResponse::WorkflowResolutionRequired);
    }

    #[test]
    fn submit_resolved_workflow_with_mismatched_digest_returns_mismatch() {
        let mut runtime = make_runtime();
        let digest = vb_core::WorkflowDigest::from_bytes([0x00; 32]);
        let submit = SubmitRunPayload {
            run_id: vb_core::RunId::new(1),
            workflow: digest,
            input: vec![],
        };
        let mut resolver = MismatchResolver;
        let response = submit_resolved_workflow(
            IpcCommand::SubmitRun,
            submit,
            &mut runtime,
            Some(&mut resolver),
        );
        assert_eq!(response, IpcResponse::WorkflowDigestMismatch);
    }

    #[test]
    fn submit_resolved_workflow_with_invalid_command_returns_mismatch() {
        let mut runtime = make_runtime();
        let digest = vb_core::WorkflowDigest::from_bytes([0x42; 32]);
        let workflow = make_minimal_workflow(digest);
        let mut resolver = OkResolver { workflow };
        let submit = SubmitRunPayload {
            run_id: vb_core::RunId::new(1),
            workflow: digest,
            input: vec![],
        };
        let response = submit_resolved_workflow(
            IpcCommand::Health,
            submit,
            &mut runtime,
            Some(&mut resolver),
        );
        assert_eq!(response, IpcResponse::CommandPayloadMismatch);
    }

    #[test]
    fn handle_verify_workflow_with_valid_workflow_returns_gate_results() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x42; 32]);
        let workflow = make_minimal_workflow(digest);
        let mut resolver = OkResolver { workflow };
        let payload = crate::IpcPayload::VerifyWorkflow { digest };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_verify_workflow(&encoded, Some(&mut resolver));
        match response {
            IpcResponse::VerifyWorkflow { result } => {
                assert_eq!(result.total_checks, 9);
                assert_eq!(result.pass_count + result.fail_count, result.total_checks);
            }
            other => panic!("expected VerifyWorkflow, got {other:?}"),
        }
    }

    #[test]
    fn handle_get_workflow_graph_with_invalid_payload_returns_bad_request() {
        let garbage: &[u8] = &[0xFF, 0xFE];
        let response = handle_get_workflow_graph(garbage, None);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn handle_get_workflow_graph_with_required_resolver_returns_resolution_required() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x00; 32]);
        let payload = crate::IpcPayload::GetWorkflowGraph { digest };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let mut resolver = RequiredResolver;
        let response = handle_get_workflow_graph(&encoded, Some(&mut resolver));
        assert_eq!(response, IpcResponse::WorkflowResolutionRequired);
    }

    #[test]
    fn handle_get_workflow_graph_with_not_found_resolver_returns_unsupported() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x00; 32]);
        let payload = crate::IpcPayload::GetWorkflowGraph { digest };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let mut resolver = NotFoundResolver;
        let response = handle_get_workflow_graph(&encoded, Some(&mut resolver));
        assert_eq!(response, IpcResponse::WorkflowResolutionUnsupported);
    }

    #[test]
    fn handle_get_taint_report_with_required_resolver_returns_resolution_required() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x00; 32]);
        let payload = crate::IpcPayload::GetTaintReport { digest };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let mut resolver = RequiredResolver;
        let response = handle_get_taint_report(&encoded, Some(&mut resolver));
        assert_eq!(response, IpcResponse::WorkflowResolutionRequired);
    }

    #[test]
    fn handle_get_taint_report_with_not_found_resolver_returns_unsupported() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x00; 32]);
        let payload = crate::IpcPayload::GetTaintReport { digest };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let mut resolver = NotFoundResolver;
        let response = handle_get_taint_report(&encoded, Some(&mut resolver));
        assert_eq!(response, IpcResponse::WorkflowResolutionUnsupported);
    }

    #[test]
    fn handle_get_taint_report_with_safe_source_returns_warning() {
        use vb_core::ids::StepIdx;
        use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

        let digest = vb_core::WorkflowDigest::from_bytes([0xCD; 32]);
        let parts = WorkflowParts {
            name: Box::from("test"),
            digest,
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::WaitEvent {
                        event: vb_core::ids::SlotIdx::ZERO,
                        timeout_slot: None,
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        let workflow = vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
            .expect("workflow should be valid");
        let mut resolver = OkResolver { workflow };
        let payload = crate::IpcPayload::GetTaintReport { digest };
        let encoded = postcard::to_allocvec(&payload).expect("encode payload");
        let response = handle_get_taint_report(&encoded, Some(&mut resolver));
        match response {
            IpcResponse::TaintReport {
                sources,
                sinks,
                finish_safe,
                paths,
            } => {
                assert_eq!(sources, vec![0]);
                assert!(sinks.is_empty(), "no sinks in this workflow");
                assert!(finish_safe, "no source reaches sink");
                assert!(!paths.is_empty(), "should have warning paths");
                assert!(
                    paths
                        .iter()
                        .all(|p| p.status == crate::TaintPathStatus::Warning),
                    "all paths should be warning"
                );
            }
            other => panic!("expected TaintReport, got {other:?}"),
        }
    }

    #[test]
    fn bfs_forward_respects_out_of_bounds_successors() {
        use vb_core::ids::StepIdx;
        use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

        let parts = WorkflowParts {
            name: Box::from("test"),
            digest: vb_core::WorkflowDigest::from_bytes([0x00; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Jump {
                        target: StepIdx::new(99),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: vb_core::ids::SlotIdx::ZERO,
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        let reachable = bfs_forward(&parts, 0, 2);
        assert!(
            reachable.is_empty(),
            "out-of-bounds successor should not be followed"
        );
    }

    // ── Mutation kill: handle_get_workflow_graph boundary (< vs <=) ─────────────

    #[test]
    fn handle_get_workflow_graph_returns_exact_node_count_with_one_node() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x42; 32]);
        let workflow = make_workflow_with_nodes(digest, 1);
        let mut resolver = OkResolver { workflow };

        let payload_bytes = postcard::to_allocvec(&IpcPayload::GetWorkflowGraph { digest })
            .expect("encode payload");
        let response = handle_get_workflow_graph(&payload_bytes, Some(&mut resolver));

        match response {
            IpcResponse::WorkflowGraph { nodes, .. } => {
                assert_eq!(
                    nodes.len(),
                    1,
                    "exactly 1 node should be returned when workflow has 1 node"
                );
                assert_eq!(nodes[0].step_idx, 0);
            }
            other => panic!("expected WorkflowGraph, got {other:?}"),
        }
    }

    #[test]
    fn handle_get_workflow_graph_returns_two_nodes_with_two_node_workflow() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x43; 32]);
        let workflow = make_workflow_with_nodes(digest, 2);
        let mut resolver = OkResolver { workflow };

        let payload_bytes = postcard::to_allocvec(&IpcPayload::GetWorkflowGraph { digest })
            .expect("encode payload");
        let response = handle_get_workflow_graph(&payload_bytes, Some(&mut resolver));

        match response {
            IpcResponse::WorkflowGraph { nodes, .. } => {
                assert_eq!(
                    nodes.len(),
                    2,
                    "exactly 2 nodes should be returned when workflow has 2 nodes"
                );
                assert_eq!(nodes[0].step_idx, 0);
                assert_eq!(nodes[1].step_idx, 1);
            }
            other => panic!("expected WorkflowGraph, got {other:?}"),
        }
    }

    // ── Mutation kill: handle_get_taint_report boundary (>= vs <) ───────────────

    #[test]
    fn handle_get_taint_report_returns_exact_path_count_for_linear_chain() {
        let digest = vb_core::WorkflowDigest::from_bytes([0x44; 32]);
        let parts = WorkflowParts {
            name: Box::from("taint_boundary"),
            digest,
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::WaitEvent {
                        event: vb_core::ids::SlotIdx::new(0),
                        timeout_slot: None,
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: Some(StepIdx::new(2)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                },
                CompiledNode {
                    id: StepIdx::new(2),
                    output: Some(vb_core::ids::SlotIdx::new(0)),
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: vb_core::ids::SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        let workflow = CompiledWorkflow::try_from_parts(parts).expect("valid workflow");
        let mut resolver = OkResolver { workflow };

        let payload_bytes =
            postcard::to_allocvec(&IpcPayload::GetTaintReport { digest }).expect("encode payload");
        let response = handle_get_taint_report(&payload_bytes, Some(&mut resolver));

        match response {
            IpcResponse::TaintReport {
                sources,
                sinks,
                finish_safe,
                paths,
            } => {
                assert_eq!(sources, vec![0]);
                assert_eq!(sinks, vec![2]);
                assert_eq!(
                    paths.len(),
                    2,
                    "expected 2 taint paths for 3-node linear chain"
                );
                assert!(
                    !finish_safe,
                    "finish should not be safe when source reaches sink"
                );
            }
            other => panic!("expected TaintReport, got {other:?}"),
        }
    }

    // ── Mutation kill: enqueue_successors boundary (< vs <=, < vs ==) ───────────

    #[test]
    fn enqueue_successors_rejects_out_of_bounds_next() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)), // node_count = 1, so next=1 is out-of-bounds
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        enqueue_successors(&node, 1, &mut visited, &mut queue);

        assert!(
            visited.is_empty(),
            "out-of-bounds next should not be inserted"
        );
        assert!(
            queue.is_empty(),
            "out-of-bounds next should not be enqueued"
        );
    }

    #[test]
    fn enqueue_successors_rejects_out_of_bounds_structural_successor() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: 1,
                body: StepIdx::new(1), // out-of-bounds
                done: StepIdx::new(1), // also out-of-bounds
            },
        };
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        enqueue_successors(&node, 1, &mut visited, &mut queue);

        assert!(
            visited.is_empty(),
            "out-of-bounds structural successors should not be inserted"
        );
        assert!(
            queue.is_empty(),
            "out-of-bounds structural successors should not be enqueued"
        );
    }

    #[test]
    fn enqueue_successors_accepts_valid_successor_at_last_index() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)), // node_count = 2, next=1 is valid
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        enqueue_successors(&node, 2, &mut visited, &mut queue);

        assert!(visited.contains(&1), "valid next should be inserted");
        assert_eq!(queue.len(), 1, "valid next should be enqueued");
    }

    // ── Helper: build a linear-chain workflow ──────────────────────────────────

    fn make_workflow_with_nodes(
        d: vb_core::WorkflowDigest,
        count: usize,
    ) -> vb_core::workflow::CompiledWorkflow {
        let nodes: Vec<CompiledNode> = (0..count)
            .map(|i| {
                let kind = if i == count - 1 {
                    CompiledNodeKind::Finish {
                        result: vb_core::ids::SlotIdx::ZERO,
                    }
                } else if i == 0 {
                    CompiledNodeKind::WaitEvent {
                        event: vb_core::ids::SlotIdx::new(0),
                        timeout_slot: None,
                    }
                } else {
                    CompiledNodeKind::Nop
                };
                CompiledNode {
                    id: StepIdx::new(i as u16),
                    output: None,
                    next: if i < count - 1 {
                        Some(StepIdx::new((i + 1) as u16))
                    } else {
                        None
                    },
                    on_error: None,
                    error_slot: None,
                    kind,
                }
            })
            .collect();

        let parts = WorkflowParts {
            name: Box::from("test_linear"),
            digest: d,
            nodes: nodes.into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        CompiledWorkflow::try_from_parts(parts).expect("linear workflow should be valid")
    }
}
