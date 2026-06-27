//! Validation fuzzing targets.
#![allow(clippy::indexing_slicing)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::let_underscore_must_use)]
#![allow(clippy::as_conversions)]
#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::len_zero)]

mod capability;
mod diagnostic;
mod verifier;

pub use capability::{fuzz_capability_contract_schema, fuzz_capability_name_schema};
pub use diagnostic::{fuzz_diagnostic_code_from_str, fuzz_diagnostic_from_error};
pub use verifier::fuzz_verifier_gates;
