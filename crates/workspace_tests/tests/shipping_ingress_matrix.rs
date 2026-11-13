//! vb-eirmk.10: source primitive shipping-ingress behavioral matrix.
//!
//! Each scenario compiles authored YAML with the production compiler,
//! persists an accepted artifact in a real Fjall journal, drives a real
//! runtime, asserts exact durable terminal evidence, and where applicable
//! drops both handles and reopens through public recovery.

#![forbid(unsafe_code)]

#[path = "shipping_ingress_matrix/failure_matrix.rs"]
mod failure_matrix;
#[path = "shipping_ingress_matrix/ingress_matrix.rs"]
mod ingress_matrix;
#[path = "shipping_ingress_matrix/source_matrix.rs"]
mod source_matrix;
#[path = "shipping_ingress_matrix/support.rs"]
mod support;
#[path = "shipping_ingress_matrix/suspension_matrix.rs"]
mod suspension_matrix;
