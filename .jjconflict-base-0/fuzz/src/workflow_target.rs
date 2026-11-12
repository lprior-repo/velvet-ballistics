//! Workflow compilation, IR, and resource budget fuzzing targets.
//
// The strict fuzz clippy denies `indexing_slicing`, `as_conversions`,
// `let_underscore_must_use`, and `arithmetic_side_effects`. The broad
// `#![allow(...)]` lines that previously suppressed those lints have been
// removed so the strict gate is enforceable. The remaining allows are
// documentary lints the strict command does not deny.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::len_zero)]

mod accessor;
mod budget;
mod collect;
mod compiled;
mod generated;
mod node_slots;
mod step_budget;
mod values;

pub use accessor::fuzz_accessor_traversal;
pub use budget::{fuzz_budget_compute, fuzz_resource_budget};
pub use collect::fuzz_collect_page_pagination;
pub use compiled::fuzz_compiled_ir;
pub use generated::fuzz_generated_compare;
pub use step_budget::fuzz_step_budget_new;
pub use values::fuzz_slot_value_roundtrip;
