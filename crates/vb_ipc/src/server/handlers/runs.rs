#![forbid(unsafe_code)]
//! Run lifecycle handlers: submit, cancel, inspect, and list events.

use vb_core::ids::RunId;
use vb_runtime::runtime::Runtime;

use crate::server::IpcResponse;
use crate::server::handlers::utilities::{decode_payload, sanitize_runtime_error};
use crate::server::{WorkflowResolutionError, WorkflowResolver};
use crate::{IpcCommand, IpcPayload, SubmitRunPayload};

pub(super) use submit::{
    SubmitCommand, handle_submit_run, handle_submit_run_inline, submit_resolved_workflow,
};

pub(super) mod submit {
    use super::*;
    use vb_core::workflow::CompiledWorkflow;

    /// Which wire command triggered a submit-run flow.
    #[derive(Clone, Copy)]
    pub enum SubmitCommand {
        SubmitRun,
        SubmitRunInline,
    }

    /// Handles a submit-run command after resolving the compiled workflow explicitly.
    pub fn handle_submit_run(
        command: SubmitCommand,
        header: &crate::IpcFrameHeader,
        payload: &[u8],
        runtime: &mut Runtime,
        resolver: Option<&mut dyn WorkflowResolver>,
    ) -> IpcResponse {
        let decoded = match decode_payload::<IpcPayload>(payload) {
            Ok(d) => d,
            Err(response) => return response,
        };

        match decoded {
            IpcPayload::SubmitRun(submit) if header.command == IpcCommand::SubmitRun => {
                submit_resolved_workflow(command, submit, runtime, resolver)
            }
            IpcPayload::SubmitRunInline(submit)
                if header.command == IpcCommand::SubmitRunInline =>
            {
                submit_resolved_workflow(command, submit, runtime, resolver)
            }
            _ => IpcResponse::CommandPayloadMismatch,
        }
    }

    /// Handles inline submit-run commands by constructing a header and delegating.
    pub fn handle_submit_run_inline(
        payload: &[u8],
        runtime: &mut Runtime,
        resolver: Option<&mut dyn WorkflowResolver>,
    ) -> IpcResponse {
        let header = crate::IpcFrameHeader::new(IpcCommand::SubmitRunInline, 0, 0, 0);
        handle_submit_run(
            SubmitCommand::SubmitRunInline,
            &header,
            payload,
            runtime,
            resolver,
        )
    }

    /// Maximum allowed size for the `SubmitRunPayload.input` field.
    /// Prevents unbounded allocation from deserialized input bytes.
    const MAX_SUBMIT_INPUT_LEN: usize = 65536;

    /// Resolves the workflow and submits it to the runtime.
    pub fn submit_resolved_workflow(
        _command: SubmitCommand,
        submit: SubmitRunPayload,
        runtime: &mut Runtime,
        resolver: Option<&mut dyn WorkflowResolver>,
    ) -> IpcResponse {
        if submit.input.len() > MAX_SUBMIT_INPUT_LEN {
            return IpcResponse::PayloadError {
                diagnostic: crate::IpcError::PayloadDecodeFailed
                    .diagnostic_code()
                    .code(),
                message: String::from("submit input exceeds maximum allowed size"),
            };
        }
        let Some(resolver) = resolver else {
            return IpcResponse::WorkflowResolutionRequired;
        };
        let workflow = match resolver.resolve_workflow(submit.workflow) {
            Ok(workflow) => workflow,
            Err(WorkflowResolutionError::Required) => {
                return IpcResponse::WorkflowResolutionRequired;
            }
            Err(WorkflowResolutionError::NotFound | WorkflowResolutionError::InvalidArtifact) => {
                return IpcResponse::WorkflowResolutionUnsupported;
            }
        };
        if workflow.digest() != submit.workflow {
            return IpcResponse::WorkflowDigestMismatch;
        }
        let result = match _command {
            SubmitCommand::SubmitRun => runtime.submit_compiled(submit.run_id, workflow),
            SubmitCommand::SubmitRunInline => runtime.submit_direct(submit.run_id, workflow),
        };
        match result {
            Ok(()) => IpcResponse::AcceptedRun {
                run_id: submit.run_id.get(),
            },
            Err(e) => IpcResponse::RuntimeError {
                message: sanitize_runtime_error(&e),
            },
        }
    }
}

/// Handles cancel-run.
pub fn handle_cancel_run(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(IpcPayload::CancelRun { run_id, reason }) = decode_payload::<IpcPayload>(payload) else {
        return IpcResponse::BadRequest;
    };

    let reason_str = reason.and_then(|bytes| String::from_utf8(bytes).ok());
    match runtime.cancel_run_with_reason(run_id, reason_str) {
        Ok(()) => IpcResponse::AcceptedRun {
            run_id: run_id.get(),
        },
        Err(e) => IpcResponse::RuntimeError {
            message: sanitize_runtime_error(&e),
        },
    }
}

/// Handles inspect-run.
pub fn handle_inspect_run(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(IpcPayload::InspectRun { run_id }) = decode_payload::<IpcPayload>(payload) else {
        return IpcResponse::BadRequest;
    };

    match runtime.snapshot_run(run_id, 0) {
        Ok(vb_runtime::shard::InspectResponse::Found(_snapshot)) => IpcResponse::Inspected {
            run_id: run_id.get(),
        },
        Ok(vb_runtime::shard::InspectResponse::NotFound { .. }) => IpcResponse::RuntimeError {
            message: String::from("run not found"),
        },
        // Handle unknown future InspectResponse variants conservatively.
        #[allow(unreachable_code)]
        Ok(_) => IpcResponse::RuntimeError {
            message: String::from("unknown inspect response variant"),
        },
        Err(e) => IpcResponse::RuntimeError {
            message: sanitize_runtime_error(&e),
        },
    }
}

/// Handles list-events.
pub fn handle_list_events(payload: &[u8], runtime: &mut Runtime) -> IpcResponse {
    let Ok(IpcPayload::ListEvents {
        run_id,
        from_sequence,
    }) = decode_payload::<IpcPayload>(payload)
    else {
        return IpcResponse::BadRequest;
    };

    match runtime.list_events(run_id) {
        Ok(events) => super::super::trace::typed_events_response(&events, from_sequence),
        Err(e) => IpcResponse::RuntimeError {
            message: sanitize_runtime_error(&e),
        },
    }
}
