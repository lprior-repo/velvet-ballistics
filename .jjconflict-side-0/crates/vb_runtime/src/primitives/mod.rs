#![forbid(unsafe_code)]
//! Iteration and compound primitive handlers.

pub mod collect;
pub mod for_each;
pub(crate) mod helpers;
pub mod reduce;
pub mod repeat;
pub mod retry;
pub mod together;
pub mod wait_ask;

#[cfg(kani)]
pub mod reentry_proofs;

#[cfg(test)]
mod reentry_tests;
