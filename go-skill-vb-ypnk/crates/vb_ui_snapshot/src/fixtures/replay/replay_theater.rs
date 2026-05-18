#![forbid(unsafe_code)]

use crate::UiSnapshotError;
use alloc::{boxed::Box, string::ToString, vec};
use vb_ui_model::{
    UiAppSnapshot, WorkflowDigest,
    replay::ReplayReportView,
    replay::{RecoveryStrategy, RecoverySuggestion},
    run::{RunEventKind, RunEventView, RunStatus, SlotDiffView},
    run::RunSummaryView,
    system::StorageHealth,
    system::SystemStatusView,
    workflow::WorkflowGraphView,
    workflow::{WorkflowEdgeView, WorkflowNodeKind, WorkflowNodeView},
};

use super::super::{make_digest, DemoFixture};

pub fn replay_theater_fixture() -> Result<DemoFixture, UiSnapshotError> {
    Ok(DemoFixture {
        name: "replay_theater".to_string(),
        screen_kind: "ReplayTheater".to_string(),
        app_snapshot: UiAppSnapshot {
            status: SystemStatusView {
                storage_health: StorageHealth::Healthy,
                writer_queue_depth: 1,
                journal_batch_healthy: true,
                snapshot_seq: Some(vb_ui_model::SeqNo::new(2048)),
                blob_store_ok: true,
                index_healthy: true,
                uptime_seconds: 64800,
                active_run_count: 1,
            },
            active_runs: vec![RunSummaryView {
                run_id: vb_ui_model::RunId::new(300),
                workflow_id: vb_ui_model::WorkflowId::new(300),
                status: RunStatus::Failure,
                started_at: 1715200000,
                finished_at: Some(1715200600),
                step_count: 6,
                event_count: 1200,
            }]
            .into_boxed_slice(),
            selected_run: None,
            selected_workflow: Some(WorkflowGraphView {
                workflow_id: vb_ui_model::WorkflowId::new(300),
                workflow_digest: make_digest(0x66),
                nodes: vec![
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(0),
                        label: "Begin".to_string(),
                        kind: WorkflowNodeKind::Start,
                        input_slot_count: 0,
                        output_slot_count: 1,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(1),
                        label: "Execute".to_string(),
                        kind: WorkflowNodeKind::Sequence,
                        input_slot_count: 1,
                        output_slot_count: 1,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(2),
                        label: "Check".to_string(),
                        kind: WorkflowNodeKind::If,
                        input_slot_count: 1,
                        output_slot_count: 2,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(3),
                        label: "Retry".to_string(),
                        kind: WorkflowNodeKind::Do,
                        input_slot_count: 1,
                        output_slot_count: 1,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(4),
                        label: "Skip".to_string(),
                        kind: WorkflowNodeKind::Do,
                        input_slot_count: 1,
                        output_slot_count: 1,
                    },
                    WorkflowNodeView {
                        step_idx: vb_ui_model::StepIdx::new(5),
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
                        label: Some("ok".to_string()),
                    },
                    WorkflowEdgeView {
                        from_step: vb_ui_model::StepIdx::new(2),
                        to_step: vb_ui_model::StepIdx::new(4),
                        label: Some("fail".to_string()),
                    },
                    WorkflowEdgeView {
                        from_step: vb_ui_model::StepIdx::new(3),
                        to_step: vb_ui_model::StepIdx::new(5),
                        label: None,
                    },
                    WorkflowEdgeView {
                        from_step: vb_ui_model::StepIdx::new(4),
                        to_step: vb_ui_model::StepIdx::new(5),
                        label: None,
                    },
                ],
                node_x: vec![150.0, 300.0, 450.0, 350.0, 550.0, 700.0],
                node_y: vec![300.0, 300.0, 300.0, 150.0, 450.0, 300.0],
            }),
            verification: None,
            replay: Some(ReplayReportView {
                run_id: vb_ui_model::RunId::new(300),
                status: RunStatus::Failure,
                selected_seq: Some(vb_ui_model::SeqNo::new(512)),
                events: vec![
                    RunEventView {
                        seq: vb_ui_model::SeqNo::new(100),
                        timestamp: 1715200000,
                        shard: 0,
                        step: vb_ui_model::StepIdx::new(0),
                        kind: RunEventKind::StepEntered,
                        evidence_id: None,
                        digest: None,
                    },
                    RunEventView {
                        seq: vb_ui_model::SeqNo::new(200),
                        timestamp: 1715200100,
                        shard: 0,
                        step: vb_ui_model::StepIdx::new(1),
                        kind: RunEventKind::StepEntered,
                        evidence_id: None,
                        digest: None,
                    },
                    RunEventView {
                        seq: vb_ui_model::SeqNo::new(300),
                        timestamp: 1715200200,
                        shard: 0,
                        step: vb_ui_model::StepIdx::new(2),
                        kind: RunEventKind::StepEntered,
                        evidence_id: None,
                        digest: None,
                    },
                    RunEventView {
                        seq: vb_ui_model::SeqNo::new(400),
                        timestamp: 1715200300,
                        shard: 0,
                        step: vb_ui_model::StepIdx::new(2),
                        kind: RunEventKind::ActionFailed,
                        evidence_id: None,
                        digest: None,
                    },
                    RunEventView {
                        seq: vb_ui_model::SeqNo::new(500),
                        timestamp: 1715200400,
                        shard: 0,
                        step: vb_ui_model::StepIdx::new(2),
                        kind: RunEventKind::ErrorCaught,
                        evidence_id: None,
                        digest: None,
                    },
                    RunEventView {
                        seq: vb_ui_model::SeqNo::new(512),
                        timestamp: 1715200500,
                        shard: 0,
                        step: vb_ui_model::StepIdx::new(2),
                        kind: RunEventKind::StepExited,
                        evidence_id: None,
                        digest: None,
                    },
                ],
                slot_diffs: vec![SlotDiffView {
                    slot_idx: vb_ui_model::SlotIdx::new(0),
                    before: Some("state:idle".to_string()),
                    after: Some("state:failed".to_string()),
                    taint_before: vb_ui_model::Taint::Clean,
                    taint_after: vb_ui_model::Taint::DerivedFromSecret,
                }],
                playback_speed: 1.0,
                is_playing: false,
                recovery: Some(RecoverySuggestion {
                    strategy: RecoveryStrategy::ScheduleRetry,
                    max_attempts: 3,
                    idempotency_required: true,
                }),
            }),
            incident: None,
            actions: [].into(),
            storage: None,
            ai_context: None,
        },
    })
}
