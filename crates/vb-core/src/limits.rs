#![forbid(unsafe_code)]

//! Compile-time resource limits for the hot runtime.
//!
//! These constants establish hard boundaries used in both the compiler (as upper
//! bounds during validation) and the runtime (for allocation and overflow checks).
//! Changing any value constitutes a protocol change requiring a major version bump.

/// Maximum number of steps allowed in a single compiled workflow.
///
pub const MAX_STEPS_PER_WORKFLOW: usize = 65_535;

/// Maximum number of slots allowed in a single compiled workflow.
///
pub const MAX_SLOTS_PER_WORKFLOW: usize = 65_535;

/// Maximum number of named slots that may be live within a single step activation.
///
pub const MAX_SLOTS_PER_STEP: usize = 256;

/// Maximum size of the constant pool in a compiled workflow.
///
pub const MAX_CONSTANTS: usize = 65_535;

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
