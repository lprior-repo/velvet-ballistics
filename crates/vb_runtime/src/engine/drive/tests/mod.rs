#![forbid(unsafe_code)]

//! Drive-loop behavior tests, split by family.
//!
//! Helper construction functions live in [`common`]; each child module
//! covers one behavior family and re-uses those helpers via
//! `super::common`.

mod cat_advanced;
mod cat_basics;
mod cat_branch;
mod cat_dwa;
mod cat_evidence;
mod cat_extras;
mod cat_regression;
mod common;