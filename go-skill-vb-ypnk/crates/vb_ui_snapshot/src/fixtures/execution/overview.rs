#![forbid(unsafe_code)]

use crate::UiSnapshotError;
use alloc::{string::ToString, vec::Vec};
use vb_ui_model::{
    UiAppSnapshot, WorkflowDigest,
    run::{RunInspectionView, RunStatus, SlotDiffView, StepStateView, StepStatus},
    run::{RunSummaryView},
    system::StorageHealth,
    system::SystemStatusView,
    workflow::WorkflowGraphView,
    workflow::{WorkflowEdgeView, WorkflowNodeKind, WorkflowNodeView},
};

use super::super::{make_digest, DemoFixture};

pub fn execution_overview_fixture() -> Result<DemoFixture, UiSnapshotError> {
    Ok(DemoFixture {
        name: "execution_overview".to_string(),
        screen_kind: "ExecutionOverview".to_string(),
        app_snapshot: UiAppSnapshot {
            status: SystemStatusView {
                storage_health: StorageHealth::Healthy,
                writer_queue_depth: 3,
                journal_batch_healthy: true,
                snapshot_seq: Some(vb_ui_model::SeqNo::new(1440)),
                blob_store_ok: true,
                index_healthy: true,
                uptime_seconds: 86400,
                active_run_count: 2,
            },
            active_runs: vec![
                RunSummaryView {
                    run_id: vb_ui_model::RunId::new(1),
                    workflow_id: vb_ui_model::WorkflowId::new(1),
                    status: RunStatus::Running,
                    started_at: 1715000000,
                    finished_at: None,
                    step_count: 5,
                    event_count: 312,
                },
                RunSummaryView {
                    run_id: vb_ui_model::RunId::new(2),
                    workflow_id: vb_ui_model::WorkflowId::new(2),
                    status: RunStatus::Success,
                    started_at: 1714900000,
                    finished_at: Some(1714901200),
                    step_count: 8,
                    event_count: 789,
                },
            ]
            .into_boxed_slice(),
            selected_run: Some(RunInspectionView {
                run_id: vb_ui_model::RunId::new(1),
                workflow_id: vb_ui_model::WorkflowId::new(1),
                status: RunStatus::Running,
                started_at: 1715000000,
                finished_at: None,
                current_step: Some(vb_ui_model::StepIdx::new(2)),
                steps: vec![
                    StepStateView {
                        step_idx: vb_ui_model::StepIdx::new(0),
                        label: "FetchSource".to_string(),
                        status: StepStatus::Success,
                        entered_at: Some(1715000000),
                        exited_at: Some(1715000100),
                        error: None,
                    },
                    StepStateView {
                        step_idx: vb_ui_model::StepIdx::new(1),
                        label: "ValidateSchema".to_string(),
                        status: StepStatus::Success,
                        entered_at: Some(1715000100),
                        exited_at: Some(1715000200),
                        error: None,
                    },
                    StepStateView {
                        step_idx: vb_ui_model::StepIdx::new(2),
                        label: "TransformData".to_string(),
                        status: StepStatus::Running,
                        entered_at: Some(1715000200),
                        exited_at: None,
                        error: None,
                    },
                    StepStateView {
                        step_idx: vb_ui_model::StepIdx::new(3),
                        label: "LoadSink".to_string(),
                        status: StepStatus::Pending,
                        entered_at: None,
                        exited_at: None,
                        error: None,
                    },
                    StepStateView {
                        step_idx: vb_ui_model::StepIdx::new(4),
                        label: "Verify".to_string(),
                        status: StepStatus::Pending,
                        entered_at: None,
                        exited_at: None,
                        error: None,
                    },
                ],
                slot_diffs: vec![
                    SlotDiffView {
                        slot_idx: vb_ui_model::SlotIdx::new(0),
                        before: Some("bytes:1024".to_string()),
                        after: Some("bytes:2048".to_string()),
                        taint_before: vb_ui_model::Taint::Clean,
                        taint_after: vb_ui_model::Taint::Clean,
                    },
                    SlotDiffView {
                        slot_idx: vb_ui_model::SlotIdx::new(1),
                        before: Some("null".to_string()),
                        after: Some("bytes:512".to_string()),
                        taint_before: vb_ui_model::Taint::Clean,
                        taint_after: vb_ui_model::Taint::DerivedFromSecret,
                    },
                ],
            }),
            selected_workflow: Some(WorkflowGraphView {
                workflow_id: vb_ui_model::WorkflowId::new(1),
                workflow_digest: make_digest(0x11),
                nodes: vec![
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(0),
                        label: "FetchSource".to_string(),
                        kind: WorkflowNodeKind::Start,
                        input_slot_count: 0,
                        output_slot_count: 1,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(1),
                        label: "ValidateSchema".to_string(),
                        kind: WorkflowNodeKind::Sequence,
                        input_slot_count: 1,
                        output_slot_count: 1,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(2),
                        label: "TransformData".to_string(),
                        kind: WorkflowNodeKind::ForEach,
                        input_slot_count: 1,
                        output_slot_count: 1,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(3),
                        label: "LoadSink".to_string(),
                        kind: WorkflowNodeKind::Do,
                        input_slot_count: 1,
                        output_slot_count: 1,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(4),
                        label: "Verify".to_string(),
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
                node_x: vec![100.0, 300.0, 500.0, 700.0, 900.0],
                node_y: vec![300.0, 300.0, 300.0, 300.0, 300.0],
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
