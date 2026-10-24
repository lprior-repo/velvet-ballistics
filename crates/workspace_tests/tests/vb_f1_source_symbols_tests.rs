#![forbid(unsafe_code)]
//! Bind the unregistered interner draft directly for standalone native checks.
//! This does not enable source text, workflow metadata, or runtime prefix setup.

#[path = "../../vb_compile/src/source_symbols.rs"]
mod source_symbols;
