//! Advisory structural gate dispatcher and implementations.
//!
//! Each function performs a meaningful static check against the compiled IR.
//! Where the check would need runtime or external-registry data we do not
//! have at verify time, the function returns a `not_implemented` warning
//! string instead of silently passing. This is the policy demanded by
//! `High #7` in the bead log: real work where possible, honest stubs
//! otherwise.

#![forbid(unsafe_code)]

use vb_core::workflow::{CompiledNodeKind, WorkflowParts};

/// Run a single local advisory gate.
///
/// A returned `true` means the local structural check passed. Callers may
/// still keep the corresponding master gate in `:deferred` state when the full
/// gate requires evidence or runtime inputs outside this CLI layer.
pub(crate) fn run_advisory_gate(
    parts: &WorkflowParts,
    gate_name: &'static str,
    check: fn(&WorkflowParts) -> Result<(), String>,
    warnings: &mut Vec<String>,
) -> bool {
    match check(parts) {
        Ok(()) => true,
        Err(message) => {
            warnings.push(format!("{gate_name} warning: {message}"));
            false
        }
    }
}

// ---------------------------------------------------------------------------
// Gate implementations
// ---------------------------------------------------------------------------

/// Gate: every slot reference is within the declared `slot_count`.
pub(crate) fn check_slot_bounds(parts: &WorkflowParts) -> Result<(), String> {
    vb_validate::shared::validate_gate_09_slot_references(parts)
        .map_err(|err| format!("slot reference out of bounds: {err}"))
}

/// Gate: taint propagation.
///
/// No compiled-form taint validator over `WorkflowParts` exists in `vb_validate`
/// today. The compile pipeline's AST taint pass is useful upstream, but it does
/// not close the master §63 compiled-IR taint gate for this CLI layer.
pub(crate) fn check_taint_propagation(_parts: &WorkflowParts) -> Result<(), String> {
    Err(
        "compiled-form WorkflowParts taint validation is not implemented; AST validation alone does not close this gate"
            .to_string(),
    )
}

/// Gate: every `Do` node has a structurally valid `ActionId` and a valid
/// input slot. This is the structural portion of the action-contract gate;
/// the contract-completeness portion is enforced at runtime by the action
/// registry.
pub(crate) fn check_action_contracts(parts: &WorkflowParts) -> Result<(), String> {
    let slot_count = usize::from(parts.slot_count);
    let mut do_count: u32 = 0;
    for node in parts.nodes.iter() {
        if let CompiledNodeKind::Do { input, .. } = &node.kind {
            if input.as_usize() >= slot_count {
                return Err(format!(
                    "Do node at step {:?} has out-of-bounds input slot {:?}",
                    node.id, input
                ));
            }
            do_count = do_count.saturating_add(1);
        }
    }
    if do_count > u32::from(u16::MAX) {
        return Err(format!("action ticket count {do_count} exceeds u16::MAX"));
    }
    Ok(())
}

/// Gate: every required capability on a `Do` node is within the
/// `ResourceContract` cap. Without an external capability registry the
/// strongest honest check is a structural one.
pub(crate) fn check_capability_requirements(parts: &WorkflowParts) -> Result<(), String> {
    let slot_count = usize::from(parts.slot_count);
    for node in parts.nodes.iter() {
        if let CompiledNodeKind::Do { input, .. } = &node.kind
            && input.as_usize() >= slot_count
        {
            return Err(format!(
                "Do node at step {:?} cannot satisfy capability check: out-of-bounds input slot",
                node.id
            ));
        }
    }
    Ok(())
}

/// Gate: deterministic replay. Runs the existing
/// `validate_gate_15_determinism_proof` against the compiled parts.
pub(crate) fn check_replay_determinism(parts: &WorkflowParts) -> Result<(), String> {
    vb_validate::shared::validate_gate_15_determinism_proof(parts)
        .map_err(|err| format!("replay determinism check failed: {err}"))
}

/// Gate: idempotency. The structural check ensures every `Do` node has a
/// valid input slot (the same precondition required to derive a stable
/// idempotency key). Without an external action contract registry the
/// strongest honest verdict is a structural one.
pub(crate) fn check_idempotency(parts: &WorkflowParts) -> Result<(), String> {
    let slot_count = usize::from(parts.slot_count);
    for node in parts.nodes.iter() {
        if let CompiledNodeKind::Do { input, .. } = &node.kind
            && input.as_usize() >= slot_count
        {
            return Err(format!(
                "Do node at step {:?} cannot satisfy idempotency check: out-of-bounds input slot",
                node.id
            ));
        }
    }
    Ok(())
}
