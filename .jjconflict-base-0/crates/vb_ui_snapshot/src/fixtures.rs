#![forbid(unsafe_code)]

use crate::UiSnapshotError;
use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
};
use serde::{Deserialize, Serialize};
use vb_ui_model::{
    Capability, UiAppSnapshot, WorkflowDigest,
    ai::AiContextView,
    ai::{AiContextPanel, ReplaySafety},
    incident::IncidentReportView,
    incident::IncidentSeverity,
    replay::ReplayReportView,
    replay::{RecoveryStrategy, RecoverySuggestion},
    run::{RunEventKind, RunEventView, RunStatus, SlotDiffView, StepStateView, StepStatus},
    run::{RunInspectionView, RunSummaryView},
    storage::StorageDoctorView,
    storage::{EvidenceCardPanel, JournalDoctorPanel, StorageHealthPanel},
    system::ActionDescriptionView,
    system::StorageHealth,
    system::SystemStatusView,
    verify::VerificationCertificate,
    verify::VerificationReportView,
    workflow::WorkflowGraphView,
    workflow::{WorkflowEdgeView, WorkflowNodeKind, WorkflowNodeView},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemoFixture {
    pub name: String,
    pub screen_kind: String,
    pub app_snapshot: UiAppSnapshot,
}

fn make_digest(data: u8) -> WorkflowDigest {
    let bytes = [data; 32];
    WorkflowDigest::from_bytes(bytes)
}

pub fn load_demo_fixture(name: &str) -> Result<DemoFixture, UiSnapshotError> {
    match name {
        "execution_overview" => Ok(execution_overview_fixture()),
        "workflow_graph_authoring" => Ok(workflow_graph_authoring_fixture()),
        "execution_details" => Ok(execution_details_fixture()),
        "verification_certificate" => Ok(verification_certificate_fixture()),
        "replay_theater" => Ok(replay_theater_fixture()),
        "incident_failure" => Ok(incident_failure_fixture()),
        "action_registry" => Ok(action_registry_fixture()),
        "storage_doctor_ai_context" => Ok(storage_doctor_ai_context_fixture()),
        _ => Err(UiSnapshotError::FixtureNotFound(name.to_string())),
    }
}

fn execution_overview_fixture() -> DemoFixture {
    DemoFixture {
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
    }
}

fn workflow_graph_authoring_fixture() -> DemoFixture {
    DemoFixture {
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
    }
}

fn execution_details_fixture() -> DemoFixture {
    DemoFixture {
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
    }
}

fn verification_certificate_fixture() -> DemoFixture {
    DemoFixture {
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
    }
}

fn replay_theater_fixture() -> DemoFixture {
    DemoFixture {
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
    }
}

fn incident_failure_fixture() -> DemoFixture {
    DemoFixture {
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
    }
}

fn action_registry_fixture() -> DemoFixture {
    let cap_storage_read = Capability::new("StorageRead".into(), vb_ui_model::ActionId::new(1));
    let cap_storage_write = Capability::new("StorageWrite".into(), vb_ui_model::ActionId::new(2));

    DemoFixture {
        name: "action_registry".to_string(),
        screen_kind: "ActionRegistry".to_string(),
        app_snapshot: UiAppSnapshot {
            status: SystemStatusView {
                storage_health: StorageHealth::Healthy,
                writer_queue_depth: 0,
                journal_batch_healthy: true,
                snapshot_seq: Some(vb_ui_model::SeqNo::new(100)),
                blob_store_ok: true,
                index_healthy: true,
                uptime_seconds: 18000,
                active_run_count: 0,
            },
            active_runs: [].into(),
            selected_run: None,
            selected_workflow: None,
            verification: None,
            replay: None,
            incident: None,
            actions: vec![
                ActionDescriptionView {
                    id: vb_ui_model::ActionId::new(10),
                    name: "FetchSource".to_string(),
                    side_effect: vb_ui_model::SideEffect::None,
                    idempotency: vb_ui_model::Idempotency::DeterministicPure,
                    retry_safety: vb_ui_model::RetrySafety::Safe,
                    required_capabilities: Box::new([cap_storage_read.clone()]),
                    timeout_ms: 5000,
                    input_slot_count: 0,
                    output_slot_count: 1,
                    max_input_bytes: 0,
                    max_output_bytes: 1048576,
                },
                ActionDescriptionView {
                    id: vb_ui_model::ActionId::new(11),
                    name: "ValidateSchema".to_string(),
                    side_effect: vb_ui_model::SideEffect::None,
                    idempotency: vb_ui_model::Idempotency::DeterministicPure,
                    retry_safety: vb_ui_model::RetrySafety::Safe,
                    required_capabilities: Box::new([cap_storage_read.clone()]),
                    timeout_ms: 2000,
                    input_slot_count: 1,
                    output_slot_count: 1,
                    max_input_bytes: 10485760,
                    max_output_bytes: 1048576,
                },
                ActionDescriptionView {
                    id: vb_ui_model::ActionId::new(12),
                    name: "TransformData".to_string(),
                    side_effect: vb_ui_model::SideEffect::Writes,
                    idempotency: vb_ui_model::Idempotency::AtLeastOnceExternal,
                    retry_safety: vb_ui_model::RetrySafety::Unsafe,
                    required_capabilities: Box::new([cap_storage_write.clone()]),
                    timeout_ms: 30000,
                    input_slot_count: 1,
                    output_slot_count: 1,
                    max_input_bytes: 104857600,
                    max_output_bytes: 104857600,
                },
                ActionDescriptionView {
                    id: vb_ui_model::ActionId::new(13),
                    name: "LoadSink".to_string(),
                    side_effect: vb_ui_model::SideEffect::Writes,
                    idempotency: vb_ui_model::Idempotency::AtLeastOnceExternal,
                    retry_safety: vb_ui_model::RetrySafety::KeyRequired,
                    required_capabilities: Box::new([cap_storage_write]),
                    timeout_ms: 60000,
                    input_slot_count: 1,
                    output_slot_count: 0,
                    max_input_bytes: 104857600,
                    max_output_bytes: 0,
                },
            ]
            .into_boxed_slice(),
            storage: None,
            ai_context: None,
        },
    }
}

fn storage_doctor_ai_context_fixture() -> DemoFixture {
    DemoFixture {
        name: "storage_doctor_ai_context".to_string(),
        screen_kind: "StorageDoctorAiContext".to_string(),
        app_snapshot: UiAppSnapshot {
            status: SystemStatusView {
                storage_health: StorageHealth::Degraded,
                writer_queue_depth: 4,
                journal_batch_healthy: true,
                snapshot_seq: Some(vb_ui_model::SeqNo::new(7200)),
                blob_store_ok: true,
                index_healthy: false,
                uptime_seconds: 259200,
                active_run_count: 2,
            },
            active_runs: [].into(),
            selected_run: None,
            selected_workflow: None,
            verification: None,
            replay: None,
            incident: None,
            actions: [].into(),
            storage: Some(StorageDoctorView {
                health: StorageHealthPanel {
                    fjall_keyspaces: vec![
                        vb_ui_model::storage::KeyspaceMetrics {
                            name: "runs".to_string(),
                            key_count: 1500,
                            size_bytes: 104857600,
                            profile: "default".to_string(),
                        },
                        vb_ui_model::storage::KeyspaceMetrics {
                            name: "journal".to_string(),
                            key_count: 500000,
                            size_bytes: 524288000,
                            profile: "journal".to_string(),
                        },
                    ],
                    writer_queue: vb_ui_model::storage::WriterQueueStatus {
                        pending_journaled: 3,
                        pending_strict: 1,
                        is_shutdown: false,
                    },
                    journal_batch: vb_ui_model::storage::JournalBatchHealth {
                        last_flush_ms: Some(1715400000),
                        flushed_count: 15000,
                        dropped_count: 0,
                    },
                    snapshot: vb_ui_model::storage::SnapshotStatus {
                        latest_seq: Some(vb_ui_model::SeqNo::new(7200)),
                        snapshot_count: 12,
                        is_corrupt: false,
                    },
                    blob_store: vb_ui_model::storage::BlobStoreStatus {
                        blob_count: 3000,
                        size_bytes: 1073741824,
                        is_accessible: true,
                    },
                    index: vb_ui_model::storage::IndexHealth {
                        status_count: 1000,
                        workflow_count: 50,
                        action_count: 25,
                        is_consistent: false,
                    },
                },
                journal: JournalDoctorPanel {
                    run_event_count: 500000,
                    snapshot_seq: 7200,
                    tail_seq: 7500,
                    corrupt_records: vb_ui_model::storage::CorruptRecordStatus::Corrupt {
                        count: 1,
                        first_seq: Some(vb_ui_model::SeqNo::new(7333)),
                    },
                    trim_recommendation: vb_ui_model::storage::TrimRecommendation::Recommended {
                        tail_seq: 7200,
                        snapshot_seq: 7200,
                    },
                    digest_checks: vb_ui_model::storage::DigestCheckResult {
                        workflow_source_ok: true,
                        compiled_ir_ok: false,
                        all_ok: false,
                    },
                },
                ai_context: AiContextPanel {
                    safe_for_model: true,
                    secrets_redacted: true,
                    blobs_summarized: true,
                    suggested_commands: vec![
                        vb_ui_model::ai::SuggestedCommand {
                            cmd: "velvet storage repair --seq 7333".to_string(),
                            desc: "Repair corrupt record".to_string(),
                        },
                        vb_ui_model::ai::SuggestedCommand {
                            cmd: "velvet journal trim --keep 7200".to_string(),
                            desc: "Trim journal".to_string(),
                        },
                        vb_ui_model::ai::SuggestedCommand {
                            cmd: "velvet index rebuild".to_string(),
                            desc: "Rebuild index".to_string(),
                        },
                    ],
                    failure_summary: String::new(),
                    replay_safety: ReplaySafety::Safe,
                },
                evidence: EvidenceCardPanel {
                    last_cert_check: Some(1715400000),
                    last_replay_check: Some(1715400100),
                    last_crash_lab_fixture: Some(1715400200),
                    incomplete_warnings: vec![vb_ui_model::storage::IncompleteWarning {
                        kind: "MissingFragments".to_string(),
                        message: "Evidence blob e8f3 missing 2 fragments".to_string(),
                    }],
                },
            }),
            ai_context: Some(AiContextView {
                run_id: vb_ui_model::RunId::new(500),
                safe_for_model: true,
                secrets_redacted: true,
                blobs_summarized: true,
                failure_summary: None,
                replay_safe: true,
                suggested_commands: Box::new([
                    "velvet storage repair --seq 7333".to_string(),
                    "velvet journal trim --keep 7200".to_string(),
                    "velvet index rebuild".to_string(),
                ]),
                last_cert_check: Some(1715400000),
                last_replay_check: Some(1715400100),
                last_crash_lab_fixture: Some("crash_seq_7333.yaml".to_string()),
                incomplete_evidence_warnings: Box::new([
                    "Evidence blob e8f3 missing 2 fragments".to_string()
                ]),
            }),
        },
    }
}

pub fn serialize_fixture(fixture: &DemoFixture) -> Result<String, UiSnapshotError> {
    serde_json::to_string_pretty(fixture)
        .map_err(|e| UiSnapshotError::IoError(format!("JSON serialization failed: {e}")))
}
