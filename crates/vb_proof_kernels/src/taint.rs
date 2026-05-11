//! Taint lattice proof kernel.
//!
//! This is a tiny, pure, sequential Rust kernel for taint lattice verification.
//! Suitable for Verus/Aeneas extraction to Lean.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    if a.rank() >= b.rank() {
        a
    } else {
        b
    }
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
mod tests {
    use super::*;

    #[test]
    fn test_join_clean_clean() {
        assert_eq!(join_taint(Taint::Clean, Taint::Clean), Taint::Clean);
    }

    #[test]
    fn test_join_clean_secret() {
        assert_eq!(join_taint(Taint::Clean, Taint::Secret), Taint::Secret);
    }

    #[test]
    fn test_join_secret_clean() {
        assert_eq!(join_taint(Taint::Secret, Taint::Clean), Taint::Secret);
    }

    #[test]
    fn test_join_secret_derived() {
        assert_eq!(join_taint(Taint::Secret, Taint::DerivedFromSecret), Taint::Secret);
    }

    #[test]
    fn test_join_derived_derived() {
        assert_eq!(join_taint(Taint::DerivedFromSecret, Taint::DerivedFromSecret), Taint::DerivedFromSecret);
    }

    #[test]
    fn test_commutative() {
        for &a in &[Taint::Clean, Taint::DerivedFromSecret, Taint::Secret] {
            for &b in &[Taint::Clean, Taint::DerivedFromSecret, Taint::Secret] {
                assert!(is_commutative(a, b), " {:?} ⊔ {:?} != {:?} ⊔ {:?}", a, b, b, a);
            }
        }
    }

    #[test]
    fn test_associative() {
        for &a in &[Taint::Clean, Taint::DerivedFromSecret, Taint::Secret] {
            for &b in &[Taint::Clean, Taint::DerivedFromSecret, Taint::Secret] {
                for &c in &[Taint::Clean, Taint::DerivedFromSecret, Taint::Secret] {
                    assert!(is_associative(a, b, c), "associativity failed");
                }
            }
        }
    }

    #[test]
    fn test_idempotent() {
        for &a in &[Taint::Clean, Taint::DerivedFromSecret, Taint::Secret] {
            assert!(is_idempotent(a), "idempotence failed for {:?}", a);
        }
    }

    #[test]
    fn test_identity() {
        for &a in &[Taint::Clean, Taint::DerivedFromSecret, Taint::Secret] {
            assert!(has_identity(a), "identity failed for {:?}", a);
        }
    }

    #[test]
    fn test_secret_never_downgrades() {
        assert!(secret_never_downgrades());
    }

    #[test]
    fn test_derived_never_downgrades() {
        assert!(derived_never_downgrades());
    }
}
