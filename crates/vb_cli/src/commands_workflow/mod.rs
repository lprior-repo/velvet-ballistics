#![forbid(unsafe_code)]
//! Pure workflow analysis logic for graph and simulate commands.

mod dot;
mod helpers;
mod simulate;

pub use dot::{generate_dot, DotGraph};
pub use simulate::{simulate_workflow, SimulationResult, SimulationStep};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
