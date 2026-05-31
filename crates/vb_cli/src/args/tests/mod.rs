use crate::args::{
    ActionRegistryMode, Command, DurabilityMode, EmitTarget, OutputFormat, ParseError, StepTarget,
    VerifyProfile, parse_args,
};
use std::ffi::OsString;
use std::path::PathBuf;

pub fn args(parts: &[&str]) -> Vec<OsString> {
    parts.iter().map(|part| OsString::from(*part)).collect()
}

mod parse_run;
mod parse_workflow;
mod parse_other;
mod parse_misc;
mod parse_misc2;
