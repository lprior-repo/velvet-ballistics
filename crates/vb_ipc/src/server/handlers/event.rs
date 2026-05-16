#![forbid(unsafe_code)]
//! Event and workflow graph handlers.

use std::collections::{HashSet, VecDeque};

use vb_core::workflow::{CompiledNode, CompiledNodeKind, WorkflowParts};
use vb_runtime::runtime::Runtime;

use super::query::{decode_payload, sanitize_runtime_error};
use crate::server::{IpcResponse, WorkflowResolutionError, WorkflowResolver};
use crate::{IpcPayload, NodeDescriptor, EdgeDescriptor, TaintPathWire};

const MAX_TAINT_PATH_ENTRIES: usize = 65536;
const MAX_VALIDATION_DETAIL_LEN: usize = 512;
const MAX_WORKFLOW_GRAPH_NODES: usize = 8192;

pub fn sanitize_validation_detail(detail: String) -> String {
    let truncated = if detail.len() <= MAX_VALIDATION_DETAIL_LEN {
        detail
    } else {
        let mut s: String = detail.chars().take(MAX_VALIDATION_DETAIL_LEN).collect();
        s.push_str("...");
        s
    };
    truncated
        .replace("/home/", "<redacted>/")
        .replace("/etc/", "<redacted>/")
        .replace("/var/", "<redacted>/")
        .replace("/tmp/", "<redacted>/")
        .replace("/usr/", "<redacted>/")
        .replace("C:\\", "<redacted>\\")
        .replace("\\\\", "<redacted>\\\\")
}

pub fn handle_verify_workflow(
    payload: &[u8],
    resolver: Option<&mut dyn WorkflowResolver>,
) -> IpcResponse {
    let Ok(IpcPayload::VerifyWorkflow { digest }) = decode_payload::<IpcPayload>(payload) else {
        return IpcResponse::BadRequest;
    };
    let Some(resolver) = resolver else {
        return IpcResponse::WorkflowResolutionRequired;
    };
    let workflow = match resolver.resolve_workflow(digest) {
        Ok(w) => w,
        Err(WorkflowResolutionError::Required) => return IpcResponse::WorkflowResolutionRequired,
        Err(WorkflowResolutionError::NotFound | WorkflowResolutionError::InvalidArtifact) => {
            return IpcResponse::WorkflowResolutionUnsupported;
        }
    };
    if workflow.digest() != digest {
        return IpcResponse::WorkflowDigestMismatch;
    }
    let parts = workflow.to_parts();
    let gate_results: Vec<(&str, Result<(), vb_validate::ValidationError>)> = vec![
        (
            "gate_07_expression_stack_depth",
            vb_validate::gates::validate_gate_07_expression_stack_depth(&parts),
        ),
        (
            "gate_08_accessor_path_segments",
            vb_validate::gates::validate_gate_08_accessor_path_segments(&parts),
        ),
        (
            "gate_09_slot_references",
            vb_validate::gates::validate_gate_09_slot_references(&parts),
        ),
        (
            "gate_10_node_kind_specific",
            vb_validate::gates::validate_gate_10_node_kind_specific(&parts),
        ),
        (
            "gate_11_loop_body_graph",
            vb_validate::gates::validate_gate_11_loop_body_graph(&parts),
        ),
        (
            "gate_12_action_contract_completeness",
            vb_validate::gates::validate_gate_12_action_contract_completeness(&parts, &[]),
        ),
        (
            "gate_13_no_slot_cycles",
            vb_validate::gates::validate_gate_13_no_slot_cycles(&parts),
        ),
        (
            "gate_14_slot_type_consistency",
            vb_validate::gates::validate_gate_14_slot_type_consistency(&parts),
        ),
        (
            "gate_15_determinism_proof",
            vb_validate::gates::validate_gate_15_determinism_proof(&parts),
        ),
    ];
    let total_checks = match u32::try_from(gate_results.len()) {
        Ok(v) => v,
        Err(_) => {
            return IpcResponse::RuntimeError {
                message: String::from("gate results count exceeds u32::MAX"),
            };
        }
    };
    let mut pass_count: u32 = 0;
    let mut fail_count: u32 = 0;
    let mut certificates: Vec<crate::CertificateWire> = Vec::new();
    for (kind, result) in gate_results {
        match result {
            Ok(()) => {
                pass_count = pass_count.saturating_add(1);
                certificates.push(crate::CertificateWire {
                    kind: kind.to_owned(),
                    status: String::from("Pass"),
                    details: String::new(),
                });
            }
            Err(err) => {
                fail_count = fail_count.saturating_add(1);
                certificates.push(crate::CertificateWire {
                    kind: kind.to_owned(),
                    status: String::from("Fail"),
                    details: sanitize_validation_detail(err.to_string()),
                });
            }
        }
    }
    IpcResponse::VerifyWorkflow {
        result: crate::VerificationResult {
            certificates,
            total_checks,
            pass_count,
            fail_count,
        },
    }
}

