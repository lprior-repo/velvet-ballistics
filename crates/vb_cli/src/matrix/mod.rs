//! Source-of-truth matrix for the 30-command CLI surface (master §33.3).
//!
//! Each submodule reads one of the six implementation-side sources of truth
//! defined for the 30-command CLI matrix and exposes a uniform API for the
//! conformance proptest in
//! `crates/workspace_tests/tests/cli_matrix_conformance.rs`.
//!
//! This module is `#![forbid(unsafe_code)]` and contains no `unwrap`/`expect`
//! in production code paths; the unit tests in each submodule may use
//! `expect`/`unwrap` only inside `#[test]` bodies.

#![forbid(unsafe_code)]

pub mod source_agent_context_json;
pub mod source_command_enum;
pub mod source_help_string;
pub mod source_parse_args;
pub mod source_run_from_env_dispatch;
pub mod source_valid_commands;
