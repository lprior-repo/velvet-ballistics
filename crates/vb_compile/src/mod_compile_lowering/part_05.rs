#![allow(unused_imports)]

#[path = "part_05_digest.rs"]
mod digest;
#[path = "part_05_ir.rs"]
mod ir;
#[path = "part_05_utility.rs"]
mod utility;

pub use digest::canonical_digest;
pub(crate) use digest::{canonical_primitive_name, digest_step_primitive, validate_branch_counts};
pub use ir::{lower_do, lower_set, lower_steps_to_ir};
pub(super) use utility::{
    StepIdxSlotExt, canonical_finish_slot, optional_slot_from_text, parse_i64_field, slot_from_text,
};

// Unit tests for canonical_digest and digest_step_primitive live in a
// separate file to keep this module under the source-length limit.
#[cfg(test)]
#[path = "../tests/digest_unit_tests.rs"]
mod tests;
