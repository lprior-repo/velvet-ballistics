#![forbid(unsafe_code)]

//! Compile-time resource limits for the hot runtime.
//!
//! These constants establish hard boundaries used in both the compiler (as upper
//! bounds during validation) and the runtime (for allocation and overflow checks).
//! Changing any value constitutes a protocol change requiring a major version bump.

/// Maximum number of steps allowed in a single compiled workflow.
///
/// Master contract: `velvet-ballistics-MASTER.md` §13 line 479 (Steps | 1000).
pub const MAX_STEPS_PER_WORKFLOW: usize = 1_000;

/// Maximum number of slots allowed in a single compiled workflow.
///
pub const MAX_SLOTS_PER_WORKFLOW: usize = 65_535;

/// Maximum number of named slots that may be live within a single step activation.
///
pub const MAX_SLOTS_PER_STEP: usize = 256;

/// Maximum size of the constant pool in a compiled workflow.
///
/// Master contract: `velvet-ballistics-MASTER.md` §13 line 483 (Constants | 8192).
pub const MAX_CONSTANTS: usize = 8_192;

/// Maximum recursive expression-evaluation depth (for safety in the bytecode engine).
///
pub const MAX_EXPRESSION_DEPTH: usize = 64;

/// Maximum number of bytecode operations allowed in one expression program.
///
pub const MAX_EXPRESSION_OPS: usize = 256;

/// Maximum number of expression programs in one compiled workflow.
///
pub const MAX_EXPRESSIONS: usize = 4_096;

/// Maximum number of accessor programs in one compiled workflow.
///
pub const MAX_ACCESSORS: usize = 8_192;

/// Maximum stack entries allowed while evaluating one expression program.
///
pub const MAX_EXPRESSION_STACK: u8 = 64;

/// `usize` form of [`MAX_EXPRESSION_STACK`] for fixed-size runtime scratch arrays.
///
pub const MAX_EXPRESSION_STACK_USIZE: usize = 64;

/// Maximum byte-length of a run name string supplied by the caller.
///
pub const MAX_RUN_NAME_LENGTH: usize = 1_024;

/// Maximum number of bytecode operations per compiled expression.
///
pub const MAX_BYTECODE_OPS_PER_EXPRESSION: usize = 256;

/// Maximum depth of accessor path segments.
///
pub const MAX_PATH_DEPTH: usize = 16;

/// Maximum nesting depth for language constructs (for_each, together, etc.).
///
pub const MAX_LANGUAGE_NESTING_DEPTH: u8 = 8;

/// Maximum number of slots in a single run frame.
///
pub const MAX_SLOTS: u16 = u16::MAX;

/// Maximum items in one runtime list arena value.
///
pub const MAX_LIST_ITEMS_PER_VALUE: usize = 65_535;

/// Maximum fields in one runtime object arena value.
///
pub const MAX_OBJECT_FIELDS_PER_VALUE: usize = 65_535;

/// Maximum bytes in one interned runtime symbol.
///
pub const MAX_SYMBOL_BYTES_PER_VALUE: usize = 4_096;

/// Maximum bytes in one runtime blob arena value.
///
pub const MAX_BLOB_BYTES_PER_VALUE: usize = 16_777_216;

/// Maximum total arena values (symbols + lists + objects + blobs) per run.
///
/// This cap prevents unbounded memory growth from nested ForEach x Together
/// compositions where individual value limits are respected but total
/// count is not bounded.
pub const MAX_VALUES_PER_RUN: usize = 1_000_000;

/// Maximum deterministic transitions per runtime tick.
///
pub const MAX_STEP_BUDGET: u64 = 10_000;

/// Maximum input bytes accepted at admission (hard limit).
///
pub const MAX_INPUT_BYTES: u32 = 16_777_216;

/// Maximum output bytes produced by a run (hard limit).
///
pub const MAX_OUTPUT_BYTES: u32 = 16_777_216;

/// Maximum blob payload bytes (hard limit).
///
pub const MAX_BLOB_BYTES: u64 = 67_108_864;

/// Maximum IPC payload bytes (hard limit).
///
pub const MAX_IPC_PAYLOAD_BYTES: u32 = 16_777_216;

/// Maximum retry attempts for action policies (hard limit).
///
pub const MAX_RETRY_ATTEMPTS: u16 = 10;

/// Maximum branch fanout (hard limit).
///
pub const MAX_FANOUT: u16 = 256;

/// Maximum collect items (hard limit).
///
pub const MAX_COLLECT_ITEMS: u32 = 1_048_576;

/// Maximum runtime queue depth (hard limit).
///
pub const MAX_QUEUE_DEPTH: u32 = 1_048_576;

/// Maximum journal batch bytes (hard limit).
///
pub const MAX_JOURNAL_BATCH_BYTES: u32 = 16_777_216;

#[cfg(test)]
#[path = "limits/tests.rs"]
mod tests;
