#![forbid(unsafe_code)]
//! Async journal writer queue and batch builder.
//!
//! Provides bounded queueing for journal events with durability profiling.

mod batch;
mod writer;

#[cfg(test)]
mod tests;

pub use batch::BatchBuilder;
pub use writer::JournalWriterQueue;
