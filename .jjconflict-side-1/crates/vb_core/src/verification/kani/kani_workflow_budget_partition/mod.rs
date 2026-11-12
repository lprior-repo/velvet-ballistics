//! Exhaustive replacement family for KANI-BUDGET-001's projected input domain.
//! Each case executes production compute; the bridge retains the original visitors.

mod bridge;
mod cases;
mod constructors;
mod domain;
mod node_equality;
mod sampling;
mod witness;
mod word;
