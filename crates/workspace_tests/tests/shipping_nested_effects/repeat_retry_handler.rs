#![forbid(unsafe_code)]
//! Cell D: scoped action retry and error handler inside an owned Repeat body.

#[path = "repeat_retry_handler/case.rs"]
mod case;
#[path = "repeat_retry_handler/guards.rs"]
mod guards;
#[path = "repeat_retry_handler/recovery.rs"]
mod recovery;
