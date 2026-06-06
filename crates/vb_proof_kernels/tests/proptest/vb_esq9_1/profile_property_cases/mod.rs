//! Cohesive property groups for profile contract validation.

mod binding;
mod gap_detection;
mod inheritance;
mod pure_core;
mod strategies;

pub(crate) use strategies::{arb_correct_workspace, arb_workspace_profile_set};
