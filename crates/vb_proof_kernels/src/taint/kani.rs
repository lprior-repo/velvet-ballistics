//! Kani bounded model checking harnesses for the taint lattice.
//!
//! PO-KANI-007: Verify lattice laws for all combinations of Taint.

#[cfg(kani)]
mod kani_taint_harnesses {
    use crate::taint::{Taint, has_identity, is_associative, is_commutative, is_idempotent};

    /// PO-KANI-007: Verify lattice laws for all combinations of Taint.
    #[cfg(kani)]
    #[kani::proof]
    fn taint_lattice_laws_kani() {
        let a_raw: u8 = kani::any();
        let b_raw: u8 = kani::any();
        let c_raw: u8 = kani::any();

        let a = match a_raw % 3 {
            0 => Taint::Clean,
            1 => Taint::DerivedFromSecret,
            _ => Taint::Secret,
        };
        let b = match b_raw % 3 {
            0 => Taint::Clean,
            1 => Taint::DerivedFromSecret,
            _ => Taint::Secret,
        };
        let c = match c_raw % 3 {
            0 => Taint::Clean,
            1 => Taint::DerivedFromSecret,
            _ => Taint::Secret,
        };

        assert!(is_commutative(a, b), "join must be commutative");
        assert!(is_associative(a, b, c), "join must be associative");
        assert!(is_idempotent(a), "join must be idempotent");
        assert!(has_identity(a), "Clean must be identity");
    }
}
