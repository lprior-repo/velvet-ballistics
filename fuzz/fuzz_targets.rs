//! Safe callable fuzz target bodies for harness integration.

/// YAML event parser fuzz target.
pub fn yaml_events(data: &[u8]) {
    fuzz_lib::fuzz_yaml_events(data);
}

/// Binary IPC frame fuzz target.
pub fn ipc_frame(data: &[u8]) {
    fuzz_lib::fuzz_ipc_frame(data);
}

/// Journal record envelope fuzz target.
pub fn journal_event(data: &[u8]) {
    fuzz_lib::fuzz_journal_event(data);
}

/// Expression lexer/parser/compiler/evaluator fuzz target.
pub fn expression(data: &[u8]) {
    fuzz_lib::fuzz_expression(data);
}

/// Compiled IR decode/validation fuzz target.
pub fn compiled_ir(data: &[u8]) {
    fuzz_lib::fuzz_compiled_ir(data);
}

/// Generated-vs-IR comparison fuzz target.
pub fn generated_compare(data: &[u8]) {
    fuzz_lib::fuzz_generated_compare(data);
}

/// Arbitrary bytecode expression evaluation fuzz target.
pub fn expr_bytecode(data: &[u8]) {
    fuzz_lib::fuzz_expr_bytecode(data);
}

/// Taint propagation invariant fuzz target.
pub fn taint_propagation(data: &[u8]) {
    fuzz_lib::fuzz_taint_propagation(data);
}

/// Resource budget counting fuzz target.
pub fn resource_budget(data: &[u8]) {
    fuzz_lib::fuzz_resource_budget(data);
}

/// Expression evaluator postcard-decode fuzz target.
pub fn expr_eval(data: &[u8]) {
    fuzz_lib::fuzz_expr_eval(data);
}

/// Accessor path traversal fuzz target.
pub fn accessor_traversal(data: &[u8]) {
    fuzz_lib::fuzz_accessor_traversal(data);
}

/// SlotValue postcard roundtrip fuzz target.
pub fn slot_value_roundtrip(data: &[u8]) {
    fuzz_lib::fuzz_slot_value_roundtrip(data);
}

/// Admission flow arbitrary artifact bytes fuzz target.
pub fn admission_fuzz(data: &[u8]) {
    fuzz_lib::fuzz_admission_fuzz(data);
}

/// UI model OutputEnvelope postcard decode target.
pub fn vb_ui_model_postcard_decode(data: &[u8]) {
    fuzz_lib::fuzz_vb_ui_model_postcard_decode(data);
}

/// Action tracker state transition fuzz target.
pub fn action_tracker(data: &[u8]) {
    fuzz_lib::fuzz_action_tracker(data);
}
