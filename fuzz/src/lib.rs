//! Fuzz library — shared fuzzing utilities for Velvet Ballistics
//!
//! Phase 0 creates the scaffold. The actual fuzzing implementations
//! will be added in later phases.

#![cfg(feature = "fuzz")]

/// Shared corpus type for fuzzing
pub type Corpus = Vec<u8>;

/// Shared mutation placeholder — real fuzzing uses libfuzzer's built-in engine
pub fn fuzzer_mutate(corpus: &Corpus, seed: u32) -> Corpus {
    let _ = seed;
    corpus.clone()
}

fn run_stub(seed: u32) -> i32 {
    let corpus = Corpus::new();
    let _ = fuzzer_mutate(&corpus, seed);
    0
}

/// Entry point for libfuzzer — dispatches to the workflow_parse fuzzer
pub extern "C" fn fuzz_workflow_parse(_data: *const u8, _size: usize) -> i32 {
    run_stub(0)
}

/// Entry point for libfuzzer — dispatches to the workflow_compile fuzzer
pub extern "C" fn fuzz_workflow_compile(data: *const u8, size: usize) -> i32 {
    let _ = (data, size);
    run_stub(1)
}

/// Entry point for libfuzzer — dispatches to the slot_value_roundtrip fuzzer
pub extern "C" fn fuzz_slot_value_roundtrip(data: *const u8, size: usize) -> i32 {
    let _ = (data, size);
    run_stub(2)
}

/// Entry point for libfuzzer — dispatches to the binary_ipc_frame fuzzer
pub extern "C" fn fuzz_binary_ipc_frame(data: *const u8, size: usize) -> i32 {
    let _ = (data, size);
    run_stub(3)
}

/// Entry point for libfuzzer — dispatches to the fjall_journal_append fuzzer
pub extern "C" fn fuzz_fjall_journal_append(data: *const u8, size: usize) -> i32 {
    let _ = (data, size);
    run_stub(4)
}
