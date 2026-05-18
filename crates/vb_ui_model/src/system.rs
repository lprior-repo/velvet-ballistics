//! System status and action-registry view types.

use alloc::boxed::Box;
use alloc::string::String;
use serde::{Deserialize, Serialize};

use vb_core::action::{Idempotency, RetrySafety, SideEffect};
use vb_core::capability::Capability;
use vb_core::ids::{ActionId, SeqNo};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemStatusView {
    pub storage_health: StorageHealth,
    pub writer_queue_depth: u32,
    pub journal_batch_healthy: bool,
    pub snapshot_seq: Option<SeqNo>,
    pub blob_store_ok: bool,
    pub index_healthy: bool,
    pub uptime_seconds: i64,
    pub active_run_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum StorageHealth {
    Healthy = 0,
    Degraded = 1,
    Corrupt = 2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionDescriptionView {
    pub id: ActionId,
    pub name: String,
    pub side_effect: SideEffect,
    pub idempotency: Idempotency,
    pub retry_safety: RetrySafety,
    pub required_capabilities: Box<[Capability]>,
    pub timeout_ms: u64,
    pub input_slot_count: u16,
    pub output_slot_count: u16,
    pub max_input_bytes: u32,
    pub max_output_bytes: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_description_view_required_capabilities_roundtrip_matches_action_contract_source()
    -> Result<(), String> {
        // Given
        let required: Box<[Capability]> = Box::from([Capability::new(
            Box::<str>::from("network.github"),
            ActionId::new(7),
        )]);
        let view = ActionDescriptionView {
            id: ActionId::new(7),
            name: String::from("github_issue"),
            side_effect: SideEffect::Writes,
            idempotency: Idempotency::IdempotentExternal,
            retry_safety: RetrySafety::KeyRequired,
            required_capabilities: required.clone(),
            timeout_ms: 1000,
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 2048,
        };

        // When
        let encoded = serde_json::to_vec(&view)
            .map_err(|error| format!("serialize ActionDescriptionView failed: {error}"))?;
        let decoded: ActionDescriptionView = serde_json::from_slice(&encoded)
            .map_err(|error| format!("deserialize ActionDescriptionView failed: {error}"))?;

        // Then
        assert_eq!(decoded.required_capabilities, required);
        assert_eq!(decoded, view);
        Ok(())
    }
}
