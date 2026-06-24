#![forbid(unsafe_code)]
//! Pure workflow analysis logic for graph and simulate commands.

mod dot;
mod helpers;
mod simulate;

#[cfg(test)]
pub(crate) use dot::DotGraph;
pub(crate) use dot::generate_dot;
pub(crate) use simulate::simulate_workflow;
#[cfg(test)]
pub(crate) use simulate::{SimulationResult, SimulationStep};
