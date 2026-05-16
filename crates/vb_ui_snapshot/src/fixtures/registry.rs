#![forbid(unsafe_code)]

use crate::UiSnapshotError;
use alloc::{boxed::Box, string::ToString, vec, vec::Vec};
use vb_ui_model::{
    Capability, UiAppSnapshot, WorkflowDigest,
    ai::{AiContextPanel, AiContextView, ReplaySafety},
    run::RunStatus,
    storage::StorageDoctorView,
    storage::{EvidenceCardPanel, JournalDoctorPanel, StorageHealthPanel},
    system::ActionDescriptionView,
    system::StorageHealth,
    system::SystemStatusView,
};

use super::{make_digest, DemoFixture};

pub fn action_registry_fixture() -> Result<DemoFixture, UiSnapshotError> {
    let cap_storage_read = Capability::new("StorageRead".into(), vb_ui_model::ActionId::new(1));
    let cap_storage_write = Capability::new("StorageWrite".into(), vb_ui_model::ActionId::new(2));

    Ok(DemoFixture {
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
    })
}

pub fn storage_doctor_ai_context_fixture() -> Result<DemoFixture, UiSnapshotError> {
    Ok(DemoFixture {
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
    })
}
