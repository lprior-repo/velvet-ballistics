use std::ffi::OsString;

fn args(parts: &[&str]) -> Vec<OsString> {
    parts.iter().map(|part| OsString::from(*part)).collect()
}

mod parse_misc;
mod parse_misc2;
mod parse_other;
mod parse_run;
mod parse_workflow;
