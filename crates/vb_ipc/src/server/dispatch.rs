#![forbid(unsafe_code)]
//! IPC command dispatch.

use vb_runtime::runtime::Runtime;

use super::error::IpcServerError;
use crate::server::{IpcServer, WorkflowResolver};

/// Serves one IPC polling turn on an existing server.
pub fn serve_ipc(
    server: &mut IpcServer,
    runtime: &mut Runtime,
    timeout: Option<std::time::Duration>,
) -> Result<bool, IpcServerError> {
    server.poll_once(runtime, timeout)
}

/// Serves one IPC polling turn with workflow resolution for submit commands.
pub fn serve_ipc_with_resolver(
    server: &mut IpcServer,
    runtime: &mut Runtime,
    timeout: Option<std::time::Duration>,
    resolver: Option<&mut dyn WorkflowResolver>,
) -> Result<bool, IpcServerError> {
    server.poll_once_with_resolver(runtime, timeout, resolver)
}

use super::handlers::{
    handle_answer_ask, handle_cancel_run, handle_complete_action, handle_fail_action,
    handle_health, handle_inspect_run, handle_list_events, handle_shutdown, handle_submit_run,
    handle_submit_run_inline,
};
use super::trace::handle_drain_trace;
use crate::IpcCommand;
use crate::server::IpcResponse;

#[cfg(test)]
pub fn dispatch_command(
    header: &crate::IpcFrameHeader,
    payload: &[u8],
    runtime: &mut Runtime,
) -> IpcResponse {
    dispatch_command_with_resolver(header, payload, runtime, None)
}

