#![cfg(all(kani, feature = "vb-god2f-action-completion"))]
#![forbid(unsafe_code)]

//! HVR-PO-RUNTIME-{001,002,006}: production-bound Kani harnesses for
//! action dispatch input limits, PC advancement, and scheduled-attempt state.

mod input_boundaries;
mod model;
mod pc_completion;
mod scheduled_attempt;
