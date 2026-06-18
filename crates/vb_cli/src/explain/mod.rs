#![forbid(unsafe_code)]
//! Explain command modules.
//!
//! ```text
//! explain/
//! ├── mod.rs          ← module tree & re-exports
//! ├── dispatch.rs     ← cmd_explain orchestrator
//! ├── compile_error.rs ← explain_error (CompileError display)
//! └── reports.rs      ← JSON failure-report builders
//! ```

mod compile_error;
pub(crate) mod dispatch;
pub(crate) mod reports;

pub(crate) use compile_error::explain_error;
pub(crate) use dispatch::cmd_explain;
pub(crate) use reports::{
    explain_compile_failure_report, explain_failure_report, explain_verification_failure_report,
};
