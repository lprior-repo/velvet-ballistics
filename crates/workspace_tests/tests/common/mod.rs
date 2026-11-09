//! Shared integration-test fixtures for workspace_tests targets.
//!
//! Each test root wires this via `#[path = "common/mod.rs"] mod common;`
//! (vb_core et al are dev-dependencies, invisible to the lib target, so the
//! harness lives here rather than in `src/`). Builders preserve workflow
//! bytes exactly; callers pass name/digest/action parameters.

pub(crate) mod workflow_fixture;
