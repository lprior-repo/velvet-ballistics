//! Fuzz target stubs for Velvet Ballistics
//!
//! Phase 0 creates the scaffold. Each fuzzer body is a stub that will
//! be implemented in subsequent phases.

/// Corpus type for fuzzing — byte array representing valid input
type Corpus = Vec<u8>;

/// Generate corpus by mutating the input data.
/// This is a placeholder that will be expanded in later phases.
fn fuzzer_mutate(corpus: &Corpus, seed: u32) -> Corpus {
    let _ = seed;
    corpus.clone()
}

fn run_stub(seed: u32) -> i32 {
    let corpus = Corpus::new();
    let _ = fuzzer_mutate(&corpus, seed);
    0
}

/// Fuzz target: workflow_parse
///
/// Fuzzes: saphyr YAML parser → Workflow AST
/// Entry point: LLVMFuzzerTestOneInput
///
/// Input: Raw YAML bytes representing a workflow definition
/// Pass: Parses successfully into a Workflow struct
/// Fail: Panics or returns non-zero (handled by libfuzzer)
#[no_mangle]
extern "C" fn fuzz_workflow_parse(data: *const u8, size: usize) -> i32 {
    let _ = (data, size);
    run_stub(0)
}

/// Fuzz target: workflow_compile
///
/// Fuzzes: Workflow AST → Compiled IR → Validation
/// Entry point: LLVMFuzzerTestOneInput
///
/// Input: Raw YAML bytes representing a workflow definition
/// Pass: Parses and compiles successfully
/// Fail: Compile error or validation failure
#[no_mangle]
extern "C" fn fuzz_workflow_compile(data: *const u8, size: usize) -> i32 {
    let _ = (data, size);
    run_stub(1)
}

/// Fuzz target: slot_value_roundtrip
///
/// Fuzzes: SlotValue → postcard bytes → SlotValue
/// Entry point: LLVMFuzzerTestOneInput
///
/// Input: Bytes representing a valid SlotValue encoding
/// Pass: Round-trip serialization/deserialization preserves value
/// Fail: Decode error or value mismatch
#[no_mangle]
extern "C" fn fuzz_slot_value_roundtrip(data: *const u8, size: usize) -> i32 {
    let _ = (data, size);
    run_stub(2)
}

/// Fuzz target: binary_ipc_frame
///
/// Fuzzes: Frame encoding/decoding for IPC
/// Entry point: LLVMFuzzerTestOneInput
///
/// Input: Bytes representing a valid IPC frame
/// Pass: Frame encodes and decodes correctly
/// Fail: Decode error or frame validation failure
#[no_mangle]
extern "C" fn fuzz_binary_ipc_frame(data: *const u8, size: usize) -> i32 {
    let _ = (data, size);
    run_stub(3)
}

/// Fuzz target: fjall_journal_append
///
/// Fuzzes: Journal entry append and recovery
/// Entry point: LLVMFuzzerTestOneInput
///
/// Input: Bytes representing a valid journal entry
/// Pass: Entry appends to journal and replays correctly
/// Fail: Corruption detection or replay failure
#[no_mangle]
extern "C" fn fuzz_fjall_journal_append(data: *const u8, size: usize) -> i32 {
    let _ = (data, size);
    run_stub(4)
}
