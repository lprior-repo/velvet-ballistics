use vstd::prelude::*;

verus! {
pub enum Capability { ReadOnly, Writer }
pub enum Operation { ScanBounded, GetExact, Append, Persist, Delete, Compact, Migrate, CreateSyntheticRun }

pub open spec fn allowed(cap: Capability, op: Operation) -> bool {
    match cap {
        Capability::ReadOnly => op is ScanBounded || op is GetExact,
        Capability::Writer => true,
    }
}

pub proof fn lemma_readonly_forbids_mutation(op: Operation)
    requires allowed(Capability::ReadOnly, op)
    ensures op is ScanBounded || op is GetExact
{}
}
