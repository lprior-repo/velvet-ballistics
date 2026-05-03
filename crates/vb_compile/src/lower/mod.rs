//! Lowering logic module.
//!
//! This module contains the `SlotCompiler` and `lower_*` functions.
//! The actual implementation is in the parent module (lib.rs).

pub use crate::{
    lower_ask, lower_choose, lower_collect, lower_do, lower_finish, lower_for_each,
    lower_reduce, lower_repeat, lower_set, lower_steps_to_ir, lower_together, lower_wait,
    SlotCompiler, WaitKind,
};
