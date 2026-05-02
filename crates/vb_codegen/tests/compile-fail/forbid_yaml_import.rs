//! Compile-fail: generated code must not reference YAML runtime.
//!
//! The runtime core is YAML-free by design. Any attempt to import a YAML
//! module in generated code must be rejected.

#![forbid(unsafe_code)]
#![deny(unused_must_use)]

extern crate yaml_rust;

fn main() {
    let _ = "yaml must not appear in generated code";
}
