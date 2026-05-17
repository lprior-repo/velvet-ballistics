//! Red-phase contract tests for bead vb-37lc canonical spelling scan.
//!
//! Tests are split by drift polish so each physical Rust file stays under 300 lines.

#![forbid(unsafe_code)]

mod vb_37lc_canonical_spelling_red {
    mod common;
    pub(crate) use common::*;
    mod part_01;
    mod part_02;
    mod part_03;
    mod part_04;
    mod part_05;
    mod part_06;
    mod part_07;
}
