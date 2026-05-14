//! Compile-fail: generated code must not contain `unsafe` blocks.
//!
//! The generated Rust workflow header includes `#![forbid(unsafe_code)]`.
//! Any `unsafe` block in generated output must be rejected at compile time.

#![forbid(unsafe_code)]
#![deny(unused_must_use)]

fn main() {
    let x: i32 = 42;
    let _y: i32 = unsafe { x + 1 };
}
