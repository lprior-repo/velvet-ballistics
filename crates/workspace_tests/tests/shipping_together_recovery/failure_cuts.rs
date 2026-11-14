#![forbid(unsafe_code)]
//! Cold recovery at real Together branch action failure and retry boundaries.

#[path = "failure_cuts/retry.rs"]
mod retry;
#[path = "failure_cuts/routing.rs"]
mod routing;
#[path = "failure_cuts/support.rs"]
mod support;
