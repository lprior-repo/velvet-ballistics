#![forbid(unsafe_code)]
//! Run submission handlers.

use vb_core::ids::SlotIdx;
use vb_core::value::SlotValue;
use vb_core::workflow::CompiledWorkflow;
use vb_runtime::runtime::Runtime;

use super::{MAX_SUBMIT_INPUT_LEN, decode_payload, ipc_error_response, sanitize_runtime_error};
use crate::server::{IpcResponse, WorkflowResolutionError, WorkflowResolver};

use crate::{IpcCommand, SubmitRunPayload};

/// Handles submit-run commands after resolving the compiled workflow explicitly.
pub fn handle_submit_run(
    header: &crate::IpcFrameHeader,
    payload: &[u8],
    runtime: &mut Runtime,
    resolver: Option<&mut dyn WorkflowResolver>,
) -> IpcResponse {
    let decoded = match decode_payload::<crate::IpcPayload>(payload) {
        Ok(d) => d,
        Err(response) => return response,
    };

    match (header.command, decoded) {
        (IpcCommand::SubmitRun, crate::IpcPayload::SubmitRun(submit))
        | (IpcCommand::SubmitRunInline, crate::IpcPayload::SubmitRunInline(submit)) => {
            submit_resolved_workflow(header.command, submit, runtime, resolver)
        }
        _ => IpcResponse::CommandPayloadMismatch,
    }
}

/// Handles inline submit-run commands.
pub fn handle_submit_run_inline(
    payload: &[u8],
    runtime: &mut Runtime,
    resolver: Option<&mut dyn WorkflowResolver>,
) -> IpcResponse {
    let header = crate::IpcFrameHeader::new(IpcCommand::SubmitRunInline, 0, 0, 0);
    handle_submit_run(&header, payload, runtime, resolver)
}

pub fn submit_resolved_workflow(
    command: IpcCommand,
    submit: SubmitRunPayload,
    runtime: &mut Runtime,
    resolver: Option<&mut dyn WorkflowResolver>,
) -> IpcResponse {
    if let Some(response) = submit_input_error(&submit) {
        return response;
    }
    let Some(resolver) = resolver else {
        return IpcResponse::WorkflowResolutionRequired;
    };
    let resolved = match resolve_submission(&submit, resolver) {
        Ok(resolved) => resolved,
        Err(response) => return response,
    };
    match dispatch_submission(command, &submit, runtime, resolver, resolved) {
        Ok(()) => IpcResponse::AcceptedRun {
            run_id: submit.run_id.get(),
        },
        Err(response) => response,
    }
}
fn submit_input_error(submit: &SubmitRunPayload) -> Option<IpcResponse> {
    if submit.input.len() <= MAX_SUBMIT_INPUT_LEN {
        return None;
    }
    Some(IpcResponse::PayloadError {
        diagnostic: crate::IpcError::PayloadDecodeFailed
            .diagnostic_code()
            .code(),
        message: String::from("submit input exceeds maximum allowed size"),
    })
}

struct ResolvedSubmission {
    workflow: CompiledWorkflow,
    inputs: Box<[(SlotIdx, SlotValue)]>,
    contracts: Box<[vb_core::action::ActionContract]>,
    grants: vb_core::CapabilitySet,
}

fn resolve_submission(
    submit: &SubmitRunPayload,
    resolver: &mut dyn WorkflowResolver,
) -> Result<ResolvedSubmission, IpcResponse> {
    let workflow = resolver
        .resolve_workflow(submit.workflow)
        .map_err(|error| match error {
            WorkflowResolutionError::Required => IpcResponse::WorkflowResolutionRequired,
            WorkflowResolutionError::NotFound | WorkflowResolutionError::InvalidArtifact => {
                IpcResponse::WorkflowResolutionUnsupported
            }
        })?;
    if workflow.digest() != submit.workflow {
        return Err(IpcResponse::WorkflowDigestMismatch);
    }
    let inputs = map_submit_inputs(&workflow, &submit.input)?;
    let contracts = resolver.resolve_action_contracts(&workflow);
    let grants = resolver.resolve_capability_grants(&workflow);
    Ok(ResolvedSubmission {
        workflow,
        inputs,
        contracts,
        grants,
    })
}

fn dispatch_submission(
    command: IpcCommand,
    submit: &SubmitRunPayload,
    runtime: &mut Runtime,
    resolver: &mut dyn WorkflowResolver,
    resolved: ResolvedSubmission,
) -> Result<(), IpcResponse> {
    if !matches!(command, IpcCommand::SubmitRun | IpcCommand::SubmitRunInline) {
        return Err(IpcResponse::CommandPayloadMismatch);
    }
    resolver.prepare_runtime(runtime, &resolved.workflow);
    submit_runtime(runtime, submit, resolved)
}

fn submit_runtime(
    runtime: &mut Runtime,
    submit: &SubmitRunPayload,
    resolved: ResolvedSubmission,
) -> Result<(), IpcResponse> {
    let result = if resolved.contracts.is_empty() {
        runtime.submit_compiled_with_inputs(submit.run_id, resolved.workflow, resolved.inputs)
    } else {
        runtime.submit_direct_with_inputs_grants_and_contracts(
            submit.run_id,
            resolved.workflow,
            resolved.inputs,
            resolved.grants,
            resolved.contracts,
        )
    };
    result.map_err(|error| IpcResponse::RuntimeError {
        message: sanitize_runtime_error(&error),
    })
}

fn map_submit_inputs(
    workflow: &CompiledWorkflow,
    bytes: &[u8],
) -> Result<Box<[(SlotIdx, SlotValue)]>, IpcResponse> {
    let invalid = || ipc_error_response(crate::IpcError::PayloadDecodeFailed);
    let values: Box<[SlotValue]> = if bytes.is_empty() {
        Box::from([])
    } else {
        // Postcard sequences start with their usize length, before any elements.
        let (count, _) = postcard::take_from_bytes::<usize>(bytes).map_err(|_| invalid())?;
        if count > workflow.input_slots().len() {
            return Err(invalid());
        }
        let (values, remainder) =
            postcard::take_from_bytes::<Box<[SlotValue]>>(bytes).map_err(|_| invalid())?;
        if !remainder.is_empty() {
            return Err(invalid());
        }
        values
    };
    let declarations = workflow.input_slots();
    if values.len() > declarations.len() {
        return Err(invalid());
    }
    let mut mapped = Vec::new();
    mapped
        .try_reserve_exact(declarations.len())
        .map_err(|_| invalid())?;
    for (position, declaration) in declarations.iter().enumerate() {
        // Synthetic secret roots belong to the host and are seeded by Runtime.
        if values.get(position).is_none()
            && workflow.source_metadata().is_some_and(|metadata| {
                metadata
                    .secret_requirements()
                    .iter()
                    .any(|binding| binding.slot() == Some(declaration.slot))
            })
        {
            continue;
        }
        let value = if let Some(value) = values.get(position) {
            Some(*value)
        } else if declaration.constraint.requires_value()
            || declaration.constraint.default_cold.is_some()
        {
            return Err(invalid());
        } else {
            declaration.constraint.default_value
        };
        let Some(value) = value else {
            continue;
        };
        vb_core::input_validation::validate_scalar_input_value(declaration, value, workflow)
            .map_err(|_| invalid())?;
        mapped.push((declaration.slot, value));
    }
    Ok(mapped.into_boxed_slice())
}
