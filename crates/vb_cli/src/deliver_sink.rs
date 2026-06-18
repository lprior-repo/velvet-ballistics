//! Deliver sink — writes a single JSON line to stdout or atomically to a file.
//!
//! This module is the public boundary for the CLI's deliver pipeline. It
//! re-exports types and functions from internal sub-modules so callers
//! never need to reach into the implementation details.
//!
//! ```text
//! deliver_sink.rs (this file)
//! ├── deliver_error.rs    — DeliverSinkError, constants, temp staging types
//! ├── deliver_target.rs   — DeliverTarget, DeliverFileTarget, parse_deliver_target
//! ├── atomic_publish.rs   — write_json_line + full publish lifecycle
//! ├── deliver_test_support.rs — test_support hooks (cfg(test))
//! ├── deliver_debug_test_support.rs — debug hooks (instrumented-cli)
//! └── deliver_test.rs     — unit tests
//! ```

pub use deliver_error::PUBLISH_STATE_UNKNOWN_MESSAGE;
pub(crate) use deliver_error::DeliverSinkError;
pub(crate) use deliver_target::{DeliverFileTarget, DeliverTarget, parse_deliver_target};
pub(crate) use atomic_publish::write_json_line;

// Internal sub-modules (not re-exported publicly)
mod deliver_error;
mod deliver_target;
mod atomic_publish;
#[cfg(test)]
mod deliver_test_support;
#[cfg(all(not(test), feature = "instrumented-cli"))]
mod deliver_debug_test_support;

#[cfg(test)]
mod deliver_test;
