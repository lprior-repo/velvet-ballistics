#![forbid(unsafe_code)]

use crate::UiSnapshotError;
use alloc::{string::ToString, vec};
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

pub fn execution_details_fixture() -> Result<DemoFixture, UiSnapshotError> {
    Ok(DemoFixture {
        name: "execution_details".to_string(),
        screen_kind: "ExecutionDetailsGraph".to_string(),
        app_snapshot: UiAppSnapshot {
            status: SystemStatusView {
                storage_health: StorageHealth::Healthy,
                writer_queue_depth: 7,
                journal_batch_healthy: true,
                snapshot_seq: Some(vb_ui_model::SeqNo::new(9876)),
                blob_store_ok: true,
                index_healthy: true,
                uptime_seconds: 172800,
                active_run_count: 3,
            },
            active_runs: vec![RunSummaryView {
                run_id: vb_ui_model::RunId::new(100),
                workflow_id: vb_ui_model::WorkflowId::new(100),
                status: RunStatus::Running,
                started_at: 1715100000,
                finished_at: None,
                step_count: 12,
                event_count: 4500,
            }]
            .into_boxed_slice(),
            selected_run: Some(RunInspectionView {
                run_id: vb_ui_model::RunId::new(100),
                workflow_id: vb_ui_model::WorkflowId::new(100),
                status: RunStatus::Running,
                started_at: 1715100000,
                finished_at: None,
                current_step: Some(vb_ui_model::StepIdx::new(6)),
                steps: vec![
                    StepStateView {
                        step_idx: vb_ui_model::StepIdx::new(0),
                        label: "Init".to_string(),
                        status: StepStatus::Success,
                        entered_at: Some(1715100000),
                        exited_at: Some(1715100010),
                        error: None,
                    },
                    StepStateView {
                        step_idx: vb_ui_model::StepIdx::new(1),
                        label: "AcquireLock".to_string(),
                        status: StepStatus::Success,
                        entered_at: Some(1715100010),
                        exited_at: Some(1715100050),
                        error: None,
                    },
                    StepStateView {
                        step_idx: vb_ui_model::StepIdx::new(2),
                        label: "FetchRecords".to_string(),
                        status: StepStatus::Success,
                        entered_at: Some(1715100050),
                        exited_at: Some(1715100100),
                        error: None,
                    },
                    StepStateView {
                        step_idx: vb_ui_model::StepIdx::new(3),
                        label: "ProcessBatch".to_string(),
                        status: StepStatus::Success,
                        entered_at: Some(1715100100),
                        exited_at: Some(1715100300),
                        error: None,
                    },
                    StepStateView {
                        step_idx: vb_ui_model::StepIdx::new(4),
                        label: "ValidateOutput".to_string(),
                        status: StepStatus::Success,
                        entered_at: Some(1715100300),
                        exited_at: Some(1715100350),
                        error: None,
                    },
                    StepStateView {
                        step_idx: vb_ui_model::StepIdx::new(5),
                        label: "WriteResults".to_string(),
                        status: StepStatus::Success,
                        entered_at: Some(1715100350),
                        exited_at: Some(1715100400),
                        error: None,
                    },
                    StepStateView {
                        step_idx: vb_ui_model::StepIdx::new(6),
                        label: "EmitCertificate".to_string(),
                        status: StepStatus::Running,
                        entered_at: Some(1715100400),
                        exited_at: None,
                        error: None,
                    },
                    StepStateView {
                        step_idx: vb_ui_model::StepIdx::new(7),
                        label: "Notify".to_string(),
                        status: StepStatus::Pending,
                        entered_at: None,
                        exited_at: None,
                        error: None,
                    },
                    StepStateView {
                        step_idx: vb_ui_model::StepIdx::new(8),
                        label: "ReleaseLock".to_string(),
                        status: StepStatus::Pending,
                        entered_at: None,
                        exited_at: None,
                        error: None,
                    },
                ],
                slot_diffs: vec![
                    SlotDiffView {
                        slot_idx: vb_ui_model::SlotIdx::new(0),
                        before: Some("count:0".to_string()),
                        after: Some("count:50000".to_string()),
                        taint_before: vb_ui_model::Taint::Clean,
                        taint_after: vb_ui_model::Taint::Clean,
                    },
                    SlotDiffView {
                        slot_idx: vb_ui_model::SlotIdx::new(1),
                        before: Some("null".to_string()),
                        after: Some("checksum:abc123".to_string()),
                        taint_before: vb_ui_model::Taint::Clean,
                        taint_after: vb_ui_model::Taint::Clean,
                    },
                ],
            }),
            selected_workflow: Some(WorkflowGraphView {
                workflow_id: vb_ui_model::WorkflowId::new(100),
                workflow_digest: make_digest(0x44),
                nodes: vec![
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(0),
                        label: "Init".to_string(),
                        kind: WorkflowNodeKind::Start,
                        input_slot_count: 0,
                        output_slot_count: 1,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(1),
                        label: "AcquireLock".to_string(),
                        kind: WorkflowNodeKind::Do,
                        input_slot_count: 1,
                        output_slot_count: 1,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(2),
                        label: "FetchRecords".to_string(),
                        kind: WorkflowNodeKind::ForEach,
                        input_slot_count: 1,
                        output_slot_count: 1,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(3),
                        label: "ProcessBatch".to_string(),
                        kind: WorkflowNodeKind::Parallel,
                        input_slot_count: 1,
                        output_slot_count: 1,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(4),
                        label: "ValidateOutput".to_string(),
                        kind: WorkflowNodeKind::Sequence,
                        input_slot_count: 1,
                        output_slot_count: 1,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(5),
                        label: "WriteResults".to_string(),
                        kind: WorkflowNodeKind::Do,
                        input_slot_count: 1,
                        output_slot_count: 1,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(6),
                        label: "EmitCertificate".to_string(),
                        kind: WorkflowNodeKind::Do,
                        input_slot_count: 1,
                        output_slot_count: 1,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(7),
                        label: "Notify".to_string(),
                        kind: WorkflowNodeKind::Do,
                        input_slot_count: 1,
                        output_slot_count: 1,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(8),
                        label: "ReleaseLock".to_string(),
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
                    WorkflowEdgeView {
                        from_step: vb_ui_model::StepIdx::new(4),
                        to_step: vb_ui_model::StepIdx::new(5),
                        label: None,
                    },
                    WorkflowEdgeView {
                        from_step: vb_ui_model::StepIdx::new(5),
                        to_step: vb_ui_model::StepIdx::new(6),
                        label: None,
                    },
                    WorkflowEdgeView {
                        from_step: vb_ui_model::StepIdx::new(6),
                        to_step: vb_ui_model::StepIdx::new(7),
                        label: None,
                    },
                    WorkflowEdgeView {
                        from_step: vb_ui_model::StepIdx::new(7),
                        to_step: vb_ui_model::StepIdx::new(8),
                        label: None,
                    },
                ],
                node_x: vec![
                    80.0, 200.0, 320.0, 440.0, 560.0, 680.0, 800.0, 920.0, 1040.0,
                ],
                node_y: vec![300.0; 9],
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
