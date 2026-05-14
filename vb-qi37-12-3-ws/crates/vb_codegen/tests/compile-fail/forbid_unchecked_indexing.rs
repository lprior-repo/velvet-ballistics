//! Compile-fail: generated code must not use `[]` indexing.
//!
//! The generated Rust workflow wraps all slot arrays in a `SlotBank` type
//! that only exposes `.get()` and `.get_mut()`. Direct `[]` indexing
//! is not available on this type, so any generated code attempting `[]`
//! would fail to compile.

#![forbid(unsafe_code)]
#![deny(unused_must_use)]

/// A slot bank that provides only checked access via `.get()` / `.get_mut()`.
/// The codegen output uses this pattern to make `[]` indexing unavailable.
pub struct SlotBank<T, const N: usize> {
    slots: [T; N],
}

impl<T: Copy + Default, const N: usize> SlotBank<T, N> {
    pub fn new() -> Self {
        Self { slots: [T::default(); N] }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.slots.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.slots.get_mut(index)
    }
}

fn main() {
    let bank: SlotBank<i32, 4> = SlotBank::new();
    let idx: usize = 2;
    let _val: i32 = bank[idx];
}
