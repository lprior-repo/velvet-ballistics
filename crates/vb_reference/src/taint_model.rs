//! Reference taint lattice model.
//!
//! This is the canonical reference implementation for the taint lattice.
//! Use this to verify the optimized implementation matches this behavior.
//!
//! Lattice ordering: Clean < DerivedFromSecret < Secret

use vb_core::value::Taint;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaintOrder {
    Clean,
    DerivedFromSecret,
    Secret,
}

impl TaintOrder {
    pub fn from_taint(t: Taint) -> Self {
        match t {
            Taint::Clean => TaintOrder::Clean,
            Taint::DerivedFromSecret => TaintOrder::DerivedFromSecret,
            Taint::Secret => TaintOrder::Secret,
        }
    }

    pub fn to_taint(self) -> Taint {
        match self {
            TaintOrder::Clean => Taint::Clean,
            TaintOrder::DerivedFromSecret => Taint::DerivedFromSecret,
            TaintOrder::Secret => Taint::Secret,
        }
    }

    pub fn rank(&self) -> u8 {
        match self {
            TaintOrder::Clean => 0,
            TaintOrder::DerivedFromSecret => 1,
            TaintOrder::Secret => 2,
        }
    }
}

pub fn join_ref(a: TaintOrder, b: TaintOrder) -> TaintOrder {
    let rank_a = a.rank();
    let rank_b = b.rank();
    if rank_a >= rank_b {
        a
    } else {
        b
    }
}

pub fn join_many_ref(taints: &[TaintOrder]) -> TaintOrder {
    let mut result = TaintOrder::Clean;
    for &t in taints {
        result = join_ref(result, t);
    }
    result
}

pub fn lattice_laws() -> Vec<LatticeLaw> {
    vec![
        LatticeLaw {
            name: "commutative".to_string(),
            check: |a: TaintOrder, b: TaintOrder| join_ref(a, b) == join_ref(b, a),
        },
        LatticeLaw {
            name: "associative".to_string(),
            check: |a: TaintOrder, b: TaintOrder, c: TaintOrder| {
                join_ref(join_ref(a, b), c) == join_ref(a, join_ref(b, c))
            },
        },
        LatticeLaw {
            name: "idempotent".to_string(),
            check: |a: TaintOrder, _b: TaintOrder| join_ref(a, a) == a,
        },
        LatticeLaw {
            name: "identity".to_string(),
            check: |a: TaintOrder, _b: TaintOrder| join_ref(a, TaintOrder::Clean) == a,
        },
        LatticeLaw {
            name: "secret_never_downgrades".to_string(),
            check: |_a: TaintOrder, b: TaintOrder| {
                if b == TaintOrder::Secret {
                    join_ref(TaintOrder::Clean, b) == TaintOrder::Secret
                } else {
                    true
                }
            },
        },
        LatticeLaw {
            name: "derived_never_downgrades".to_string(),
            check: |_a: TaintOrder, b: TaintOrder| {
                if b == TaintOrder::DerivedFromSecret {
                    join_ref(TaintOrder::Clean, b) == TaintOrder::DerivedFromSecret
                } else {
                    true
                }
            },
        },
    ]
}

pub struct LatticeLaw {
    pub name: String,
    pub check: fn(TaintOrder, TaintOrder) -> bool,
}

impl LatticeLaw {
    pub fn check_two(&self, a: TaintOrder, b: TaintOrder) -> bool {
        (self.check)(a, b)
    }

    pub fn check_three(&self, a: TaintOrder, b: TaintOrder, c: TaintOrder) -> bool
    where
        Self: Sized,
    {
        let f = self.check;
        f(a, b) && f(b, c) && f(a, c)
    }
}

pub fn all_join_results() -> Vec<(TaintOrder, TaintOrder, TaintOrder)> {
    let values = [
        TaintOrder::Clean,
        TaintOrder::DerivedFromSecret,
        TaintOrder::Secret,
    ];
    let mut results = Vec::new();
    for &a in &values {
        for &b in &values {
            let j = join_ref(a, b);
            results.push((a, b, j));
        }
    }
    results
}

pub struct TaintModel;

impl TaintModel {
    pub fn new() -> Self {
        TaintModel
    }

    pub fn join(&self, a: Taint, b: Taint) -> Taint {
        join_ref(TaintOrder::from_taint(a), TaintOrder::from_taint(b)).to_taint()
    }

    pub fn join_all(&self, taints: &[Taint]) -> Taint {
        let orders: Vec<TaintOrder> = taints.iter().map(|&t| TaintOrder::from_taint(t)).collect();
        join_many_ref(&orders).to_taint()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_commutative() {
        let law = lattice_laws().into_iter().find(|l| l.name == "commutative").unwrap();
        for &a in &[TaintOrder::Clean, TaintOrder::DerivedFromSecret, TaintOrder::Secret] {
            for &b in &[TaintOrder::Clean, TaintOrder::DerivedFromSecret, TaintOrder::Secret] {
                assert!(law.check_two(a, b), "join({}, {}) != join({}, {})", a.rank(), b.rank(), b.rank(), a.rank());
            }
        }
    }

    #[test]
    fn test_join_associative() {
        let law = lattice_laws().into_iter().find(|l| l.name == "associative").unwrap();
        for &a in &[TaintOrder::Clean, TaintOrder::DerivedFromSecret, TaintOrder::Secret] {
            for &b in &[TaintOrder::Clean, TaintOrder::DerivedFromSecret, TaintOrder::Secret] {
                for &c in &[TaintOrder::Clean, TaintOrder::DerivedFromSecret, TaintOrder::Secret] {
                    assert!(law.check(a, b, c), "associativity failed");
                }
            }
        }
    }

    #[test]
    fn test_join_idempotent() {
        let law = lattice_laws().into_iter().find(|l| l.name == "idempotent").unwrap();
        for &a in &[TaintOrder::Clean, TaintOrder::DerivedFromSecret, TaintOrder::Secret] {
            assert!(law.check_two(a, a), "idempotence failed for {:?}", a);
        }
    }

    #[test]
    fn test_join_identity() {
        let law = lattice_laws().into_iter().find(|l| l.name == "identity").unwrap();
        for &a in &[TaintOrder::Clean, TaintOrder::DerivedFromSecret, TaintOrder::Secret] {
            assert!(law.check_two(a, TaintOrder::Clean), "identity failed for {:?}", a);
        }
    }

    #[test]
    fn test_all_join_results_table() {
        let results = all_join_results();
        assert_eq!(results.len(), 9);
    }
}
