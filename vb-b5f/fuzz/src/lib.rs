//! Fuzz library — shared fuzzing utilities for Velvet Ballistics
//!
//! Phase 0 creates the scaffold. The actual fuzzing implementations
//! will be added in later phases.

#![cfg(feature = "fuzz")]

use std::slice;

/// Shared corpus type for fuzzing
pub type Corpus = Vec<u8>;

/// Shared mutation placeholder — real fuzzing uses libfuzzer's built-in engine
pub fn fuzzer_mutate(corpus: &Corpus, _seed: u32) -> Corpus {
    let _ = _seed;
    corpus.clone()
}

/// Entry point for libfuzzer — dispatches to the workflow_parse fuzzer
#[unsafe(no_mangle)]
extern "C" fn fuzz_workflow_parse(data: *const u8, size: usize) -> i32 {
    let corpus = unsafe { slice::from_raw_parts(data, size).to_vec() };
    let _ = fuzzer_mutate(&corpus, 0);
    0
}

/// Entry point for libfuzzer — dispatches to the workflow_compile fuzzer
#[unsafe(no_mangle)]
extern "C" fn fuzz_workflow_compile(data: *const u8, size: usize) -> i32 {
    let corpus = unsafe { slice::from_raw_parts(data, size).to_vec() };
    let _ = fuzzer_mutate(&corpus, 1);
    0
}

/// Entry point for libfuzzer — dispatches to the slot_value_roundtrip fuzzer
#[unsafe(no_mangle)]
extern "C" fn fuzz_slot_value_roundtrip(data: *const u8, size: usize) -> i32 {
    let corpus = unsafe { slice::from_raw_parts(data, size).to_vec() };
    let _ = fuzzer_mutate(&corpus, 2);
    0
}

/// Entry point for libfuzzer — dispatches to the binary_ipc_frame fuzzer
#[unsafe(no_mangle)]
extern "C" fn fuzz_binary_ipc_frame(data: *const u8, size: usize) -> i32 {
    let corpus = unsafe { slice::from_raw_parts(data, size).to_vec() };
    let _ = fuzzer_mutate(&corpus, 3);
    0
}

/// Entry point for libfuzzer — dispatches to the fjall_journal_append fuzzer
#[unsafe(no_mangle)]
extern "C" fn fuzz_fjall_journal_append(data: *const u8, size: usize) -> i32 {
    let corpus = unsafe { slice::from_raw_parts(data, size).to_vec() };
    let _ = fuzzer_mutate(&corpus, 4);
    0
}
