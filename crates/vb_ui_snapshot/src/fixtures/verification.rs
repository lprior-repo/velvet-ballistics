#![forbid(unsafe_code)]

use crate::UiSnapshotError;
use alloc::{boxed::Box, string::ToString};
use vb_ui_model::{
    UiAppSnapshot, WorkflowDigest,
    run::RunStatus,
    system::StorageHealth,
    system::SystemStatusView,
    verify::VerificationCertificate,
    verify::VerificationReportView,
    workflow::WorkflowGraphView,
    workflow::{WorkflowEdgeView, WorkflowNodeKind, WorkflowNodeView},
};

use super::{make_digest, DemoFixture};

pub fn verification_certificate_fixture() -> Result<DemoFixture, UiSnapshotError> {
    Ok(DemoFixture {
        name: "verification_certificate".to_string(),
        screen_kind: "VerificationCertificate".to_string(),
        app_snapshot: UiAppSnapshot {
            status: SystemStatusView {
                storage_health: StorageHealth::Healthy,
                writer_queue_depth: 0,
                journal_batch_healthy: true,
                snapshot_seq: Some(vb_ui_model::SeqNo::new(500)),
                blob_store_ok: true,
                index_healthy: true,
                uptime_seconds: 43200,
                active_run_count: 0,
            },
            active_runs: [].into(),
            selected_run: None,
            selected_workflow: Some(WorkflowGraphView {
                workflow_id: vb_ui_model::WorkflowId::new(200),
                workflow_digest: make_digest(0x55),
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
                        label: "Process".to_string(),
                        kind: WorkflowNodeKind::Sequence,
                        input_slot_count: 1,
                        output_slot_count: 1,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(2),
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
                ],
                node_x: vec![200.0, 400.0, 600.0],
                node_y: vec![300.0; 3],
            }),
            verification: Some(VerificationReportView {
                workflow_id: vb_ui_model::WorkflowId::new(200),
                workflow_digest: make_digest(0x55),
                passed: true,
                warnings: vec!["No retry policy on Process step".to_string()],
                certificate: VerificationCertificate {
                    structure: true,
                    boundedness: true,
                    resources: true,
                    taint: true,
                    action_policy: true,
                    durability: true,
                    idempotency: true,
                    capability: true,
                },
                gate_results: vec![
                    vb_ui_model::verify::GateResult {
                        name: "Structure".to_string(),
                        passed: true,
                        detail: None,
                    },
                    vb_ui_model::verify::GateResult {
                        name: "Boundedness".to_string(),
                        passed: true,
                        detail: None,
                    },
                    vb_ui_model::verify::GateResult {
                        name: "Resources".to_string(),
                        passed: true,
                        detail: None,
                    },
                    vb_ui_model::verify::GateResult {
                        name: "Taint".to_string(),
                        passed: true,
                        detail: None,
                    },
                    vb_ui_model::verify::GateResult {
                        name: "ActionPolicy".to_string(),
                        passed: true,
                        detail: None,
                    },
                    vb_ui_model::verify::GateResult {
                        name: "Durability".to_string(),
                        passed: true,
                        detail: None,
                    },
                    vb_ui_model::verify::GateResult {
                        name: "Idempotency".to_string(),
                        passed: true,
                        detail: None,
                    },
                    vb_ui_model::verify::GateResult {
                        name: "Capability".to_string(),
                        passed: true,
                        detail: None,
                    },
                ],
            }),
            replay: None,
            incident: None,
            actions: [].into(),
            storage: None,
            ai_context: None,
        },
    })
}
