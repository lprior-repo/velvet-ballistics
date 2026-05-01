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
