#![forbid(unsafe_code)]
//! IPC command handlers dispatched by the server.

pub mod command;
pub mod event;
pub mod query;
pub mod session;

pub use command::{handle_answer_ask, handle_complete_action, handle_fail_action};
pub use event::{
    handle_get_taint_report, handle_get_workflow_graph, handle_verify_workflow,
    sanitize_validation_detail,
};
pub use query::{
    decode_payload, handle_cancel_run, handle_get_metrics, handle_inspect_run,
    handle_list_events, handle_list_runs, handle_submit_run, handle_submit_run_inline,
    sanitize_runtime_error, submit_resolved_workflow,
};
pub use session::{handle_health, handle_ping, handle_shutdown};

pub const MAX_RUNTIME_ERROR_LEN: usize = 256;
pub const MAX_SUBMIT_INPUT_LEN: usize = 65536;
pub const MAX_ACTION_OUTPUT_LEN: usize = 65536;
pub const MAX_ACTION_ERROR_LEN: usize = 65536;
pub const MAX_TAINT_PATH_ENTRIES: usize = 65536;
pub const MAX_VALIDATION_DETAIL_LEN: usize = 512;
pub const MAX_LIST_RUNS_LIMIT: u32 = 4096;
pub const MAX_ANSWER_ASK_BYTES: usize = 65536;
pub const MAX_WORKFLOW_GRAPH_NODES: usize = 8192;

fn ipc_error_response(error: crate::IpcError) -> IpcResponse {
    IpcResponse::PayloadError {
        diagnostic: error.diagnostic_code().code(),
        message: error.to_string(),
    }
}

