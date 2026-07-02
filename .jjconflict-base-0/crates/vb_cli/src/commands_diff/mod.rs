#![forbid(unsafe_code)]
#![allow(unreachable_pub)]
//! Pure diff computation logic, separated from I/O and formatting.
//!
//! The module is decomposed across focused submodules:
//!
//! - [`schema`] — FNV-1a schema fingerprint, [`schema::KnownVariant`],
//!   [`schema::KNOWN_VARIANTS`], compile-time guards (static-array-length
//!   idiom — no `assert!` macros in production).
//! - [`event_name`] — canonical [`event_name::event_name`] lookup plus
//!   [`event_name::KnownVariant::name`] /
//!   [`event_name::KnownVariant::try_from_event`].
//! - [`summary`] — human-readable [`summary::summary_text`] (digest hex
//!   truncation for `RunAdmission`).
//! - [`diff`] — [`diff::DiffResult`], [`diff::compute_diff`],
//!   [`diff::events_differ`], [`diff::collect_step_outcomes`], and
//!   [`diff::collect_slot_values`].
//! - [`diff_event_summary`] — [`diff_event_summary::diff_event_summary`]
//!   per-event JSON projection (split out of `diff` to keep each
//!   production source under the 300-line limit).
//!
//! All public items are re-exported at the module root so external
//! callers (e.g. `vb_cli::commands_diff::event_name`) continue to work
//! unchanged.

// `JournalEvent` is re-exported here so `commands_diff::tests` can
// construct `JournalEvent::*` literals via `use super::*;` without
// adding a second `use vb_storage::events::JournalEvent;` line. The
// `#[allow(unused_imports)]` silences a false-positive warning in
// non-`cfg(test)` compilations where `mod tests` is excluded.
#[allow(unused_imports)]
use vb_storage::events::JournalEvent;

mod diff;
mod diff_event_summary;
mod event_name;
mod schema;
mod summary;

// Public re-exports for the CLI surface. The `pub use` keeps
// `vb_cli::commands_diff::{compute_diff, diff_event_summary, event_name,
// events_differ, collect_step_outcomes, collect_slot_values, summary_text,
// DiffResult}` paths identical to the pre-split layout, which is
// required by `incident_ops.rs`, `replay/mod.rs`, and the
// `workspace_tests` integration suite.
//
// The `#[allow(unused_imports)]` is a targeted, justified exception:
// rustc's `unused_imports` lint fires on these `pub use` statements when
// the crate is type-checked in isolation (the cross-crate consumers
// `workspace_tests`, `incident_ops`, and `replay` are not visible to
// rustc when checking only `vb_cli`). The re-exports are verified to be
// referenced by those consumers at the workspace-check level
// (`cargo check --workspace --all-targets --all-features`).
#[allow(unused_imports)]
pub use diff::collect_slot_values;
#[allow(unused_imports)]
pub use diff::collect_step_outcomes;
#[allow(unused_imports)]
pub use diff::compute_diff;
#[allow(unused_imports)]
pub use diff::events_differ;
#[allow(unused_imports)]
pub use diff_event_summary::diff_event_summary;
#[allow(unused_imports)]
pub use event_name::event_name;
#[allow(unused_imports)]
pub use summary::summary_text;

// Internal re-exports so `commands_diff::tests` (which uses `super::*`)
// can keep referencing the closed `KnownVariant` enum, the
// `KNOWN_VARIANTS` slice, and the FNV-1a fingerprint constants without
// rewriting every test to use `super::schema::...`. These are
// `pub(crate)` (already that way on the original definitions) so the
// re-exports themselves are crate-internal.
#[cfg(test)]
pub(crate) use schema::{EXPECTED_SCHEMA_HASH, SCHEMA_HASH};
#[cfg(test)]
pub(crate) use schema::{KNOWN_VARIANTS, KnownVariant};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
