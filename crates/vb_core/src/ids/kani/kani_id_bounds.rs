//! Kani harnesses proving ID newtype `as_usize` never panics.
//!
//! All `checked_index!` types wrap `u16` and use `usize::from(self.0)`.
//! Since `usize` is at least 16 bits on all platforms, this conversion is
//! infallible. `FanoutLimit::as_usize` uses `try_from` with a saturating
//! fallback to `usize::MAX`, so it also never panics.

#![forbid(unsafe_code)]

use super::*;

#[kani::proof]
fn step_idx_as_usize_never_panics() {
    let id = StepIdx::new(kani::any());
    let _ = id.as_usize();
}

#[kani::proof]
fn slot_idx_as_usize_never_panics() {
    let id = SlotIdx::new(kani::any());
    let _ = id.as_usize();
}

#[kani::proof]
fn expr_idx_as_usize_never_panics() {
    let id = ExprIdx::new(kani::any());
    let _ = id.as_usize();
}

#[kani::proof]
fn accessor_idx_as_usize_never_panics() {
    let id = AccessorIdx::new(kani::any());
    let _ = id.as_usize();
}

#[kani::proof]
fn const_idx_as_usize_never_panics() {
    let id = ConstIdx::new(kani::any());
    let _ = id.as_usize();
}

#[kani::proof]
fn fanout_limit_as_usize_never_panics() {
    let limit = FanoutLimit::new(kani::any());
    let _ = limit.as_usize();
}

#[kani::proof]
fn branch_idx_get_never_panics() {
    let idx = BranchIdx::new(kani::any());
    let _: u16 = idx.get();
}

#[kani::proof]
fn step_idx_as_usize_returns_usize_from_u16() {
    let inner = kani::any::<u16>();
    let id = StepIdx::new(inner);
    let result = id.as_usize();
    kani::assert(result == usize::from(inner),
        "as_usize must equal usize::from(inner)",
    );
}

#[kani::proof]
fn slot_idx_as_usize_returns_usize_from_u16() {
    let inner = kani::any::<u16>();
    let id = SlotIdx::new(inner);
    let result = id.as_usize();
    kani::assert(result == usize::from(inner),
        "as_usize must equal usize::from(inner)",
    );
}

#[kani::proof]
fn expr_idx_as_usize_returns_usize_from_u16() {
    let inner = kani::any::<u16>();
    let id = ExprIdx::new(inner);
    let result = id.as_usize();
    kani::assert(result == usize::from(inner),
        "as_usize must equal usize::from(inner)",
    );
}

#[kani::proof]
fn accessor_idx_as_usize_returns_usize_from_u16() {
    let inner = kani::any::<u16>();
    let id = AccessorIdx::new(inner);
    let result = id.as_usize();
    kani::assert(result == usize::from(inner),
        "as_usize must equal usize::from(inner)",
    );
}

#[kani::proof]
fn const_idx_as_usize_returns_usize_from_u16() {
    let inner = kani::any::<u16>();
    let id = ConstIdx::new(inner);
    let result = id.as_usize();
    kani::assert(result == usize::from(inner),
        "as_usize must equal usize::from(inner)",
    );
}
