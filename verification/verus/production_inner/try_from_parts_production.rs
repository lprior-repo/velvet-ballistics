// SPDX-License-Identifier: MIT
// Partial LegacyV2 binding to production workflow constructor and types.
// Legacy constructor: crates/vb_core/src/workflow/compiled.rs:47-49
// Production `crates/vb_core/src/workflow/compiled.rs:47-49`
// (delegates to shared profile construction:
//  crates/vb_core/src/workflow/profile_construct.rs:9-22)
// Full CompiledWorkflow struct: crates/vb_core/src/workflow/compiled.rs:24-43
// Full WorkflowParts struct: crates/vb_core/src/workflow/parts.rs:11-46
// Production `crates/vb_core/src/workflow/parts.rs:11-46`
// Full WorkflowError enum: crates/vb_core/src/workflow/error.rs:13-274
// Production `crates/vb_core/src/workflow/error.rs:13-274`
// (including concurrent-lane variant at :268-273, not reachable via legacy constructor)
// This facade is a LegacyV2 partial projection only; it does not establish
// ConcurrentV3 behavior or complete native type/error surface parity.
// No validation is reimplemented here. The actual crate executes its slot-flow,
// recursive input, source metadata, retry, resource, and ownership checks.
// The existing external proof bridge remains unproved; native linkage does not
// give Verus contracts to these production functions.
pub use vb_core::*;
pub use vb_core::workflow::*;
