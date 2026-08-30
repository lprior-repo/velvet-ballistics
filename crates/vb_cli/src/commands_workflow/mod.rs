#![forbid(unsafe_code)]
//! Pure workflow analysis logic for graph and simulate commands.
//!
//! Both `graph` and `simulate` are **static preflight** operations:
//! they read a `CompiledWorkflow` and produce analysis output without
//! executing the workflow, mutating state, or accessing storage.
//!
//! See `simulate` module docs for the full boundary definition between
//! static preflight (this module) and live runtime execution (`vb_runtime`).

mod dot;
mod helpers;
mod simulate;

pub(crate) use dot::generate_dot;
pub(crate) use simulate::simulate_workflow;
