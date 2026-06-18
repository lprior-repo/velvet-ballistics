#![forbid(unsafe_code)]
//! Runtime admission submodules.
//!
//! Split from the former monolithic `runtime_admission.rs` into focused
//! modules that separate concerns along the DDD boundaries:
//!
//! - **admission_check** — preflight gates, submit methods, adapters
//! - **admission_policy** — policy evaluation, budget-request building
//! - **admission_result** — error mapping chain (`AggregateBudgetError` →
//!   `AdmissionError` → `RuntimeError`)

mod admission_check;
mod admission_policy;
mod admission_result;
