//! Safe callable fuzz target bodies for harness integration.

/// YAML parser fuzz target.
pub fn workflow_parse(data: &[u8]) {
    fuzz_lib::fuzz_workflow_parse(data);
}

/// Workflow compiler fuzz target.
pub fn workflow_compile(data: &[u8]) {
    fuzz_lib::fuzz_workflow_compile(data);
}

/// SlotValue postcard roundtrip fuzz target.
pub fn slot_value_roundtrip(data: &[u8]) {
    fuzz_lib::fuzz_slot_value_roundtrip(data);
}

/// Binary IPC frame fuzz target.
pub fn binary_ipc_frame(data: &[u8]) {
    fuzz_lib::fuzz_binary_ipc_frame(data);
}

/// Journal record envelope fuzz target.
pub fn journal_record(data: &[u8]) {
    fuzz_lib::fuzz_journal_record(data);
}

/// Expression lexer/parser/compiler/evaluator fuzz target.
pub fn expression(data: &[u8]) {
    fuzz_lib::fuzz_expression(data);
}
