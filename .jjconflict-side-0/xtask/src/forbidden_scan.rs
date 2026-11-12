#![forbid(unsafe_code)]
#![allow(unreachable_pub)]

// `forbidden-scan` xtask command.
// Scans first-party Rust crates for forbidden unsafe patterns and discipline violations.
// Returns non-zero exit code if any forbidden patterns are found.

mod command;
mod patterns;
mod scan;
mod types;

pub use command::cmd_forbidden_scan;
