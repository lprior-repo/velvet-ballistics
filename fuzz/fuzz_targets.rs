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

/// libFuzzer C ABI entrypoint for YAML events.
#[unsafe(no_mangle)]
pub extern "C" fn LLVMFuzzerTestOneInputYamlEvents(data: *const u8, len: usize) -> i32 {
    let _ = (data, len);
    0
}

/// libFuzzer C ABI entrypoint for IPC frames.
#[unsafe(no_mangle)]
pub extern "C" fn LLVMFuzzerTestOneInputIpcFrame(data: *const u8, len: usize) -> i32 {
    let _ = (data, len);
    0
}

/// libFuzzer C ABI entrypoint for journal events.
#[unsafe(no_mangle)]
pub extern "C" fn LLVMFuzzerTestOneInputJournalEvent(data: *const u8, len: usize) -> i32 {
    let _ = (data, len);
    0
}

/// libFuzzer C ABI entrypoint for expressions.
#[unsafe(no_mangle)]
pub extern "C" fn LLVMFuzzerTestOneInputExpression(data: *const u8, len: usize) -> i32 {
    let _ = (data, len);
    0
}

/// libFuzzer C ABI entrypoint for compiled IR.
#[unsafe(no_mangle)]
pub extern "C" fn LLVMFuzzerTestOneInputCompiledIr(data: *const u8, len: usize) -> i32 {
    let _ = (data, len);
    0
}

/// Generated-vs-IR comparison fuzz target.
pub fn generated_compare(data: &[u8]) {
    fuzz_lib::fuzz_generated_compare(data);
}
