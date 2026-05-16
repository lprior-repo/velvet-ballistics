#![forbid(unsafe_code)]

use crate::UiSnapshotError;
use alloc::string::ToString;
use vb_ui_model::{
    UiAppSnapshot, WorkflowDigest,
    run::RunStatus,
    system::StorageHealth,
    system::SystemStatusView,
    workflow::WorkflowGraphView,
    workflow::{WorkflowEdgeView, WorkflowNodeKind, WorkflowNodeView},
};

use super::super::{make_digest, DemoFixture};

pub fn workflow_graph_authoring_fixture() -> Result<DemoFixture, UiSnapshotError> {
    Ok(DemoFixture {
        name: "workflow_graph_authoring".to_string(),
        screen_kind: "WorkflowGraphAuthoring".to_string(),
        app_snapshot: UiAppSnapshot {
            status: SystemStatusView {
                storage_health: StorageHealth::Healthy,
                writer_queue_depth: 0,
                journal_batch_healthy: true,
                snapshot_seq: Some(vb_ui_model::SeqNo::new(0)),
                blob_store_ok: true,
                index_healthy: true,
                uptime_seconds: 3600,
                active_run_count: 0,
            },
            active_runs: [].into(),
            selected_run: None,
            selected_workflow: Some(WorkflowGraphView {
                workflow_id: vb_ui_model::WorkflowId::new(10),
                workflow_digest: make_digest(0x33),
                nodes: vec![
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(0),
                        label: "Source".to_string(),
                        kind: WorkflowNodeKind::Start,
                        input_slot_count: 0,
                        output_slot_count: 2,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(1),
                        label: "Branch".to_string(),
                        kind: WorkflowNodeKind::If,
                        input_slot_count: 1,
                        output_slot_count: 2,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(2),
                        label: "OnTrue".to_string(),
                        kind: WorkflowNodeKind::Do,
                        input_slot_count: 1,
                        output_slot_count: 1,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(3),
                        label: "OnFalse".to_string(),
                        kind: WorkflowNodeKind::Parallel,
                        input_slot_count: 1,
                        output_slot_count: 1,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(4),
                        label: "Merge".to_string(),
                        kind: WorkflowNodeKind::Sequence,
                        input_slot_count: 2,
                        output_slot_count: 1,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(5),
                        label: "Sink".to_string(),
                        kind: WorkflowNodeKind::Finish,
                        input_slot_count: 1,
                        output_slot_count: 0,
                    },
                ],
                edges: vec![
                    WorkflowEdgeView {
                        from_step: vb_ui_model::StepIdx::new(0),
                        to_step: vb_ui_model::StepIdx::new(1),
                        label: Some("data".to_string()),
                    },
                    WorkflowEdgeView {
                        from_step: vb_ui_model::StepIdx::new(1),
                        to_step: vb_ui_model::StepIdx::new(2),
                        label: Some("true".to_string()),
                    },
                    WorkflowEdgeView {
                        from_step: vb_ui_model::StepIdx::new(1),
                        to_step: vb_ui_model::StepIdx::new(3),
                        label: Some("false".to_string()),
                    },
                    WorkflowEdgeView {
                        from_step: vb_ui_model::StepIdx::new(2),
                        to_step: vb_ui_model::StepIdx::new(4),
                        label: None,
                    },
                    WorkflowEdgeView {
                        from_step: vb_ui_model::StepIdx::new(3),
                        to_step: vb_ui_model::StepIdx::new(4),
                        label: None,
                    },
                    WorkflowEdgeView {
                        from_step: vb_ui_model::StepIdx::new(4),
                        to_step: vb_ui_model::StepIdx::new(5),
                        label: None,
                    },
                ],
                node_x: vec![100.0, 300.0, 200.0, 400.0, 300.0, 500.0],
                node_y: vec![300.0, 300.0, 150.0, 450.0, 300.0, 300.0],
            }),
            verification: None,
            replay: None,
            incident: None,
            actions: [].into(),
            storage: None,
            ai_context: None,
        },
    })
}
