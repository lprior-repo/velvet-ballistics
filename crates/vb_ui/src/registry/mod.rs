#![forbid(unsafe_code)]
//! Action Registry / Contract Inspector screen model (Screen 7).
//!
//! Provides pure data transformations for the Action Registry UI screen.
//! All functions are read-only views of ActionContract data.
//!
//! Layout structure:
//! ```text
//! +---------------------------------------------------------------+
//! | vb -- Action Registry                                          |
//! +----------------------------+----------------------------------+
//! | Action List                |  Action Contract Inspector        |
//! | [github.issue.create   #17]|  +- Contract -------------------+ |
//! | [slack.notify          #42]|  | Id: 17                       | |
//! | [http.get              #8 ]|  | Input slots: 1  Output: 1    | |
//! |                            |  | Max in: 1024  Max out: 512   | |
//! |                            |  | Timeout: 5000ms              | |
//! |                            |  | Idempotency: IdempotentExt   | |
//! |                            |  | SideEffect: Writes            | |
//! |                            |  | RetrySafety: KeyRequired      | |
//! |                            |  | ABI digest: 0x7f3a...        | |
//! |                            |  +-----------------------------+ |
//! |                            |  +- Capability Delta ----------+ |
//! |                            |  | Required: [net, secrets]     | |
//! |                            |  | Granted:  [net]             | |
//! |                            |  | Missing:  [secrets]         | |
//! |                            |  +-----------------------------+ |
//! |                            |  +- Failure Codes -------------+ |
//! |                            |  | [RateLimited]  [Timeout]     | |
//! |                            |  | [PermissionDeny] [Invalid]  | |
//! |                            |  | [ExternalUnavail]           | |
//! |                            |  +-----------------------------+ |
//! |                            |  +- Example Call (Postcard) ---+ |
//! |                            |  | ENCODING: postc_bin         | |
//! |                            |  | Input  Slot(0): JsonValue    | |
//! |                            |  |              max 1024 bytes | |
//! |                            |  | Output Slot(0): JsonValue   | |
//! |                            |  |              max 512 bytes | |
//! |                            |  | Timeout: 5000ms             | |
//! |                            |  +-----------------------------+ |
//! +----------------------------+----------------------------------+
//! ```
//!
//! ## Color Assignments
//!
//! | Element | Color | Hex |
//! |---------|-------|-----|
//! | Idempotency: DeterministicPure | `success` | `#16A66A` |
//! | Idempotency: IdempotentExternal | `active_cyan` | `#19A7CE` |
//! | Idempotency: AtLeastOnceExternal | `warning` | `#F59E0B` |
//! | SideEffect: None | `text_tertiary` | `#7A8796` |
//! | SideEffect: Writes | `running` | `#1F7AF5` |
//! | SideEffect: Sends | `active_cyan` | `#19A7CE` |
//! | SideEffect: Creates | `durable` | `#14B8A6` |
//! | SideEffect: Destroys | `failure` | `#E5484D` |
//! | RetrySafety: Safe | `success` | `#16A66A` |
//! | RetrySafety: KeyRequired | `warning` | `#F59E0B` |
//! | RetrySafety: Unsafe | `failure` | `#E5484D` |
//! | strict_safe badge | `success` | `#16A66A` |
//! | not strict_safe badge | `failure` | `#E5484D` |
//! | Capability: granted | `success` | `#16A66A` |
//! | Capability: missing | `failure` | `#E5484D` |
//! | Failure: applicable | `failure` | `#E5484D` |
//! | Failure: not applicable | `text_tertiary` | `#7A8796` |

use vb_core::action::{ActionContract, ActionFailureCode, Idempotency, RetrySafety, SideEffect};
use vb_core::capability::{Capability, CapabilitySet};
use vb_ui_model::system::ActionDescriptionView;

// ---------------------------------------------------------------------------
// Color constants
// ---------------------------------------------------------------------------

