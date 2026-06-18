#![forbid(unsafe_code)]
//! Node-level validation sub-modules.
//!
//! - **[`bounds`]** — Primitive index bounds validators (step, slot, expr, const, accessor).
//! - **[`common`]** — Shared node field validators (optional slots/steps, build-list/object, loops).
//! - **[`kinds`]** — Exhaustive [`CompiledNodeKind`] dispatch for per-variant validation.
//! - **[`branch_tables`]** — Branch-table "must have at least one target" invariant.

pub(crate) mod bounds;
pub(crate) mod branch_tables;
pub(crate) mod common;
pub(crate) mod kinds;