pub fn dispatch_command_with_resolver(
    header: &crate::IpcFrameHeader,
    payload: &[u8],
    runtime: &mut Runtime,
    resolver: Option<&mut dyn WorkflowResolver>,
) -> IpcResponse {
    match header.command {
        IpcCommand::Health => handle_health(),
        IpcCommand::Shutdown => handle_shutdown(runtime),
        IpcCommand::SubmitRun => handle_submit_run(header, payload, runtime, resolver),
        IpcCommand::SubmitRunInline => handle_submit_run_inline(payload, runtime, resolver),
        IpcCommand::CancelRun => handle_cancel_run(payload, runtime),
        IpcCommand::InspectRun => handle_inspect_run(payload, runtime),
        IpcCommand::ListEvents => handle_list_events(payload, runtime),
        IpcCommand::AnswerAsk => handle_answer_ask(payload, runtime),
        IpcCommand::CompleteAction => handle_complete_action(payload, runtime),
        IpcCommand::FailAction => handle_fail_action(payload, runtime),
        IpcCommand::DrainTrace => handle_drain_trace(payload, runtime),
        IpcCommand::UnknownCommand(_) => IpcResponse::BadRequest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::{WorkflowResolutionError, WorkflowResolver};
    use std::num::NonZeroUsize;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;
    use vb_runtime::runtime::Runtime;
    use vb_runtime::shard::ShardConfig;

    static NEXT_SOCKET_ID: AtomicU64 = AtomicU64::new(0);

    fn temp_socket_path(name: &str) -> PathBuf {
        let sequence_result =
            NEXT_SOCKET_ID.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            });
        let sequence = match sequence_result {
            Ok(current) => current,
            Err(_overflowed_current) => 0,
        };
        let suffix = name
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .take(12)
            .collect::<String>();
        PathBuf::from(format!(
            "/tmp/vbd_{}_{}_{}.sock",
            std::process::id(),
            sequence,
            suffix
        ))
    }

    struct CleanupPath<'a>(&'a std::path::Path);
    impl Drop for CleanupPath<'_> {
        fn drop(&mut self) {
            if let Err(_cleanup_error) = std::fs::remove_file(self.0) {}
        }
    }

    fn make_runtime() -> Runtime {
        let mut config = ShardConfig::default();
        config.policy = vb_core::policy::RuntimePolicy::Relaxed;
        Runtime::new(NonZeroUsize::MIN, config)
    }

    #[test]
    fn serve_ipc_returns_true_when_server_should_continue() {
        let path = temp_socket_path("continue");
        let _cleanup = CleanupPath(&path);
        let mut server = IpcServer::bind(&path).expect("bind should succeed");
        let mut runtime = make_runtime();
        server.set_test_poll_once_result(Ok(true));
        let result = serve_ipc(&mut server, &mut runtime, Some(Duration::ZERO));
        assert_eq!(result, Ok(true));
    }

    #[test]
    fn serve_ipc_returns_false_on_shutdown() {
        let path = temp_socket_path("shutdown");
        let _cleanup = CleanupPath(&path);
        let mut server = IpcServer::bind(&path).expect("bind should succeed");
        let mut runtime = make_runtime();
        server.set_test_poll_once_result(Ok(false));
        let result = serve_ipc(&mut server, &mut runtime, Some(Duration::ZERO));
        assert_eq!(result, Ok(false));
    }

    #[test]
    fn serve_ipc_propagates_poll_once_errors() {
        let path = temp_socket_path("error");
        let _cleanup = CleanupPath(&path);
        let mut server = IpcServer::bind(&path).expect("bind should succeed");
        let mut runtime = make_runtime();
        server.set_test_poll_once_result(Err(IpcServerError::TooManyClients));
        let result = serve_ipc(&mut server, &mut runtime, Some(Duration::ZERO));
        assert!(matches!(result, Err(IpcServerError::TooManyClients)));
    }

    #[test]
    fn serve_ipc_with_resolver_forwards_to_poll_once_with_resolver() {
        let path = temp_socket_path("resolver");
        let _cleanup = CleanupPath(&path);
        let mut server = IpcServer::bind(&path).expect("bind should succeed");
        let mut runtime = make_runtime();
        server.set_test_poll_once_result(Ok(true));
        let result = serve_ipc_with_resolver(&mut server, &mut runtime, Some(Duration::ZERO), None);
        assert_eq!(result, Ok(true));
    }

    #[test]
    fn serve_ipc_with_resolver_returns_false_when_server_should_shutdown() {
        let path = temp_socket_path("resolver_shutdown");
        let _cleanup = CleanupPath(&path);
        let mut server = IpcServer::bind(&path).expect("bind should succeed");
        let mut runtime = make_runtime();
        server.set_test_poll_once_result(Ok(false));
        let result = serve_ipc_with_resolver(&mut server, &mut runtime, Some(Duration::ZERO), None);
        assert_eq!(result, Ok(false));
    }

    #[test]
    fn serve_ipc_with_resolver_propagates_poll_once_errors() {
        let path = temp_socket_path("resolver_error");
        let _cleanup = CleanupPath(&path);
        let mut server = IpcServer::bind(&path).expect("bind should succeed");
        let mut runtime = make_runtime();
        server.set_test_poll_once_result(Err(IpcServerError::TooManyClients));
        let result = serve_ipc_with_resolver(&mut server, &mut runtime, Some(Duration::ZERO), None);
        assert!(matches!(result, Err(IpcServerError::TooManyClients)));
    }

    #[test]
    fn dispatch_command_wrapper_delegates_to_dispatch_command_with_resolver() {
        let mut runtime = make_runtime();
        let header = crate::IpcFrameHeader::new(IpcCommand::Health, 0, 0, 0);
        let response = dispatch_command(&header, &[], &mut runtime);
        assert_eq!(response, IpcResponse::Healthy);
    }

    #[test]
    fn dispatch_command_with_resolver_submit_run_inline() {
        let mut runtime = make_runtime();
        let header = crate::IpcFrameHeader::new(IpcCommand::SubmitRunInline, 0, 0, 0);
        let response = dispatch_command_with_resolver(&header, &[], &mut runtime, None);
        match response {
            IpcResponse::PayloadError { .. } | IpcResponse::BadRequest => {}
            other => panic!("expected PayloadError or BadRequest, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_command_with_resolver_cancel_run() {
        let mut runtime = make_runtime();
        let header = crate::IpcFrameHeader::new(IpcCommand::CancelRun, 0, 0, 0);
        let response = dispatch_command_with_resolver(&header, &[], &mut runtime, None);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn dispatch_command_with_resolver_inspect_run() {
        let mut runtime = make_runtime();
        let header = crate::IpcFrameHeader::new(IpcCommand::InspectRun, 0, 0, 0);
        let response = dispatch_command_with_resolver(&header, &[], &mut runtime, None);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn dispatch_command_with_resolver_list_events() {
        let mut runtime = make_runtime();
        let header = crate::IpcFrameHeader::new(IpcCommand::ListEvents, 0, 0, 0);
        let response = dispatch_command_with_resolver(&header, &[], &mut runtime, None);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn dispatch_command_with_resolver_answer_ask() {
        let mut runtime = make_runtime();
        let header = crate::IpcFrameHeader::new(IpcCommand::AnswerAsk, 0, 0, 0);
        let response = dispatch_command_with_resolver(&header, &[], &mut runtime, None);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn dispatch_command_with_resolver_complete_action() {
        let mut runtime = make_runtime();
        let header = crate::IpcFrameHeader::new(IpcCommand::CompleteAction, 0, 0, 0);
        let response = dispatch_command_with_resolver(&header, &[], &mut runtime, None);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn dispatch_command_with_resolver_fail_action() {
        let mut runtime = make_runtime();
        let header = crate::IpcFrameHeader::new(IpcCommand::FailAction, 0, 0, 0);
        let response = dispatch_command_with_resolver(&header, &[], &mut runtime, None);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn dispatch_command_with_resolver_drain_trace() {
        let mut runtime = make_runtime();
        let header = crate::IpcFrameHeader::new(IpcCommand::DrainTrace, 0, 0, 0);
        let response = dispatch_command_with_resolver(&header, &[], &mut runtime, None);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    // ══ UnknownCommand dispatch tests (PO-KANI-003 compensating evidence) ═══════

    #[test]
    fn dispatch_unknown_command_returns_bad_request() {
        let mut runtime = make_runtime();
        let header = crate::IpcFrameHeader::new(IpcCommand::UnknownCommand(99), 0, 0, 0);
        let response = dispatch_command_with_resolver(&header, &[], &mut runtime, None);
        assert_eq!(response, IpcResponse::BadRequest);
    }

    #[test]
    fn dispatch_unknown_command_with_resolver_present_returns_bad_request() {
        let mut runtime = make_runtime();
        struct NoopResolver;
        impl WorkflowResolver for NoopResolver {
            fn resolve_workflow(
                &mut self,
                _digest: vb_core::WorkflowDigest,
            ) -> Result<vb_core::workflow::CompiledWorkflow, WorkflowResolutionError> {
                Err(WorkflowResolutionError::NotFound)
            }
        }
        let mut resolver = NoopResolver;
        let header = crate::IpcFrameHeader::new(IpcCommand::UnknownCommand(42), 0, 0, 0);
        let response =
            dispatch_command_with_resolver(&header, &[], &mut runtime, Some(&mut resolver));
        assert_eq!(
            response,
            IpcResponse::BadRequest,
            "UnknownCommand must return BadRequest even when resolver is present"
        );
    }

    #[test]
    fn dispatch_unknown_command_with_various_ids_returns_bad_request() {
        let mut runtime = make_runtime();
        let unknown_ids = [0u16, 12, 13, 14, 15, 16, 99, u16::MAX];
        for &id in &unknown_ids {
            let header = crate::IpcFrameHeader::new(IpcCommand::UnknownCommand(id), 0, 0, 0);
            let response = dispatch_command_with_resolver(&header, &[], &mut runtime, None);
            assert_eq!(
                response,
                IpcResponse::BadRequest,
                "UnknownCommand({id}) must return BadRequest"
            );
        }
    }

    // ══ Missing dispatch arm coverage (SubmitRun, Shutdown) ═════════════════════

    #[test]
    fn dispatch_submit_run_without_resolver_returns_workflow_resolution_required() {
        let mut runtime = make_runtime();
        let header = crate::IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 0, 0);
        // Encode a valid SubmitRun payload so the handler proceeds past payload
        // decode and reaches the resolver gate, verifying the dispatch arm is
        // exercised end-to-end.
        let submit_payload = crate::SubmitRunPayload {
            run_id: vb_core::ids::RunId::new(1),
            workflow: vb_core::ids::WorkflowDigest::from_bytes([0xAB; 32]),
            input: Vec::new(),
        };
        let encoded = postcard::to_allocvec(&crate::IpcPayload::SubmitRun(submit_payload))
            .expect("test payload must encode");
        let response = dispatch_command_with_resolver(&header, &encoded, &mut runtime, None);
        assert_eq!(response, IpcResponse::WorkflowResolutionRequired);
    }

    #[test]
    fn dispatch_shutdown_returns_shutting_down() {
        let mut runtime = make_runtime();
        let header = crate::IpcFrameHeader::new(IpcCommand::Shutdown, 0, 0, 0);
        let response = dispatch_command_with_resolver(&header, &[], &mut runtime, None);
        assert_eq!(response, IpcResponse::ShuttingDown);
    }
}