/// Panel background: `#FFFFFF`.
pub const COLOR_SURFACE: &str = "#FFFFFF";
/// Muted surface: `#F2F5F8`.
pub const COLOR_SURFACE_MUTED: &str = "#F2F5F8";
/// Board background: `#F4F6F8`.
pub const COLOR_BACKGROUND_BOARD: &str = "#F4F6F8";
/// Hairline: `#DDE3EA`.
pub const COLOR_LINE_HAIR: &str = "#DDE3EA";
/// Primary text: `#101828`.
pub const COLOR_TEXT_PRIMARY: &str = "#101828";
/// Secondary text: `#475467`.
pub const COLOR_TEXT_SECONDARY: &str = "#475467";
/// Tertiary text: `#7A8796`.
pub const COLOR_TEXT_TERTIARY: &str = "#7A8796";
/// Success: `#16A66A`.
pub const COLOR_SUCCESS: &str = "#16A66A";
/// Running/active: `#1F7AF5`.
pub const COLOR_RUNNING: &str = "#1F7AF5";
/// Active cyan: `#19A7CE`.
pub const COLOR_ACTIVE_CYAN: &str = "#19A7CE";
/// Warning: `#F59E0B`.
pub const COLOR_WARNING: &str = "#F59E0B";
/// Failure: `#E5484D`.
pub const COLOR_FAILURE: &str = "#E5484D";
/// Taint: `#8B5CF6`.
pub const COLOR_TAINT: &str = "#8B5CF6";
/// Durable: `#14B8A6`.
pub const COLOR_DURABLE: &str = "#14B8A6";
/// Pending: `#98A2B3`.
pub const COLOR_PENDING: &str = "#98A2B3";

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors that can occur in the Action Registry UI layer.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RegistryError {
    /// Action registry is empty or not initialized.
    RegistryEmpty,
    /// Requested action ID has no registered contract.
    ActionNotFound(u16),
    /// Capability set does not satisfy required permissions.
    CapabilityCheckFailed(String),
    /// Postcard binary encoding failed for example call view.
    EncodingFailed,
    /// Internal UI state is inconsistent.
    UiStateCorrupted,
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A single row in the action list panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionListRow {
    /// Human-readable action name.
    pub name: String,
    /// Numeric action identifier.
    pub action_id: u16,
    /// Side-effect classification.
    pub side_effect_class: SideEffect,
    /// Idempotency classification.
    pub idempotency: Idempotency,
    /// Retry safety classification.
    pub retry_safety: RetrySafety,
    /// True if retry_safety is Safe.
    pub strict_safe: bool,
    /// First required capability, if any.
    pub required_capability: Option<Capability>,
}

/// Full inspector view for a selected action contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionContractInspector {
    /// The full contract struct.
    pub contract: ActionContract,
    /// Number of input slots.
    pub input_slot_count: u16,
    /// Number of output slots.
    pub output_slot_count: u16,
    /// Maximum input byte length.
    pub max_input_bytes: u32,
    /// Maximum output byte length.
    pub max_output_bytes: u32,
    /// Maximum wall-clock time in milliseconds.
    pub timeout_ms: u64,
    /// Idempotency classification.
    pub idempotency_classification: Idempotency,
    /// Side-effect classification.
    pub side_effect_classification: SideEffect,
    /// Retry safety classification.
    pub retry_safety: RetrySafety,
    /// Stable ABI digest (u64).
    pub action_abi_digest: u64,
}

/// Delta between required, granted, and missing capabilities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityDelta {
    /// Required capabilities for the action.
    pub required: Vec<Capability>,
    /// Capabilities granted in the current session.
    pub granted: Vec<Capability>,
    /// Capabilities required but not granted.
    pub missing: Vec<Capability>,
}

/// A single failure code row in the failure code panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureCodeRow {
    /// The failure code enum variant.
    pub code: ActionFailureCode,
    /// Human-readable label.
    pub label: String,
    /// Whether this code applies to the selected action.
    pub is_applicable: bool,
    /// Hex color for display.
    pub color: String,
}

