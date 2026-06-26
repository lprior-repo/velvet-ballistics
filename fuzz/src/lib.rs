//! Shared fuzz target bodies for Velvet Ballistics evidence gates.
#![allow(clippy::indexing_slicing)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::let_underscore_must_use)]
#![allow(clippy::as_conversions)]
#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::len_zero)]

pub mod bin_common;

pub mod yaml_target;
pub mod validation_target;
pub mod journal_target;
pub mod expression_target;
pub mod workflow_target;
pub mod ipc_target;
pub mod boundary_target;

pub use yaml_target::fuzz_yaml_events;
pub use yaml_target::fuzz_strict_yaml_profile;
pub use yaml_target::fuzz_compile_source_ast_marks;
pub use yaml_target::fuzz_span_bridge;

pub use validation_target::fuzz_capability_name_schema;
pub use validation_target::fuzz_capability_contract_schema;
pub use validation_target::fuzz_verifier_gates;
pub use validation_target::fuzz_diagnostic_from_error;
pub use validation_target::fuzz_diagnostic_code_from_str;

pub use journal_target::fuzz_journal_event;
pub use journal_target::fuzz_replay_events;
pub use journal_target::fuzz_extract_terminal;
pub use journal_target::fuzz_action_tracker;
pub use journal_target::fuzz_admission_flow;
pub use journal_target::fuzz_admission_fuzz;
pub use journal_target::fuzz_strict_artifact_decoder;
pub use journal_target::fuzz_digest_coherence;
pub use journal_target::fuzz_readback_family_set;
pub use journal_target::fuzz_admission_input_surface;
pub use journal_target::fuzz_accepted_artifact_decode;
pub use journal_target::fuzz_recovery_decode;
pub use journal_target::fuzz_vb_qi37_12_persisted_payload_decode;
pub use journal_target::fuzz_storage_envelope_boundary;
pub use journal_target::fuzz_binary_payload_boundary;
pub use journal_target::fuzz_accepted_artifact_envelope_qi37_4_2;

pub use expression_target::fuzz_expression;
pub use expression_target::fuzz_expr_bytecode;
pub use expression_target::fuzz_taint_propagation;
pub use expression_target::fuzz_expr_eval;

pub use workflow_target::fuzz_compiled_ir;
pub use workflow_target::fuzz_generated_compare;
pub use workflow_target::fuzz_resource_budget;
pub use workflow_target::fuzz_budget_compute;
pub use workflow_target::fuzz_accessor_traversal;
pub use workflow_target::fuzz_slot_value_roundtrip;
pub use workflow_target::fuzz_collect_page_pagination;
pub use workflow_target::fuzz_step_budget_new;

pub use ipc_target::fuzz_ipc_frame;
pub use ipc_target::fuzz_ipc_decode;
pub use ipc_target::fuzz_ipc_frame_boundary;

pub use boundary_target::fuzz_external_input_adapter_boundary;
