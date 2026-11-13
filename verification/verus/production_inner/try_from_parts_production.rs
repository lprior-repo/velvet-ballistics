// SPDX-License-Identifier: MIT
// Native binding to the complete production workflow constructor and types.
// Source: crates/vb_core/src/workflow/compiled.rs:47-49
// Shared checked construction: crates/vb_core/src/workflow/profile_construct.rs:9-22
// Source: crates/vb_core/src/workflow/parts.rs:11-46
// Source: crates/vb_core/src/workflow/error.rs:13-265
// No validation is reimplemented here. The actual crate executes its slot-flow,
// recursive input, source metadata, retry, resource, and ownership checks.
// The existing external proof bridge remains unproved; native linkage does not
// give Verus contracts to these production functions.
pub use vb_core::*;
pub use vb_core::workflow::*;
