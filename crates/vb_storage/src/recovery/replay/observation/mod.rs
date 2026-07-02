#![forbid(unsafe_code)]
#![allow(dead_code)]
//! Semantic observation module for journal recovery.
//!
//! Projects journal events into a stable, semantic-only schema so two
//! equivalent runs produce identical observation digests while
//! divergent runs diverge in a detectable way.
//!
//! Layout (split for source-length discipline):
//! - [`types`]: thin re-export module over the schema sub-modules.
//! - [`subject`]: `DigestSubject` + `DigestObservation` + schema version.
//! - [`lifecycle`]: lifecycle / step / wait / timer / terminal observations.
//! - [`action_types`]: action observation types (state, outcome, struct).
//! - [`ask`]: ask / answer / slot observations.
//! - [`signature`]: `JournalObservation` enum + signature struct.
//! - [`action`]: action-event projection dispatcher.
//! - [`digest`]: digest + slot-observation helpers.
//! - [`encode`]: canonical BLAKE3 encoders.
//! - [`policy`]: policy / taint byte-tag projections.
//! - [`helpers`]: thin re-export module over the helper sub-modules.
//! - [`normalize`]: top-level `observe_journal` pipeline.

pub(crate) mod action;
pub(crate) mod action_types;
pub(crate) mod ask;
pub(crate) mod digest;
pub(crate) mod encode;
pub(crate) mod encode_action;
pub(crate) mod encode_ask;
pub(crate) mod helpers;
pub(crate) mod lifecycle;
pub(crate) mod normalize;
pub(crate) mod normalize_push;
pub(crate) mod policy;
pub(crate) mod signature;
pub(crate) mod subject;
pub(crate) mod types;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