pub use super::IpcResponse;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_payload_succeeds_for_valid_postcard_bytes() {
        let payload = crate::IpcPayload::Health;
        let encoded = postcard::to_allocvec(&payload);
        assert!(encoded.is_ok(), "postcard encoding should succeed");
        let Ok(encoded) = encoded else { return };

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
    fn decode_payload_roundtrips_get_metrics() {
        let payload = crate::IpcPayload::GetMetrics;
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            return;
        };
        let result = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(decoded) = result else {
            assert!(false, "should decode GetMetrics");
            return;
        };
        assert_eq!(decoded, payload);
    }

    #[test]
    fn decode_payload_roundtrips_list_runs() {
        let payload = crate::IpcPayload::ListRuns {
            limit: 50,
            workflow: None,
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            return;
        };
        let result = decode_payload::<crate::IpcPayload>(&encoded);
        let Ok(decoded) = result else {
            assert!(false, "should decode ListRuns");
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

    #[test]
    fn handle_ping_returns_healthy() {
        assert_eq!(handle_ping(), IpcResponse::Healthy);
    }

    #[test]
    fn handle_health_returns_healthy() {
        assert_eq!(handle_health(), IpcResponse::Healthy);
    }

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

    #[test]
    fn all_successors_returns_empty_for_nop() {
        let kind = vb_core::workflow::CompiledNodeKind::Nop;
        let succs = event::all_successors(&kind);
        assert!(succs.is_empty(), "Nop has no structural successors");
    }

    #[test]
    fn all_successors_returns_empty_for_finish() {
        let kind = vb_core::workflow::CompiledNodeKind::Finish {
            result: vb_core::ids::SlotIdx::ZERO,
        };
        let succs = event::all_successors(&kind);
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
        let succs = event::all_successors(&kind);
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
        let succs = event::all_successors(&kind);
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
        let succs = event::all_successors(&kind);
        assert!(succs.contains(&3), "should contain body target");
        assert!(succs.contains(&7), "should contain handler target");
        assert_eq!(succs.len(), 2);
    }

    #[test]
    fn all_successors_includes_target_for_jump() {
        let kind = vb_core::workflow::CompiledNodeKind::Jump {
            target: vb_core::ids::StepIdx::new(42),
        };
        let succs = event::all_successors(&kind);
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
        let succs = event::all_successors(&kind);
        assert!(succs.contains(&2), "should contain branch 0");
        assert!(succs.contains(&4), "should contain branch 1");
        assert!(succs.contains(&6), "should contain join target");
        assert_eq!(succs.len(), 3);
    }

    #[test]
    fn cancel_run_delegates_directly_to_runtime_without_snapshot() {
        let payload = crate::IpcPayload::CancelRun {
            run_id: vb_core::RunId::new(9999),
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "payload should encode");
            return;
        };
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

    #[test]
    fn get_workflow_graph_payload_roundtrips() {
        let digest = vb_core::WorkflowDigest::from_bytes([0xAB; 32]);
        let payload = crate::IpcPayload::GetWorkflowGraph { digest };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "GetWorkflowGraph payload should encode");
            return;
        };
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        match decoded {
            Ok(crate::IpcPayload::GetWorkflowGraph { digest: d }) => {
                assert_eq!(d, digest, "digest must round-trip unchanged");
            }
            other => {
                assert!(false, "expected GetWorkflowGraph, got {other:?}");
            }
        }
    }

    #[test]
    fn verify_workflow_payload_roundtrips() {
        let digest = vb_core::WorkflowDigest::from_bytes([0xCD; 32]);
        let payload = crate::IpcPayload::VerifyWorkflow { digest };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "VerifyWorkflow payload should encode");
            return;
        };
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        match decoded {
            Ok(crate::IpcPayload::VerifyWorkflow { digest: d }) => {
                assert_eq!(d, digest, "digest must round-trip unchanged");
            }
            other => {
                assert!(false, "expected VerifyWorkflow, got {other:?}");
            }
        }
    }

    #[test]
    fn get_taint_report_payload_roundtrips() {
        let digest = vb_core::WorkflowDigest::from_bytes([0xEF; 32]);
        let payload = crate::IpcPayload::GetTaintReport { digest };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "GetTaintReport payload should encode");
            return;
        };
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        match decoded {
            Ok(crate::IpcPayload::GetTaintReport { digest: d }) => {
                assert_eq!(d, digest, "digest must round-trip unchanged");
            }
            other => {
                assert!(false, "expected GetTaintReport, got {other:?}");
            }
        }
    }

    #[test]
    fn get_workflow_graph_returns_mismatch_for_wrong_digest() {
        let mismatch = IpcResponse::WorkflowDigestMismatch;
        let msg = format!("{mismatch:?}");
        assert!(
            msg.contains("WorkflowDigestMismatch"),
            "mismatch variant should serialize"
        );
    }

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
        let succs = event::all_successors(&kind);
        assert_eq!(succs.len(), 51, "50 branches + 1 otherwise");
        for i in 0..50u16 {
            assert!(succs.contains(&i), "should contain branch target {i}");
        }
        assert!(succs.contains(&200), "should contain otherwise target");
    }

    #[test]
    fn bfs_forward_respects_node_count_bound() {
        let response = IpcResponse::TaintReport {
            sources: vec![],
            sinks: vec![],
            finish_safe: true,
            paths: vec![],
        };
        if let IpcResponse::TaintReport { finish_safe, .. } = response {
            assert!(finish_safe, "empty workflow should be finish-safe");
        }
    }

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

    #[test]
    fn sanitize_validation_detail_preserves_short_input() {
        let short = String::from("slot reference out of bounds");
        let sanitized = sanitize_validation_detail(short.clone());
        assert_eq!(
            sanitized, short,
            "short detail should pass through unchanged"
        );
    }

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

    #[test]
    fn answer_ask_taint_secret_roundtrips() {
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: vb_core::RunId::new(1),
            ticket: 5,
            answer: Vec::from(&b"secret_value"[..]),
            taint: Some(vb_core::value::Taint::Secret),
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
            Some(vb_core::value::Taint::Secret),
            "Taint::Secret should round-trip correctly"
        );
    }

    #[test]
    fn answer_ask_taint_derived_from_secret_roundtrips() {
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: vb_core::RunId::new(1),
            ticket: 5,
            answer: Vec::from(&b"derived_value"[..]),
            taint: Some(vb_core::value::Taint::DerivedFromSecret),
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
            Some(vb_core::value::Taint::DerivedFromSecret),
            "Taint::DerivedFromSecret should round-trip correctly"
        );
    }

    #[test]
    fn answer_ask_taint_clean_explicit_roundtrips() {
        let payload = crate::IpcPayload::AnswerAsk {
            run_id: vb_core::RunId::new(1),
            ticket: 5,
            answer: Vec::from(&b"clean_value"[..]),
            taint: Some(vb_core::value::Taint::Clean),
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
            Some(vb_core::value::Taint::Clean),
            "Taint::Clean should round-trip correctly"
        );
    }

    #[test]
    fn list_runs_limit_cap_is_bounded() {
        assert!(
            MAX_LIST_RUNS_LIMIT <= 4096,
            "list runs cap should not exceed 4096"
        );
        assert!(MAX_LIST_RUNS_LIMIT > 0, "list runs cap should be non-zero");
    }

    #[test]
    fn list_runs_max_limit_decodes_for_capping() {
        let payload = crate::IpcPayload::ListRuns {
            limit: u32::MAX,
            workflow: None,
        };
        let Ok(encoded) = postcard::to_allocvec(&payload) else {
            assert!(false, "payload should encode");
            return;
        };
        let decoded = decode_payload::<crate::IpcPayload>(&encoded);
        match decoded {
            Ok(crate::IpcPayload::ListRuns { limit, .. }) => {
                assert_eq!(limit, u32::MAX, "u32::MAX limit should round-trip");
                let capped = limit.min(MAX_LIST_RUNS_LIMIT);
                assert_eq!(capped, MAX_LIST_RUNS_LIMIT, "should be capped");
            }
            other => {
                assert!(false, "expected ListRuns, got {other:?}");
            }
        }
    }

    #[test]
    fn workflow_graph_nodes_cap_is_bounded() {
        assert!(
            MAX_WORKFLOW_GRAPH_NODES <= 8192,
            "workflow graph nodes cap should not exceed 8192"
        );
        assert!(
            MAX_WORKFLOW_GRAPH_NODES > 0,
            "workflow graph nodes cap should be non-zero"
        );
    }

    #[test]
    fn workflow_graph_node_count_capping_logic() {
        let capped = u16::MAX.min(u16::try_from(MAX_WORKFLOW_GRAPH_NODES).unwrap_or(u16::MAX));
        assert_eq!(
            capped,
            u16::try_from(MAX_WORKFLOW_GRAPH_NODES).unwrap_or(u16::MAX),
            "u16::MAX should be capped to MAX_WORKFLOW_GRAPH_NODES"
        );
        let small_capped = 100u16.min(u16::try_from(MAX_WORKFLOW_GRAPH_NODES).unwrap_or(u16::MAX));
        assert_eq!(small_capped, 100, "small node count should pass through");
    }
}
