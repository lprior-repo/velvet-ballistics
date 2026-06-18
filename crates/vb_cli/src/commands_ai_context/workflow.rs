//! Workflow digest extraction and compiled-IR decoding for AI context output.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

use super::node_rendering::{compiled_node_json, referenced_actions};

pub(super) fn workflow_digest_from_events(
    events: &[vb_storage::JournalEvent],
) -> Option<vb_core::WorkflowDigest> {
    events.iter().find_map(|event| match event {
        vb_storage::JournalEvent::RunAccepted { workflow, .. } => Some(*workflow),
        _ => None,
    })
}

pub(super) fn ai_workflow_summary(
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
        Ok(None) => return workflow_summary_from_source(journal, digest),
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

// ── Compiled-IR decoding ──────────────────────────────────────────────

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

// ── Workflow summaries ────────────────────────────────────────────────

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

fn digest_hex(digest: vb_core::WorkflowDigest) -> String {
    digest
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
