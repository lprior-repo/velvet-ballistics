//! Compile-fail: generated code must not call `.unwrap()`.
//!
//! The generated Rust workflow returns `Result` everywhere and propagates
//! errors with `?`. This test verifies that the codegen helper type
//! `DriveResult` intentionally omits `.unwrap()`, so any generated code
//! attempting to call it would fail to compile.

#![forbid(unsafe_code)]
#![deny(unused_must_use)]

/// A Result-like type that only exposes safe, fallible access.
/// The codegen output uses this pattern to make `.unwrap()` unavailable.
pub enum DriveResult<T, E> {
    Ok(T),
    Err(E),
}

impl<T, E> DriveResult<T, E> {
    pub fn ok(self) -> Option<T> {
        match self {
            DriveResult::Ok(v) => Some(v),
            DriveResult::Err(_) => None,
        }
    }

    pub fn err(self) -> Option<E> {
        match self {
            DriveResult::Ok(_) => None,
            DriveResult::Err(e) => Some(e),
        }
    }
}

fn main() {
    let res: DriveResult<i32, &str> = DriveResult::Ok(42);
    let _val: i32 = res.unwrap();
}