fn node_kind_label(kind: &vb_core::workflow::CompiledNodeKind) -> &'static str {
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
        vb_core::workflow::CompiledNodeKind::ErrorHandler { .. } => "ErrorHandler",
        vb_core::workflow::CompiledNodeKind::Jump { .. } => "Jump",
        vb_core::workflow::CompiledNodeKind::Finish { .. } => "Finish",
    }
}

fn collect_edges_from_node(
    step: u16,
    kind: &vb_core::workflow::CompiledNodeKind,
    edges: &mut Vec<EdgeDescriptor>,
) {
    match kind {
        vb_core::workflow::CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => {
            for (i, branch) in branches.iter().enumerate() {
                edges.push(EdgeDescriptor {
                    from: step,
                    to: branch.target.get(),
                    label: Some(format!("branch_{i}")),
                    edge_type: String::from("branch"),
                });
            }
            if let Some(fallback) = otherwise {
                edges.push(EdgeDescriptor {
                    from: step,
                    to: fallback.get(),
                    label: Some(String::from("otherwise")),
                    edge_type: String::from("branch"),
                });
            }
        }
        vb_core::workflow::CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => {
            for (i, branch) in branches.iter().enumerate() {
                edges.push(EdgeDescriptor {
                    from: step,
                    to: branch.target.get(),
                    label: Some(format!("branch_{i}")),
                    edge_type: String::from("branch"),
                });
            }
            if let Some(fallback) = otherwise {
                edges.push(EdgeDescriptor {
                    from: step,
                    to: fallback.get(),
                    label: Some(String::from("otherwise")),
                    edge_type: String::from("branch"),
                });
            }
        }
        vb_core::workflow::CompiledNodeKind::ForEachStart { body, done, .. }
        | vb_core::workflow::CompiledNodeKind::ForEachNext { body, done, .. } => {
            edges.push(EdgeDescriptor {
                from: step,
                to: body.get(),
                label: Some(String::from("body")),
                edge_type: String::from("loop_body"),
            });
            edges.push(EdgeDescriptor {
                from: step,
                to: done.get(),
                label: Some(String::from("done")),
                edge_type: String::from("loop_exit"),
            });
        }
        vb_core::workflow::CompiledNodeKind::TogetherStart { branches, join, .. } => {
            for (i, branch_step) in branches.iter().enumerate() {
                edges.push(EdgeDescriptor {
                    from: step,
                    to: branch_step.get(),
                    label: Some(format!("branch_{i}")),
                    edge_type: String::from("parallel_branch"),
                });
            }
            edges.push(EdgeDescriptor {
                from: step,
                to: join.get(),
                label: Some(String::from("join")),
                edge_type: String::from("parallel_join"),
            });
        }
        vb_core::workflow::CompiledNodeKind::CollectStart { body, done, .. }
        | vb_core::workflow::CompiledNodeKind::CollectPage { body, done, .. }
        | vb_core::workflow::CompiledNodeKind::CollectNext { body, done, .. } => {
            edges.push(EdgeDescriptor {
                from: step,
                to: body.get(),
                label: Some(String::from("body")),
                edge_type: String::from("loop_body"),
            });
            edges.push(EdgeDescriptor {
                from: step,
                to: done.get(),
                label: Some(String::from("done")),
                edge_type: String::from("loop_exit"),
            });
        }
        vb_core::workflow::CompiledNodeKind::ReduceStart { body, done, .. }
        | vb_core::workflow::CompiledNodeKind::ReduceNext { body, done, .. } => {
            edges.push(EdgeDescriptor {
                from: step,
                to: body.get(),
                label: Some(String::from("body")),
                edge_type: String::from("loop_body"),
            });
            edges.push(EdgeDescriptor {
                from: step,
                to: done.get(),
                label: Some(String::from("done")),
                edge_type: String::from("loop_exit"),
            });
        }
        vb_core::workflow::CompiledNodeKind::RepeatStart { body, done, .. }
        | vb_core::workflow::CompiledNodeKind::RepeatAttempt { body, done, .. } => {
            edges.push(EdgeDescriptor {
                from: step,
                to: body.get(),
                label: Some(String::from("body")),
                edge_type: String::from("loop_body"),
            });
            edges.push(EdgeDescriptor {
                from: step,
                to: done.get(),
                label: Some(String::from("done")),
                edge_type: String::from("loop_exit"),
            });
        }
        vb_core::workflow::CompiledNodeKind::RepeatCheck { done, .. } => {
            edges.push(EdgeDescriptor {
                from: step,
                to: done.get(),
                label: Some(String::from("done")),
                edge_type: String::from("loop_exit"),
            });
        }
        vb_core::workflow::CompiledNodeKind::ErrorHandler { body, handler, .. } => {
            edges.push(EdgeDescriptor {
                from: step,
                to: body.get(),
                label: Some(String::from("body")),
                edge_type: String::from("fallthrough"),
            });
            edges.push(EdgeDescriptor {
                from: step,
                to: handler.get(),
                label: Some(String::from("handler")),
                edge_type: String::from("error_handler"),
            });
        }
        vb_core::workflow::CompiledNodeKind::Jump { target } => {
            edges.push(EdgeDescriptor {
                from: step,
                to: target.get(),
                label: None,
                edge_type: String::from("jump"),
            });
        }
        vb_core::workflow::CompiledNodeKind::Nop
        | vb_core::workflow::CompiledNodeKind::SetConst { .. }
        | vb_core::workflow::CompiledNodeKind::Copy { .. }
        | vb_core::workflow::CompiledNodeKind::EvalExpr { .. }
        | vb_core::workflow::CompiledNodeKind::BuildObject { .. }
        | vb_core::workflow::CompiledNodeKind::BuildList { .. }
        | vb_core::workflow::CompiledNodeKind::Do { .. }
        | vb_core::workflow::CompiledNodeKind::ForEachJoin { .. }
        | vb_core::workflow::CompiledNodeKind::TogetherBranch { .. }
        | vb_core::workflow::CompiledNodeKind::TogetherJoin { .. }
        | vb_core::workflow::CompiledNodeKind::CollectFinish { .. }
        | vb_core::workflow::CompiledNodeKind::ReduceFinish { .. }
        | vb_core::workflow::CompiledNodeKind::RepeatFinish { .. }
        | vb_core::workflow::CompiledNodeKind::WaitUntil { .. }
        | vb_core::workflow::CompiledNodeKind::WaitEvent { .. }
        | vb_core::workflow::CompiledNodeKind::Ask { .. }
        | vb_core::workflow::CompiledNodeKind::AskResume { .. }
        | vb_core::workflow::CompiledNodeKind::RetryCheck { .. }
        | vb_core::workflow::CompiledNodeKind::Finish { .. } => {}
    }
}

