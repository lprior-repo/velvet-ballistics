//! Iteration and compound primitive handlers.

pub mod collect;
pub mod for_each;
pub(crate) mod helpers;
pub mod reduce;
pub mod repeat;
pub mod together;
pub mod wait;
pub mod ask;
pub mod wait_ask;
#[cfg(test)]
mod ask_tests;
#[cfg(test)]
mod wait_tests;
