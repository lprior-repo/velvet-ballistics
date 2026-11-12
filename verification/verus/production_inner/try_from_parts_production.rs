// SPDX-License-Identifier: MIT
// Native binding to the complete production workflow constructor and types.
// Source: crates/vb_core/src/workflow/compiled.rs:35-71
// Source: crates/vb_core/src/workflow/parts.rs:11-46
// Source: crates/vb_core/src/workflow/error.rs:13-250
// No validation is reimplemented here. The actual crate executes its slot-flow,
// recursive input, source metadata, retry, resource, and ownership checks.
// The existing external proof bridge remains unproved; native linkage does not
// give Verus contracts to these production functions.
pub use vb_core::*;
pub use vb_core::workflow::*;
