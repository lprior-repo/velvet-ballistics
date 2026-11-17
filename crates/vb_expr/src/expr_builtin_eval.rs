#![forbid(unsafe_code)]
//! Public expression operator boundary regressions.

use vb_core::SlotValue;

#[cfg(test)]
#[path = "expr_builtin_eval/blackhat_tests.rs"]
mod blackhat_tests;