/// Example call view showing postcard binary schema summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExampleCallView {
    /// Human-readable action name.
    pub action_name: String,
    /// Numeric action identifier.
    pub action_id: u16,
    /// Input slot schema summary.
    pub input_schema_summary: String,
    /// Output slot schema summary.
    pub output_schema_summary: String,
    /// Maximum input bytes.
    pub max_input_bytes: u32,
    /// Maximum output bytes.
    pub max_output_bytes: u32,
    /// Timeout in milliseconds.
    pub timeout_ms: u64,
    /// Encoding label.
    pub encoding_label: String,
}

/// Screen state for the Action Registry screen.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActionRegistryState {
    /// Currently selected action ID, if any.
    pub selected_action_id: Option<u16>,
    /// List of all available actions.
    pub action_list: Vec<ActionListRow>,
    /// Currently selected inspector, if any.
    pub selected_inspector: Option<ActionContractInspector>,
    /// Capability delta for selected action.
    pub capability_delta: Option<CapabilityDelta>,
    /// Failure codes for selected action.
    pub failure_codes: Vec<FailureCodeRow>,
    /// Example call view for selected action.
    pub example_call: Option<ExampleCallView>,
}

// ---------------------------------------------------------------------------
// Contract functions
// ---------------------------------------------------------------------------

/// Build the action list from a slice of action contracts.
pub fn get_action_list(contracts: &[ActionContract]) -> Vec<ActionListRow> {
    let mut rows = Vec::with_capacity(contracts.len());
    for contract in contracts {
        let name = action_name_from_id(contract.id);
        let required_cap = contract.required_capabilities.first().cloned();
        rows.push(ActionListRow {
            name,
            action_id: contract.id.get(),
            side_effect_class: contract.side_effect,
            idempotency: contract.idempotency,
            retry_safety: contract.retry_safety,
            strict_safe: contract.retry_safety == RetrySafety::Safe,
            required_capability: required_cap,
        });
    }
    rows
}

/// Build an inspector view from an action contract.
pub fn get_action_inspector(contract: &ActionContract) -> ActionContractInspector {
    ActionContractInspector {
        contract: contract.clone(),
        input_slot_count: contract.input_slot_count,
        output_slot_count: contract.output_slot_count,
        max_input_bytes: contract.max_input_bytes,
        max_output_bytes: contract.max_output_bytes,
        timeout_ms: contract.timeout_ms,
        idempotency_classification: contract.idempotency,
        side_effect_classification: contract.side_effect,
        retry_safety: contract.retry_safety,
        action_abi_digest: compute_abi_digest(contract),
    }
}

/// Compute the capability delta for a contract against a granted set.
pub fn compute_capability_delta(
    contract: &ActionContract,
    granted: &CapabilitySet,
) -> CapabilityDelta {
    let mut required_list = Vec::with_capacity(contract.required_capabilities.len());
    let mut granted_list = Vec::new();
    let mut missing_list = Vec::new();

    for cap in &contract.required_capabilities {
        required_list.push(cap.clone());
        if granted.grants(cap) {
            granted_list.push(cap.clone());
        } else {
            missing_list.push(cap.clone());
        }
    }

    CapabilityDelta {
        required: required_list,
        granted: granted_list,
        missing: missing_list,
    }
}

/// Get failure code rows for an action contract.
pub fn get_failure_codes(_contract: &ActionContract) -> Vec<FailureCodeRow> {
    vec![
        FailureCodeRow {
            code: ActionFailureCode::RateLimited,
            label: "RateLimited".to_string(),
            is_applicable: true,
            color: COLOR_FAILURE.to_string(),
        },
        FailureCodeRow {
            code: ActionFailureCode::Timeout,
            label: "Timeout".to_string(),
            is_applicable: true,
            color: COLOR_FAILURE.to_string(),
        },
        FailureCodeRow {
            code: ActionFailureCode::PermissionDenied,
            label: "PermissionDenied".to_string(),
            is_applicable: true,
            color: COLOR_FAILURE.to_string(),
        },
        FailureCodeRow {
            code: ActionFailureCode::InvalidInput,
            label: "InvalidInput".to_string(),
            is_applicable: true,
            color: COLOR_FAILURE.to_string(),
        },
        FailureCodeRow {
            code: ActionFailureCode::ExternalUnavailable,
            label: "ExternalUnavailable".to_string(),
            is_applicable: true,
            color: COLOR_FAILURE.to_string(),
        },
    ]
}

