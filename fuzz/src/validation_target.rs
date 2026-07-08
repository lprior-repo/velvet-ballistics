//! Validation fuzzing targets.
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

mod capability;
mod diagnostic;
mod verifier;

pub use capability::{fuzz_capability_contract_schema, fuzz_capability_name_schema};
pub use diagnostic::{fuzz_diagnostic_code_from_str, fuzz_diagnostic_from_error};
pub use verifier::fuzz_verifier_gates;
