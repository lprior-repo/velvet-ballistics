#![forbid(unsafe_code)]
//! Compiled-workflow summary JSON for the AI context packet.

use serde_json::Value;

use super::push_unique_u32;

fn digest_hex(digest: vb_core::WorkflowDigest) -> String {
    digest
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub(crate) fn ai_workflow_summary(
    journal: &vb_storage::FjallJournal,
    digest: Option<vb_core::WorkflowDigest>,
) -> Value {
    let Some(digest) = digest else {
        return serde_json::json!({
            "digest": null,
            "compiled_ir": {"available": false, "reason": "workflow digest not present in run header or events"},
            "source_included": false,
        });
    };
    let record = match journal.compiled_ir(digest) {
        Ok(Some(record)) => record,
        Ok(None) => match journal.compiled_ir_for_source_digest(digest) {
            Ok(Some(record)) => record,
            Ok(None) => return workflow_summary_from_source(journal, digest),
            Err(e) => {
                return serde_json::json!({
                    "digest": digest_hex(digest),
                    "compiled_ir": {"available": false, "reason": format!("compiled IR source-digest lookup error: {e}")},
                    "source_included": false,
                });
            }
        },
        Err(e) => {
            return serde_json::json!({
                "digest": digest_hex(digest),
                "compiled_ir": {"available": false, "reason": format!("compiled IR read error: {e}")},
                "source_included": false,
            });
        }
    };
    match decode_compiled_workflow_from_ir(&record.ir) {
        Ok(compiled) => compiled_workflow_summary(digest, &compiled),
        Err(_) => serde_json::json!({
            "digest": digest_hex(digest),
            "compiled_ir": {"available": false, "reason": "compiled IR decode failed"},
            "source_included": false,
        }),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DecodeCompiledWorkflowError {
    DirectWorkflowPartsDecode,
    DirectWorkflowCompile,
    AcceptedArtifactDecode,
    AcceptedArtifactWorkflowPartsDecode,
    AcceptedArtifactWorkflowCompile,
}

fn decode_compiled_workflow_from_ir(
    ir: &[u8],
) -> Result<vb_core::CompiledWorkflow, DecodeCompiledWorkflowError> {
    decode_direct_compiled_workflow(ir).or_else(|_| decode_accepted_artifact_workflow(ir))
}

fn decode_direct_compiled_workflow(
    ir: &[u8],
) -> Result<vb_core::CompiledWorkflow, DecodeCompiledWorkflowError> {
    let parts = postcard::from_bytes::<vb_core::WorkflowParts>(ir)
        .map_err(|_| DecodeCompiledWorkflowError::DirectWorkflowPartsDecode)?;
    vb_core::CompiledWorkflow::try_from_parts(parts)
        .map_err(|_| DecodeCompiledWorkflowError::DirectWorkflowCompile)
}

fn decode_accepted_artifact_workflow(
    ir: &[u8],
) -> Result<vb_core::CompiledWorkflow, DecodeCompiledWorkflowError> {
    let artifact = postcard::from_bytes::<vb_storage::admission::AcceptedArtifact>(ir)
        .map_err(|_| DecodeCompiledWorkflowError::AcceptedArtifactDecode)?;
    let parts = postcard::from_bytes::<vb_core::WorkflowParts>(&artifact.ir)
        .map_err(|_| DecodeCompiledWorkflowError::AcceptedArtifactWorkflowPartsDecode)?;
    vb_core::CompiledWorkflow::try_from_parts(parts)
        .map_err(|_| DecodeCompiledWorkflowError::AcceptedArtifactWorkflowCompile)
}

fn workflow_summary_from_source(
    journal: &vb_storage::FjallJournal,
    digest: vb_core::WorkflowDigest,
) -> Value {
    let source = match journal.workflow_source(digest) {
        Ok(Some(record)) => record.source,
        Ok(None) => {
            return serde_json::json!({
                "digest": digest_hex(digest),
                "compiled_ir": {"available": false, "reason": "compiled IR and workflow source not found"},
                "source_included": false,
            });
        }
        Err(e) => {
            return serde_json::json!({
                "digest": digest_hex(digest),
                "compiled_ir": {"available": false, "reason": format!("compiled IR not found; workflow source read error: {e}")},
                "source_included": false,
            });
        }
    };
    match vb_compile::compile_workflow(&source) {
        Ok(compiled) => compiled_workflow_summary(digest, &compiled),
        Err(e) => serde_json::json!({
            "digest": digest_hex(digest),
            "compiled_ir": {"available": false, "reason": format!("compiled IR not found; workflow source compile failed: {e}")},
            "source_included": false,
        }),
    }
}

fn compiled_workflow_summary(
    digest: vb_core::WorkflowDigest,
    compiled: &vb_core::CompiledWorkflow,
) -> Value {
    let nodes: Vec<Value> = (0..compiled.node_count())
        .filter_map(|raw| compiled_node_json(compiled, raw))
        .collect();
    serde_json::json!({
        "digest": digest_hex(digest),
        "compiled_ir": {
            "available": true,
            "name": compiled.name(),
            "entry": compiled.entry().get(),
            "node_count": compiled.node_count(),
            "slot_count": compiled.slot_count(),
            "resource_contract": compiled.resource_contract(),
            "nodes": nodes,
        },
        "referenced_actions": referenced_actions(compiled),
        "source_included": false,
    })
}

fn compiled_node_json(compiled: &vb_core::CompiledWorkflow, raw: u16) -> Option<Value> {
    let step = vb_core::StepIdx::new(raw);
    compiled.node(step).map(|node| {
        serde_json::json!({
            "step": raw,
            "name": compiled.step_name(step),
            "kind": node_kind_name(&node.kind),
            "output": node.output.slot().map(|slot| slot.get()),
            "next": node.routing.next.step().map(|next| next.get()),
        })
    })
}

fn referenced_actions(compiled: &vb_core::CompiledWorkflow) -> Vec<u32> {
    (0..compiled.node_count())
        .filter_map(|raw| compiled.node(vb_core::StepIdx::new(raw)))
        .filter_map(|node| match &node.kind {
            vb_core::workflow::CompiledNodeKind::Do { action, .. } => Some(u32::from(action.get())),
            _ => None,
        })
        .fold(Vec::<u32>::new(), push_unique_u32)
}

pub(crate) fn workflow_digest_from_events(
    events: &[vb_storage::JournalEvent],
) -> Option<vb_core::WorkflowDigest> {
    events.iter().find_map(|event| match event {
        vb_storage::JournalEvent::RunAccepted { workflow, .. } => Some(*workflow),
        _ => None,
    })
}

fn node_kind_name(kind: &vb_core::workflow::CompiledNodeKind) -> &'static str {
    match kind {
        vb_core::workflow::CompiledNodeKind::Nop => "Nop",
        vb_core::workflow::CompiledNodeKind::SetConst { .. } => "SetConst",
        vb_core::workflow::CompiledNodeKind::Copy { .. } => "Copy",
        vb_core::workflow::CompiledNodeKind::EvalExpr { .. } => "EvalExpr",
        vb_core::workflow::CompiledNodeKind::BuildObject { .. } => "BuildObject",
        vb_core::workflow::CompiledNodeKind::BuildList { .. } => "BuildList",
        vb_core::workflow::CompiledNodeKind::Do { .. } => "Do",
        vb_core::workflow::CompiledNodeKind::Choose { .. } => "Choose",
        vb_core::workflow::CompiledNodeKind::ChooseSlot { .. } => "ChooseSlot",
        vb_core::workflow::CompiledNodeKind::ForEachStart { .. } => "ForEachStart",
        vb_core::workflow::CompiledNodeKind::ForEachNext { .. } => "ForEachNext",
        vb_core::workflow::CompiledNodeKind::ForEachJoin { .. } => "ForEachJoin",
        vb_core::workflow::CompiledNodeKind::TogetherStart { .. } => "TogetherStart",
        vb_core::workflow::CompiledNodeKind::TogetherBranch { .. } => "TogetherBranch",
        vb_core::workflow::CompiledNodeKind::TogetherJoin { .. } => "TogetherJoin",
        vb_core::workflow::CompiledNodeKind::CollectStart { .. } => "CollectStart",
        vb_core::workflow::CompiledNodeKind::CollectPage { .. } => "CollectPage",
        vb_core::workflow::CompiledNodeKind::CollectNext { .. } => "CollectNext",
        vb_core::workflow::CompiledNodeKind::CollectFinish { .. } => "CollectFinish",
        vb_core::workflow::CompiledNodeKind::ReduceStart { .. } => "ReduceStart",
        vb_core::workflow::CompiledNodeKind::ReduceNext { .. } => "ReduceNext",
        vb_core::workflow::CompiledNodeKind::ReduceFinish { .. } => "ReduceFinish",
        vb_core::workflow::CompiledNodeKind::RepeatStart { .. } => "RepeatStart",
        vb_core::workflow::CompiledNodeKind::RepeatAttempt { .. } => "RepeatAttempt",
        vb_core::workflow::CompiledNodeKind::RepeatCheck { .. } => "RepeatCheck",
        vb_core::workflow::CompiledNodeKind::RepeatFinish { .. } => "RepeatFinish",
        vb_core::workflow::CompiledNodeKind::WaitUntil { .. } => "WaitUntil",
        vb_core::workflow::CompiledNodeKind::WaitEvent { .. } => "WaitEvent",
        vb_core::workflow::CompiledNodeKind::Ask { .. } => "Ask",
        vb_core::workflow::CompiledNodeKind::AskResume { .. } => "AskResume",
        vb_core::workflow::CompiledNodeKind::RetryCheck { .. } => "RetryCheck",
        vb_core::workflow::CompiledNodeKind::Jump { .. } => "Jump",
        vb_core::workflow::CompiledNodeKind::Finish { .. } => "Finish",
        vb_core::workflow::CompiledNodeKind::ErrorHandler { .. } => "ErrorHandler",
        _ => "Unknown",
    }
}