/// Render the example call view for an action contract using postcard schema.
pub fn render_example_call(contract: &ActionContract) -> ExampleCallView {
    let input_summary = if contract.input_slot_count > 0 {
        format!(
            "Slot(0): [u8; N]  -- serialized input value\n              max {} bytes",
            contract.max_input_bytes
        )
    } else {
        "No input slots".to_string()
    };

    let output_summary = if contract.output_slot_count > 0 {
        format!(
            "Slot(0): [u8; M]  -- serialized output value\n              max {} bytes",
            contract.max_output_bytes
        )
    } else {
        "No output slots".to_string()
    };

    ExampleCallView {
        action_name: action_name_from_id(contract.id),
        action_id: contract.id.get(),
        input_schema_summary: input_summary,
        output_schema_summary: output_summary,
        max_input_bytes: contract.max_input_bytes,
        max_output_bytes: contract.max_output_bytes,
        timeout_ms: contract.timeout_ms,
        encoding_label: "postc_bin".to_string(),
    }
}

/// Select an action by ID and populate all derived views.
pub fn select_action(
    state: &mut ActionRegistryState,
    contracts: &[ActionContract],
    granted: &CapabilitySet,
    action_id: u16,
) -> Result<(), RegistryError> {
    if contracts.is_empty() {
        return Err(RegistryError::RegistryEmpty);
    }

    let contract = contracts
        .iter()
        .find(|c| c.id.get() == action_id)
        .ok_or(RegistryError::ActionNotFound(action_id))?;

    state.selected_action_id = Some(action_id);
    state.selected_inspector = Some(get_action_inspector(contract));
    state.capability_delta = Some(compute_capability_delta(contract, granted));
    state.failure_codes = get_failure_codes(contract);
    state.example_call = Some(render_example_call(contract));

    Ok(())
}

/// Derive a human-readable name from an ActionId.
pub fn action_name_from_id(action_id: vb_core::ids::ActionId) -> String {
    let id_val = action_id.get();
    format!("action_{}", id_val)
}

/// Compute a stable u64 digest from an ActionContract.
pub fn compute_abi_digest(contract: &ActionContract) -> u64 {
    use core::hash::{Hash, Hasher};

    let mut hasher = seahash::SeaHasher::new();
    contract.id.hash(&mut hasher);
    contract.input_slot_count.hash(&mut hasher);
    contract.output_slot_count.hash(&mut hasher);
    contract.max_input_bytes.hash(&mut hasher);
    contract.max_output_bytes.hash(&mut hasher);
    contract.timeout_ms.hash(&mut hasher);
    idempotency_tag(contract.idempotency).hash(&mut hasher);
    side_effect_tag(contract.side_effect).hash(&mut hasher);
    retry_safety_tag(contract.retry_safety).hash(&mut hasher);
    hasher.finish()
}

fn idempotency_tag(idempotency: Idempotency) -> u8 {
    match idempotency {
        Idempotency::DeterministicPure => 0,
        Idempotency::IdempotentExternal => 1,
        Idempotency::AtLeastOnceExternal => 2,
    }
}

fn side_effect_tag(side_effect: SideEffect) -> u8 {
    match side_effect {
        SideEffect::None => 0,
        SideEffect::Writes => 1,
        SideEffect::Sends => 2,
        SideEffect::Creates => 3,
        SideEffect::Destroys => 4,
    }
}

fn retry_safety_tag(safety: RetrySafety) -> u8 {
    match safety {
        RetrySafety::Safe => 0,
        RetrySafety::KeyRequired => 1,
        RetrySafety::Unsafe => 2,
    }
}

