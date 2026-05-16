#![forbid(unsafe_code)]

pub mod execution;
pub mod registry;
pub mod replay;
pub mod verification;

use alloc::format;
use serde::{Deserialize, Serialize};
use vb_ui_model::{UiAppSnapshot, WorkflowDigest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemoFixture {
    pub name: String,
    pub screen_kind: String,
    pub app_snapshot: UiAppSnapshot,
}

pub fn make_digest(data: u8) -> WorkflowDigest {
    let bytes = [data; 32];
    WorkflowDigest::from_bytes(bytes)
}

pub fn load_demo_fixture(name: &str) -> Result<DemoFixture, crate::UiSnapshotError> {
    match name {
        "execution_overview" => execution::execution_overview_fixture(),
        "workflow_graph_authoring" => execution::workflow_graph_authoring_fixture(),
        "execution_details" => execution::execution_details_fixture(),
        "verification_certificate" => verification::verification_certificate_fixture(),
        "replay_theater" => replay::replay_theater_fixture(),
        "incident_failure" => replay::incident_failure_fixture(),
        "action_registry" => registry::action_registry_fixture(),
        "storage_doctor_ai_context" => registry::storage_doctor_ai_context_fixture(),
        _ => Err(crate::UiSnapshotError::FixtureNotFound(name.to_string())),
    }
}

pub fn serialize_fixture(fixture: &DemoFixture) -> Result<String, crate::UiSnapshotError> {
    serde_json::to_string_pretty(fixture)
        .map_err(|e| crate::UiSnapshotError::IoError(format!("JSON serialization failed: {e}")))
}
