//! Property test modules for vb_compile.
//!
//! Each submodule covers one Master plan §38 property and is gated by
//! `#[cfg(test)]` at the parent crate root.

pub(crate) mod bytecode_ast_parity;
