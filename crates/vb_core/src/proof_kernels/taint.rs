//! Taint lattice proof kernel.
//!
//! This is a tiny, pure, sequential Rust kernel for taint lattice verification.
//! Suitable for Verus/Aeneas extraction to Lean.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Taint {
    Clean,
    DerivedFromSecret,
    Secret,
}

impl Taint {
    pub fn rank(&self) -> u8 {
        match self {
            Taint::Clean => 0,
            Taint::DerivedFromSecret => 1,
            Taint::Secret => 2,
        }
    }
}

pub fn join_taint(a: Taint, b: Taint) -> Taint {
    if a.rank() >= b.rank() { a } else { b }
}

pub fn join_many(taints: &[Taint]) -> Taint {
    let mut result = Taint::Clean;
    for &t in taints {
        result = join_taint(result, t);
    }
    result
}

pub fn is_commutative(a: Taint, b: Taint) -> bool {
    join_taint(a, b) == join_taint(b, a)
}

pub fn is_associative(a: Taint, b: Taint, c: Taint) -> bool {
    join_taint(join_taint(a, b), c) == join_taint(a, join_taint(b, c))
}

pub fn is_idempotent(a: Taint) -> bool {
    join_taint(a, a) == a
}

pub fn has_identity(a: Taint) -> bool {
    join_taint(a, Taint::Clean) == a
}

pub fn secret_never_downgrades() -> bool {
    join_taint(Taint::Clean, Taint::Secret) == Taint::Secret
}

pub fn derived_never_downgrades() -> bool {
    join_taint(Taint::Clean, Taint::DerivedFromSecret) == Taint::DerivedFromSecret
}

pub fn all_lattice_laws(a: Taint, b: Taint, c: Taint) -> bool {
    is_commutative(a, b)
        && is_associative(a, b, c)
        && is_idempotent(a)
        && has_identity(a)
        && secret_never_downgrades()
        && derived_never_downgrades()
}

#[cfg(test)]
mod tests;
