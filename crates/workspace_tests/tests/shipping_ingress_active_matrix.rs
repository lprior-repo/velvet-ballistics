#![forbid(unsafe_code)]

#[path = "shipping_ingress_active_matrix/harness.rs"]
mod harness;
#[path = "shipping_ingress_active_matrix/support.rs"]
mod support;

#[path = "shipping_ingress_active_matrix/controls.rs"]
mod controls;
#[path = "shipping_ingress_active_matrix/ipc.rs"]
mod ipc;
#[path = "shipping_ingress_active_matrix/retries.rs"]
mod retries;
