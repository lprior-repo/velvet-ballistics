//! Non-Verus lattice operations: join_taint and join_many.

use super::r#type::Taint;

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
