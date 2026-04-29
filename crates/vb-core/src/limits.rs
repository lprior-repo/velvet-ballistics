#![forbid(unsafe_code)]

//! Compile-time resource limits for the hot runtime.
//!
//! These constants establish hard boundaries used in both the compiler (as upper
//! bounds during validation) and the runtime (for allocation and overflow checks).
//! Changing any value constitutes a protocol change requiring a major version bump.

/// Maximum number of steps allowed in a single compiled workflow.
///
pub const MAX_STEPS_PER_WORKFLOW: usize = 65_535;

/// Maximum number of named slots that may be live within a single step activation.
///
pub const MAX_SLOTS_PER_STEP: usize = 256;

/// Maximum size of the constant pool in a compiled workflow.
///
pub const MAX_CONSTANTS: usize = 65_535;

/// Maximum recursive expression-evaluation depth (for safety in the bytecode engine).
///
pub const MAX_EXPRESSION_DEPTH: usize = 64;

/// Maximum byte-length of a run name string supplied by the caller.
///
pub const MAX_RUN_NAME_LENGTH: usize = 1_024;
