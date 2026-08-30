#![forbid(unsafe_code)]
//! Semantic observation normalization module.
//!
//! Normalizes raw `JournalEvent` streams into stable, schema-versioned
//! `JournalObservation` lists with BLAKE3 digests for fast comparison.
//!
//! Modules:
//! - `types`: Observation type definitions
//! - `helpers`: Utility functions for observation construction
//! - `action`: Action event observation handling
//! - `normalize`: Core normalization logic

pub mod action;
pub mod helpers;
pub mod normalize;
pub mod types;

// Re-exports
pub use normalize::{semantic_observation_signature, semantic_observations};
pub use types::{
    AskObservation, DigestObservation, DigestSubject, JournalObservation,
    JournalObservationSignature, LifecycleObservation, ObservationSignatureError, SlotObservation,
    StepObservation, TerminalObservation, TimerObservation, WaitObservation,
    SEMANTIC_OBSERVATION_SCHEMA_VERSION,
};
