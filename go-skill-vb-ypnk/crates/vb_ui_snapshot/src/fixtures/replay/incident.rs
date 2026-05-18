#![forbid(unsafe_code)]

use crate::UiSnapshotError;
use alloc::{boxed::Box, string::ToString, vec};
use vb_ui_model::{
    UiAppSnapshot, WorkflowDigest,
    incident::IncidentReportView,
    incident::IncidentSeverity,
    run::RunStatus,
    run::RunSummaryView,
    system::StorageHealth,
    system::SystemStatusView,
    workflow::WorkflowGraphView,
    workflow::{WorkflowEdgeView, WorkflowNodeKind, WorkflowNodeView},
};

use super::super::{make_digest, DemoFixture};

pub fn incident_failure_fixture() -> Result<DemoFixture, UiSnapshotError> {
    Ok(DemoFixture {
        name: "incident_failure".to_string(),
        screen_kind: "IncidentFailureConsole".to_string(),
        app_snapshot: UiAppSnapshot {
            status: SystemStatusView {
                storage_health: StorageHealth::Degraded,
                writer_queue_depth: 12,
                journal_batch_healthy: false,
                snapshot_seq: Some(vb_ui_model::SeqNo::new(333)),
                blob_store_ok: true,
                index_healthy: false,
                uptime_seconds: 99999,
                active_run_count: 5,
            },
            active_runs: vec![RunSummaryView {
                run_id: vb_ui_model::RunId::new(400),
                workflow_id: vb_ui_model::WorkflowId::new(400),
                status: RunStatus::Failure,
                started_at: 1715300000,
                finished_at: Some(1715300300),
                step_count: 4,
                event_count: 200,
            }]
            .into_boxed_slice(),
            selected_run: None,
            selected_workflow: Some(WorkflowGraphView {
                workflow_id: vb_ui_model::WorkflowId::new(400),
                workflow_digest: make_digest(0x77),
                nodes: vec![
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(0),
                        label: "Start".to_string(),
                        kind: WorkflowNodeKind::Start,
                        input_slot_count: 0,
                        output_slot_count: 1,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(1),
                        label: "Load".to_string(),
                        kind: WorkflowNodeKind::Do,
                        input_slot_count: 1,
                        output_slot_count: 1,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(2),
                        label: "Transform".to_string(),
                        kind: WorkflowNodeKind::ForEach,
                        input_slot_count: 1,
                        output_slot_count: 1,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(3),
                        label: "Save".to_string(),
                        kind: WorkflowNodeKind::Do,
                        input_slot_count: 1,
                        output_slot_count: 1,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(4),
                        label: "End".to_string(),
                        kind: WorkflowNodeKind::Finish,
                        input_slot_count: 1,
                        output_slot_count: 0,
                    },
                ],
                edges: vec![
                    WorkflowEdgeView {
                        from_step: vb_ui_model::StepIdx::new(0),
                        to_step: vb_ui_model::StepIdx::new(1),
                        label: None,
                    },
                    WorkflowEdgeView {
                        from_step: vb_ui_model::StepIdx::new(1),
                        to_step: vb_ui_model::StepIdx::new(2),
                        label: None,
                    },
                    WorkflowEdgeView {
                        from_step: vb_ui_model::StepIdx::new(2),
                        to_step: vb_ui_model::StepIdx::new(3),
                        label: None,
                    },
                    WorkflowEdgeView {
                        from_step: vb_ui_model::StepIdx::new(3),
                        to_step: vb_ui_model::StepIdx::new(4),
                        label: None,
                    },
                ],
                node_x: vec![100.0, 250.0, 400.0, 550.0, 700.0],
                node_y: vec![300.0; 5],
            }),
            verification: None,
            replay: None,
            incident: Some(IncidentReportView {
                run_id: vb_ui_model::RunId::new(400),
                failure_step: vb_ui_model::StepIdx::new(2),
                failure_action: vb_ui_model::ActionId::new(5),
                failure_code: "TRANSFORM_ERR_NULL_INPUT".to_string(),
                attempt: 2,
                timestamp: 1715300200,
                severity: IncidentSeverity::Critical,
                safe_to_retry: true,
                idempotency_key_required: true,
                strict_durability: false,
                replay_safe: true,
                repair_hints: vec![
                    "Ensure input stream filters null values".to_string(),
                    "Add null-check guard before Transform step".to_string(),
                    "Consider adding a fallback default value".to_string(),
                ],
                evidence_chain: vb_ui_model::incident::EvidenceChain {
                    scheduled_durable: true,
                    completion_durable: true,
                    side_effect_certainty: 0.99,
                    journal_tail: Some(vb_ui_model::SeqNo::new(333)),
                },
            }),
            actions: [].into(),
            storage: None,
            ai_context: None,
        },
    })
}