/// Get the color for an idempotency classification.
pub fn idempotency_color(idempotency: Idempotency) -> &'static str {
    match idempotency {
        Idempotency::DeterministicPure => COLOR_SUCCESS,
        Idempotency::IdempotentExternal => COLOR_ACTIVE_CYAN,
        Idempotency::AtLeastOnceExternal => COLOR_WARNING,
    }
}

/// Get the color for a side-effect classification.
pub fn side_effect_color(side_effect: SideEffect) -> &'static str {
    match side_effect {
        SideEffect::None => COLOR_TEXT_TERTIARY,
        SideEffect::Writes => COLOR_RUNNING,
        SideEffect::Sends => COLOR_ACTIVE_CYAN,
        SideEffect::Creates => COLOR_DURABLE,
        SideEffect::Destroys => COLOR_FAILURE,
    }
}

/// Get the color for a retry-safety classification.
pub fn retry_safety_color(safety: RetrySafety) -> &'static str {
    match safety {
        RetrySafety::Safe => COLOR_SUCCESS,
        RetrySafety::KeyRequired => COLOR_WARNING,
        RetrySafety::Unsafe => COLOR_FAILURE,
    }
}

/// Convert an ActionDescriptionView to ActionListRow.
pub fn action_list_row_from_description(desc: &ActionDescriptionView) -> ActionListRow {
    ActionListRow {
        name: desc.name.clone(),
        action_id: desc.id.get(),
        side_effect_class: desc.side_effect,
        idempotency: desc.idempotency,
        retry_safety: desc.retry_safety,
        strict_safe: desc.retry_safety == RetrySafety::Safe,
        required_capability: desc.required_capabilities.first().cloned(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::ActionId;

    fn make_contract(id: u16) -> ActionContract {
        ActionContract {
            id: ActionId::new(id),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 512,
            timeout_ms: 5000,
            idempotency: Idempotency::IdempotentExternal,
            side_effect: SideEffect::Writes,
            retry_safety: RetrySafety::KeyRequired,
            required_capabilities: Box::new([
                Capability::new("net".into(), ActionId::new(id)),
                Capability::new("secrets".into(), ActionId::new(id)),
            ]),
        }
    }

    #[test]
    fn test_action_list_row_from_contract() {
        let contract = make_contract(17);
        let rows = get_action_list(&[contract]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows.first().map(|row| row.action_id), Some(17));
        assert_eq!(rows.first().map(|row| row.strict_safe), Some(false));
        assert_eq!(
            rows.first().map(|row| row.required_capability.is_some()),
            Some(true)
        );
    }

    #[test]
    fn test_inspector_from_contract() {
        let contract = make_contract(42);
        let inspector = get_action_inspector(&contract);
        assert_eq!(inspector.input_slot_count, 1);
        assert_eq!(inspector.output_slot_count, 1);
        assert_eq!(inspector.timeout_ms, 5000);
    }

    #[test]
    fn test_capability_delta_all_granted() {
        let contract = make_contract(1);
        let granted = CapabilitySet::from_grants(Box::new([
            Capability::new("net".into(), ActionId::new(1)),
            Capability::new("secrets".into(), ActionId::new(1)),
        ]));
        let delta = compute_capability_delta(&contract, &granted);
        assert!(delta.missing.is_empty());
        assert_eq!(delta.granted.len(), 2);
    }

    #[test]
    fn test_capability_delta_partially_granted() {
        let contract = make_contract(1);
        let granted =
            CapabilitySet::from_grants(Box::new([Capability::new("net".into(), ActionId::new(1))]));
        let delta = compute_capability_delta(&contract, &granted);
        assert_eq!(delta.missing.len(), 1);
        assert_eq!(delta.granted.len(), 1);
    }

    #[test]
    fn test_capability_delta_none_granted() {
        let contract = make_contract(1);
        let granted = CapabilitySet::empty();
        let delta = compute_capability_delta(&contract, &granted);
        assert_eq!(delta.missing.len(), 2);
        assert!(delta.granted.is_empty());
    }

    #[test]
    fn test_failure_codes_all_applicable() {
        let contract = make_contract(1);
        let codes = get_failure_codes(&contract);
        assert_eq!(codes.len(), 5);
        assert!(codes.iter().all(|c| c.is_applicable));
    }

    #[test]
    fn test_example_call_view_postcard_encoding() {
        let contract = make_contract(1);
        let view = render_example_call(&contract);
        assert_eq!(view.encoding_label, "postc_bin");
        assert!(view.input_schema_summary.contains("Slot(0)"));
    }

    #[test]
    fn test_abi_digest_stable() {
        let contract = make_contract(99);
        let d1 = compute_abi_digest(&contract);
        let d2 = compute_abi_digest(&contract);
        assert_eq!(d1, d2);
    }

    #[test]
    fn test_strict_safe_true_for_safe_retry() {
        let mut contract = make_contract(1);
        contract.retry_safety = RetrySafety::Safe;
        let rows = get_action_list(&[contract]);
        assert_eq!(rows.first().map(|row| row.strict_safe), Some(true));
    }

    #[test]
    fn test_strict_safe_false_for_unsafe_retry() {
        let mut contract = make_contract(1);
        contract.retry_safety = RetrySafety::Unsafe;
        let rows = get_action_list(&[contract]);
        assert_eq!(rows.first().map(|row| row.strict_safe), Some(false));
    }

    #[test]
    fn test_action_name_derivation() {
        let name = action_name_from_id(ActionId::new(42));
        assert_eq!(name, "action_42");
    }

    #[test]
    fn test_idempotency_color_mapping() {
        assert_eq!(
            idempotency_color(Idempotency::DeterministicPure),
            COLOR_SUCCESS
        );
        assert_eq!(
            idempotency_color(Idempotency::IdempotentExternal),
            COLOR_ACTIVE_CYAN
        );
        assert_eq!(
            idempotency_color(Idempotency::AtLeastOnceExternal),
            COLOR_WARNING
        );
    }

    #[test]
    fn test_side_effect_color_mapping() {
        assert_eq!(side_effect_color(SideEffect::None), COLOR_TEXT_TERTIARY);
        assert_eq!(side_effect_color(SideEffect::Writes), COLOR_RUNNING);
        assert_eq!(side_effect_color(SideEffect::Sends), COLOR_ACTIVE_CYAN);
        assert_eq!(side_effect_color(SideEffect::Creates), COLOR_DURABLE);
        assert_eq!(side_effect_color(SideEffect::Destroys), COLOR_FAILURE);
    }

    #[test]
    fn test_retry_safety_color_mapping() {
        assert_eq!(retry_safety_color(RetrySafety::Safe), COLOR_SUCCESS);
        assert_eq!(retry_safety_color(RetrySafety::KeyRequired), COLOR_WARNING);
        assert_eq!(retry_safety_color(RetrySafety::Unsafe), COLOR_FAILURE);
    }

    #[test]
    fn test_select_action_populates_state() {
        let contracts = [make_contract(10), make_contract(20)];
        let granted = CapabilitySet::empty();
        let mut state = ActionRegistryState::default();

        let result = select_action(&mut state, &contracts, &granted, 20);
        assert!(result.is_ok());
        assert_eq!(state.selected_action_id, Some(20));
        assert!(state.selected_inspector.is_some());
        assert!(state.capability_delta.is_some());
        assert!(!state.failure_codes.is_empty());
        assert!(state.example_call.is_some());
    }

    #[test]
    fn test_select_action_not_found() {
        let contracts = [make_contract(10)];
        let granted = CapabilitySet::empty();
        let mut state = ActionRegistryState::default();

        let result = select_action(&mut state, &contracts, &granted, 99);
        assert!(matches!(result, Err(RegistryError::ActionNotFound(99))));
    }

    #[test]
    fn test_select_action_empty_registry() {
        let contracts: [ActionContract; 0] = [];
        let granted = CapabilitySet::empty();
        let mut state = ActionRegistryState::default();

        let result = select_action(&mut state, &contracts, &granted, 1);
        assert!(matches!(result, Err(RegistryError::RegistryEmpty)));
    }
}
