// PO-VB-DYBJ-001
// Verus artifact for RunId constructor/accessor/ZERO invariants.
// Production binding: mirrors `vb_core::ids::RunId` at
// `crates/vb_core/src/ids/mod.rs:65,229-231` where RunId is `#[repr(transparent)]`
// over `u64`, `new(value)` stores `value`, `get()` returns the field, and
// `ZERO` is `Self(0)`.

use vstd::prelude::*;

verus! {

pub struct RunIdModel {
    pub value: u64,
}

impl RunIdModel {
    pub open spec fn view(self) -> int {
        self.value as int
    }
}

pub open spec fn run_id_zero() -> RunIdModel {
    RunIdModel { value: 0 }
}

pub open spec fn run_id_new(value: u64) -> RunIdModel {
    RunIdModel { value }
}

pub open spec fn run_id_get(run_id: RunIdModel) -> u64 {
    run_id.value
}

pub proof fn proof_run_id_new_get_parametric(value: u64)
    ensures
        run_id_get(run_id_new(value)) == value,
        run_id_new(value).view() == value as int,
{
}

pub proof fn proof_run_id_zero_matches_new_zero()
    ensures
        run_id_get(run_id_zero()) == 0,
        run_id_zero().view() == run_id_new(0).view(),
{
}

pub proof fn proof_run_id_max_preserved()
    ensures
        run_id_get(run_id_new(u64::MAX)) == u64::MAX,
        run_id_new(u64::MAX).view() == u64::MAX as int,
{
}

} // verus!

fn main() {}
