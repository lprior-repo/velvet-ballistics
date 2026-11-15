//! Lattice laws over the actual five-variant runtime taint and join body.
use vstd::prelude::*;
mod slot {
    pub use vb_core::SlotValue;
}
#[path = "../../crates/vb_core/src/value/taint.rs"]
mod production;

verus! {

pub fn associative(a: production::Taint, b: production::Taint, c: production::Taint) -> (out: (
    production::Taint,
    production::Taint,
))
    ensures
        out.0 == out.1,
{
    let ab = production::join_taint(a, b);
    let bc = production::join_taint(b, c);
    (production::join_taint(ab, c), production::join_taint(a, bc))
}

pub fn commutative(a: production::Taint, b: production::Taint) -> (out: (
    production::Taint,
    production::Taint,
))
    ensures
        out.0 == out.1,
{
    (production::join_taint(a, b), production::join_taint(b, a))
}

pub fn idempotent(a: production::Taint) -> (out: production::Taint)
    ensures
        out == a,
{
    production::join_taint(a, a)
}

pub fn clean_secret_preserves_secret() -> (out: production::Taint)
    ensures
        out == production::Taint::Secret,
{
    production::join_taint(production::Taint::Clean, production::Taint::Secret)
}

pub fn clean_derived_preserves_derived() -> (out: production::Taint)
    ensures
        out == production::Taint::DerivedFromSecret,
{
    production::join_taint(production::Taint::Clean, production::Taint::DerivedFromSecret)
}

} // verus!
