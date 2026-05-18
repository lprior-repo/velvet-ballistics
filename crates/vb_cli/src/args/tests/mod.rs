use std::ffi::OsString;

fn args(parts: &[&str]) -> Vec<OsString> {
    parts.iter().map(|part| OsString::from(*part)).collect()
}

mod action;
mod cancel;
mod core;
mod journal;
mod observability;
mod run;
mod status;
mod workflow;
