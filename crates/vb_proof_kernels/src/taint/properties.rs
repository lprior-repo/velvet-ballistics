//! Lattice property verification functions.
//!
//! Each function checks a single lattice law and returns a boolean.
//! These are pure, deterministic, and suitable for both unit tests
//! and property-based testing.

use super::lattice::join_taint;
use super::r#type::Taint;

/// Verifies join is commutative: a ⊔ b == b ⊔ a.
pub fn is_commutative(a: Taint, b: Taint) -> bool {
    join_taint(a, b) == join_taint(b, a)
}

/// Verifies join is associative: (a ⊔ b) ⊔ c == a ⊔ (b ⊔ c).
pub fn is_associative(a: Taint, b: Taint, c: Taint) -> bool {
    join_taint(join_taint(a, b), c) == join_taint(a, join_taint(b, c))
}

/// Verifies join is idempotent: a ⊔ a == a.
pub fn is_idempotent(a: Taint) -> bool {
    join_taint(a, a) == a
}

/// Verifies Clean is the identity element: a ⊔ Clean == a.
pub fn has_identity(a: Taint) -> bool {
    join_taint(a, Taint::Clean) == a
}

/// Verifies Secret never downgrades when joined with Clean.
pub fn secret_never_downgrades() -> bool {
    join_taint(Taint::Clean, Taint::Secret) == Taint::Secret
}

/// Verifies DerivedFromSecret never downgrades when joined with Clean.
pub fn derived_never_downgrades() -> bool {
    join_taint(Taint::Clean, Taint::DerivedFromSecret) == Taint::DerivedFromSecret
}

/// Composite checker: all six lattice laws hold for the given inputs.
pub fn all_lattice_laws(a: Taint, b: Taint, c: Taint) -> bool {
    is_commutative(a, b)
        && is_associative(a, b, c)
        && is_idempotent(a)
        && has_identity(a)
        && secret_never_downgrades()
        && derived_never_downgrades()
}