pub fn handle_get_workflow_graph(
    payload: &[u8],
    resolver: Option<&mut dyn WorkflowResolver>,
) -> IpcResponse {
    let Ok(IpcPayload::GetWorkflowGraph { digest }) = decode_payload::<IpcPayload>(payload) else {
        return IpcResponse::BadRequest;
    };

    let Some(resolver) = resolver else {
        return IpcResponse::WorkflowResolutionRequired;
    };

    let workflow = match resolver.resolve_workflow(digest) {
        Ok(w) => w,
        Err(WorkflowResolutionError::Required) => return IpcResponse::WorkflowResolutionRequired,
        Err(WorkflowResolutionError::NotFound | WorkflowResolutionError::InvalidArtifact) => {
            return IpcResponse::WorkflowResolutionUnsupported;
        }
    };

    if workflow.digest() != digest {
        return IpcResponse::WorkflowDigestMismatch;
    }

    let node_count = workflow.node_count();
    let capped_node_count = node_count.min(u16::try_from(MAX_WORKFLOW_GRAPH_NODES).unwrap_or(u16::MAX));
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    let mut idx: u16 = 0;
    while idx < capped_node_count {
        let step = vb_core::ids::StepIdx::new(idx);
        let Some(compiled_node) = workflow.node(step) else {
            break;
        };

        let title = workflow
            .step_name(step)
            .map(String::from)
            .or_else(|| compiled_node.output.map(|s| format!("slot_{}", s.get())))
            .unwrap_or_else(|| {
                let kind_str = node_kind_label(&compiled_node.kind);
                format!("{kind_str}_{idx}")
            });

        let next = compiled_node.next.map(|n| n.get());

        if let Some(target) = next {
            edges.push(EdgeDescriptor {
                from: idx,
                to: target,
                label: None,
                edge_type: String::from("fallthrough"),
            });
        }

        collect_edges_from_node(idx, &compiled_node.kind, &mut edges);

        nodes.push(NodeDescriptor {
            step_idx: idx,
            kind: String::from(node_kind_label(&compiled_node.kind)),
            next,
            title,
        });

        idx = idx.saturating_add(1);
    }

    IpcResponse::WorkflowGraph { nodes, edges }
}

