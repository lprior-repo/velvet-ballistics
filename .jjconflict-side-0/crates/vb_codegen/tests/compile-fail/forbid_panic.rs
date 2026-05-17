//! Compile-fail: generated code must not call `panic!()`.
//!
//! The generated Rust workflow enforces a no-panic contract by ensuring all
//! control flow returns `Result`. This test verifies that `panic!()` in a
//! const context (which is how the generated code header validates itself)
//! produces a compile-time error.

#![forbid(unsafe_code)]
#![deny(unused_must_use)]

/// Const assertion: panic in const-eval context is a hard compile error.
/// The codegen output validates its own header by including a similar guard.
const _: () = panic!("generated code must not panic");

fn main() {}
