#![forbid(unsafe_code)]
//! Lifecycle command surface for bead state management.
//!
//! This module provides the CLI-facing lifecycle commands (cancel, resume,
//! retry, answer) and the journal replay functionality for state recovery.
//!
//! ## State Machine
//!
//! - `Pending`: Run accepted but not yet active
//! - `Active`: Run is executing
//! - `WaitingAnswer`: Run blocked waiting for external answer
//! - `Cancelled`: Run was cancelled
//! - `Completed`: Run finished successfully
//! - `Failed`: Run encountered an error
//!
//! ## Valid Transitions
//!
//! | From State     | Command | Valid |
//! |----------------|---------|-------|
//! | Active         | Cancel  | Yes   |
//! | WaitingAnswer  | Cancel  | Yes   |
//! | WaitingAnswer  | Resume  | Yes   |
//! | Failed         | Retry   | Yes   |
//! | WaitingAnswer  | Answer  | Yes   |

mod handlers;
pub(super) mod state;

// Re-export public API
#[doc(hidden)]
pub use handlers::test_helpers;
pub use handlers::{answer, cancel, resume, retry};
pub use state::{LifecycleResult, replay};