pub fn handle_get_taint_report(
    payload: &[u8],
    resolver: Option<&mut dyn WorkflowResolver>,
) -> IpcResponse {
    let Ok(IpcPayload::GetTaintReport { digest }) = decode_payload::<IpcPayload>(payload) else {
        return IpcResponse::BadRequest;
    };

    let Some(resolver) = resolver else {
        return IpcResponse::WorkflowResolutionRequired;
    };

    let workflow = match resolver.resolve_workflow(digest) {
        Ok(w) => w,
        Err(WorkflowResolutionError::Required) => return IpcResponse::WorkflowResolutionRequired,
        Err(WorkflowResolutionError::NotFound | WorkflowResolutionError::InvalidArtifact) => {
            return IpcResponse::WorkflowResolutionUnsupported;
        }
    };

    if workflow.digest() != digest {
        return IpcResponse::WorkflowDigestMismatch;
    }

    let parts = workflow.to_parts();

    let sources: Vec<u16> = parts
        .nodes
        .iter()
        .filter(|node| {
            matches!(
                node.kind,
                vb_core::workflow::CompiledNodeKind::WaitEvent { .. }
                    | vb_core::workflow::CompiledNodeKind::Ask { .. }
            )
        })
        .map(|node| node.id.get())
        .collect();

    let sinks: Vec<u16> = parts
        .nodes
        .iter()
        .filter(|node| {
            matches!(
                node.kind,
                vb_core::workflow::CompiledNodeKind::Finish { .. }
            )
        })
        .map(|node| node.id.get())
        .collect();

    let sink_set: HashSet<u16> = sinks.iter().copied().collect();
    let node_count = parts.nodes.len();
    let mut paths: Vec<TaintPathWire> = Vec::new();
    let mut any_source_reaches_sink = false;

    for source_idx in &sources {
        let reachable = bfs_forward(&parts, *source_idx, node_count);
        let reaches_sink = reachable.iter().any(|step| sink_set.contains(step));

        if reaches_sink {
            any_source_reaches_sink = true;
        }

        let status_str = if reaches_sink {
            String::from("dangerous")
        } else {
            String::from("warning")
        };

        for step in &reachable {
            if paths.len() >= MAX_TAINT_PATH_ENTRIES {
                break;
            }
            paths.push(TaintPathWire {
                from: *source_idx,
                to: *step,
                status: status_str.clone(),
            });
        }

        if paths.len() >= MAX_TAINT_PATH_ENTRIES {
            break;
        }
    }

    IpcResponse::TaintReport {
        sources,
        sinks,
        finish_safe: !any_source_reaches_sink,
        paths,
    }
}

fn bfs_forward(
    parts: &vb_core::workflow::WorkflowParts,
    start: u16,
    node_count: usize,
) -> Vec<u16> {
    let start_step = vb_core::ids::StepIdx::new(start);
    let mut visited = HashSet::new();
    visited.insert(start);
    let mut result = Vec::new();
    let mut queue = VecDeque::new();

    if let Some(node) = parts.nodes.get(start_step.as_usize()) {
        enqueue_successors(node, node_count, &mut visited, &mut queue);
    }

    while let Some(current) = queue.pop_front() {
        result.push(current);

        let current_step = vb_core::ids::StepIdx::new(current);
        if let Some(node) = parts.nodes.get(current_step.as_usize()) {
            enqueue_successors(node, node_count, &mut visited, &mut queue);
        }
    }

    result
}

