// Local Verus sketch for vb_queue_semantics queue arithmetic.
//
// STATUS: NON-PROOF / NOT PRODUCTION-BOUND.
//
// This file is intentionally retained only as a local mathematical sketch for
// future proof planning. It does not import `crates/vb_queue_semantics/src/lib.rs`,
// does not use `extern_spec!`, and does not prove any production Rust function.
// Do not cite this file as `deductively_verified` evidence for production queue
// behavior. Production-bound queue proofs require reviewed contracts against the
// real Rust functions and production types.
//
// Actual production source path: crates/vb_queue_semantics/src/lib.rs
// Actual local Verus path: crates/vb_queue_semantics/verification/verus/
//
// The prior unregistered proof comments and tautological lemmas were retired by
// vb-dzibx because they were not central registry obligations and the proofs
// were not bound to production code.
use vstd::prelude::*;

verus! {

/// Local host-width model for the current verifier/build target used by this
/// repository (`linux_x86_64`). This is a sketch constant, not a portability
/// proof for all Rust `usize` targets.
pub open spec fn local_usize_max() -> int {
    18446744073709551615
}

/// Local admission predicate corresponding to the intended mathematical shape
/// of `helper_enqueue_accepts`. This is not a production contract.
pub open spec fn local_enqueue_admits(capacity: int, len: int) -> bool {
    0 < capacity && len < capacity
}

/// Local saturating subtraction model. This is not linked to
/// `remaining_capacity` or `usize::saturating_sub`.
pub open spec fn local_remaining_capacity(capacity: int, len: int) -> int {
    if len <= capacity {
        capacity - len
    } else {
        0
    }
}

/// Local model of the production warning-threshold branch shape, including the
/// `checked_mul(8) == None => capacity` overflow branch that the retired model
/// omitted. This remains a local sketch until bound to production code.
pub open spec fn local_warning_threshold(capacity: int) -> int {
    if capacity < 0 {
        1
    } else if local_usize_max() < capacity * 8 {
        capacity
    } else {
        let threshold = (capacity * 8) / 10;
        if threshold == 0 {
            1
        } else {
            threshold
        }
    }
}

/// Local warning-payload presence predicate. It deliberately avoids defining a
/// mirror `WarningPayload` type because the prior mirror type was not the
/// production struct.
pub open spec fn local_warning_payload_present(capacity: int, depth: int) -> bool {
    depth >= local_warning_threshold(capacity) && depth <= capacity
}

/// Local full-queue predicate. This is not linked to `queue_is_full`.
pub open spec fn local_queue_is_full(capacity: int, len: int) -> bool {
    len >= capacity
}

/// Local capacity-validity predicate. This is not linked to the production
/// `Result<(), CapacityRejection>` or its named rejection variants.
pub open spec fn local_validate_capacity_ok(capacity: int, maximum: int) -> bool {
    0 < capacity && capacity <= maximum
}

/// Local pop-decision predicates. They deliberately avoid mirror `PopDecision`
/// and `EnqueueDecision` types because the retired bridge used non-production
/// struct encodings for production enums.
pub open spec fn local_command_pop_front(capacity: int, len: int) -> bool {
    len > 0 && capacity > 0
}

pub open spec fn local_shard_tick_pop_front(capacity: int, len: int) -> bool {
    local_command_pop_front(capacity, len)
}

pub open spec fn local_runtime_queue_full_maps(depth: int, capacity: int) -> bool {
    depth >= capacity
}

pub open spec fn local_valid_queue_state(capacity: int, len: int) -> bool {
    0 < capacity && 0 <= len && len <= capacity
}

} // verus!
fn main() {}
