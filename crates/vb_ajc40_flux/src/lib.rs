#![forbid(unsafe_code)]

//! Crate-wired Flux refinement harness for vb-ajc40.
//!
//! This verification-only crate depends on `vb_core` and calls the production
//! post-decode validation seams before exposing scalar refinements that Flux can
//! prove. It has no runtime role in production behavior.

#[cfg(feature = "positive")]
pub mod positive;
#[cfg(feature = "negative")]
pub mod negative;