pub(crate) fn all_successors(kind: &vb_core::workflow::CompiledNodeKind) -> Vec<u16> {
    let mut succs = Vec::new();
    match kind {
        vb_core::workflow::CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => {
            for branch in branches {
                succs.push(branch.target.get());
            }
            if let Some(fallback) = otherwise {
                succs.push(fallback.get());
            }
        }
        vb_core::workflow::CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => {
            for branch in branches {
                succs.push(branch.target.get());
            }
            if let Some(fallback) = otherwise {
                succs.push(fallback.get());
            }
        }
        vb_core::workflow::CompiledNodeKind::ForEachStart { body, done, .. }
        | vb_core::workflow::CompiledNodeKind::ForEachNext { body, done, .. } => {
            succs.push(body.get());
            succs.push(done.get());
        }
        vb_core::workflow::CompiledNodeKind::TogetherStart { branches, join, .. } => {
            for branch_step in branches {
                succs.push(branch_step.get());
            }
            succs.push(join.get());
        }
        vb_core::workflow::CompiledNodeKind::CollectStart { body, done, .. }
        | vb_core::workflow::CompiledNodeKind::CollectPage { body, done, .. }
        | vb_core::workflow::CompiledNodeKind::CollectNext { body, done, .. } => {
            succs.push(body.get());
            succs.push(done.get());
        }
        vb_core::workflow::CompiledNodeKind::ReduceStart { body, done, .. }
        | vb_core::workflow::CompiledNodeKind::ReduceNext { body, done, .. } => {
            succs.push(body.get());
            succs.push(done.get());
        }
        vb_core::workflow::CompiledNodeKind::RepeatStart { body, done, .. }
        | vb_core::workflow::CompiledNodeKind::RepeatAttempt { body, done, .. } => {
            succs.push(body.get());
            succs.push(done.get());
        }
        vb_core::workflow::CompiledNodeKind::RepeatCheck { done, .. } => {
            succs.push(done.get());
        }
        vb_core::workflow::CompiledNodeKind::ErrorHandler { body, handler, .. } => {
            succs.push(body.get());
            succs.push(handler.get());
        }
        vb_core::workflow::CompiledNodeKind::Jump { target } => {
            succs.push(target.get());
        }
        vb_core::workflow::CompiledNodeKind::Nop
        | vb_core::workflow::CompiledNodeKind::SetConst { .. }
        | vb_core::workflow::CompiledNodeKind::Copy { .. }
        | vb_core::workflow::CompiledNodeKind::EvalExpr { .. }
        | vb_core::workflow::CompiledNodeKind::BuildObject { .. }
        | vb_core::workflow::CompiledNodeKind::BuildList { .. }
        | vb_core::workflow::CompiledNodeKind::Do { .. }
        | vb_core::workflow::CompiledNodeKind::ForEachJoin { .. }
        | vb_core::workflow::CompiledNodeKind::TogetherBranch { .. }
        | vb_core::workflow::CompiledNodeKind::TogetherJoin { .. }
        | vb_core::workflow::CompiledNodeKind::CollectFinish { .. }
        | vb_core::workflow::CompiledNodeKind::ReduceFinish { .. }
        | vb_core::workflow::CompiledNodeKind::RepeatFinish { .. }
        | vb_core::workflow::CompiledNodeKind::WaitUntil { .. }
        | vb_core::workflow::CompiledNodeKind::WaitEvent { .. }
        | vb_core::workflow::CompiledNodeKind::Ask { .. }
        | vb_core::workflow::CompiledNodeKind::AskResume { .. }
        | vb_core::workflow::CompiledNodeKind::RetryCheck { .. }
        | vb_core::workflow::CompiledNodeKind::Finish { .. } => {}
    }
    succs
}

fn enqueue_successors(
    node: &vb_core::workflow::CompiledNode,
    node_count: usize,
    visited: &mut HashSet<u16>,
    queue: &mut VecDeque<u16>,
) {
    if let Some(next) = node.next {
        let next_u16 = next.get();
        let next_usize = next.as_usize();
        if next_usize < node_count && visited.insert(next_u16) {
            queue.push_back(next_u16);
        }
    }

    for succ in all_successors(&node.kind) {
        let succ_usize = usize::from(succ);
        if succ_usize < node_count && visited.insert(succ) {
            queue.push_back(succ);
        }
    }
}
