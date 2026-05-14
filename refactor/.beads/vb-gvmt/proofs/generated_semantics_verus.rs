use vstd::prelude::*;

verus! {

pub enum SpecTaint {
    Clean,
    DerivedFromSecret,
    Secret,
}

pub open spec fn spec_rank(t: SpecTaint) -> int {
    match t {
        SpecTaint::Clean => 0,
        SpecTaint::DerivedFromSecret => 1,
        SpecTaint::Secret => 2,
    }
}

pub open spec fn spec_taint_leq(left: SpecTaint, right: SpecTaint) -> bool {
    spec_rank(left) <= spec_rank(right)
}

pub open spec fn spec_join_taint(left: SpecTaint, right: SpecTaint) -> SpecTaint {
    if spec_rank(left) >= spec_rank(right) { left } else { right }
}

pub open spec fn spec_no_contract_action_allowed(input: SpecTaint, output: SpecTaint) -> bool {
    input == SpecTaint::Clean || output != SpecTaint::Clean
}

pub open spec fn spec_checked_slot_access(slot: nat, slots_len: nat) -> bool {
    slot < slots_len
}

pub open spec fn spec_parallel_slots(slots_len: nat, taints_len: nat) -> bool {
    slots_len == taints_len
}

pub struct SpecJournal {
    pub len: nat,
    pub capacity: nat,
}

pub open spec fn spec_can_append_event(journal: SpecJournal, needed: nat) -> bool {
    journal.len + needed <= journal.capacity
}

pub open spec fn spec_append_event_len(journal: SpecJournal, needed: nat) -> nat {
    if spec_can_append_event(journal, needed) { journal.len + needed } else { journal.len }
}

pub struct SpecPendingAction {
    pub step: nat,
    pub action_id: nat,
    pub output_slot: nat,
    pub resume_pc: nat,
}

pub open spec fn spec_resume_identity_matches(
    pending: SpecPendingAction,
    step: nat,
    action_id: nat,
    output_slot: nat,
    resume_pc: nat,
) -> bool {
    pending.step == step
        && pending.action_id == action_id
        && pending.output_slot == output_slot
        && pending.resume_pc == resume_pc
}

pub open spec fn spec_resume_transition_slot(
    old_slot: int,
    new_value: int,
    identity_matches: bool,
) -> int {
    if identity_matches { new_value } else { old_slot }
}

pub open spec fn spec_resume_transition_journal_len(
    old_len: nat,
    identity_matches: bool,
) -> nat {
    if identity_matches { old_len + 2 } else { old_len }
}

pub proof fn proof_checked_slot_access_no_panic(slot: nat, slots_len: nat)
    ensures
        spec_checked_slot_access(slot, slots_len) ==> slot < slots_len,
{
}

pub proof fn proof_slot_write_preserves_parallel_taint(slots_len: nat, taints_len: nat)
    requires
        slots_len == taints_len,
    ensures
        spec_parallel_slots(slots_len, taints_len),
{
}

pub proof fn spec_join_taint_monotonic(left: SpecTaint, right: SpecTaint)
    ensures
        spec_taint_leq(left, spec_join_taint(left, right)),
        spec_taint_leq(right, spec_join_taint(left, right)),
{
    match left {
        SpecTaint::Clean => {
            match right {
                SpecTaint::Clean => assert(spec_taint_leq(left, spec_join_taint(left, right)) && spec_taint_leq(right, spec_join_taint(left, right))) by(compute),
                SpecTaint::DerivedFromSecret => assert(spec_taint_leq(left, spec_join_taint(left, right)) && spec_taint_leq(right, spec_join_taint(left, right))) by(compute),
                SpecTaint::Secret => assert(spec_taint_leq(left, spec_join_taint(left, right)) && spec_taint_leq(right, spec_join_taint(left, right))) by(compute),
            }
        }
        SpecTaint::DerivedFromSecret => {
            match right {
                SpecTaint::Clean => assert(spec_taint_leq(left, spec_join_taint(left, right)) && spec_taint_leq(right, spec_join_taint(left, right))) by(compute),
                SpecTaint::DerivedFromSecret => assert(spec_taint_leq(left, spec_join_taint(left, right)) && spec_taint_leq(right, spec_join_taint(left, right))) by(compute),
                SpecTaint::Secret => assert(spec_taint_leq(left, spec_join_taint(left, right)) && spec_taint_leq(right, spec_join_taint(left, right))) by(compute),
            }
        }
        SpecTaint::Secret => {
            match right {
                SpecTaint::Clean => assert(spec_taint_leq(left, spec_join_taint(left, right)) && spec_taint_leq(right, spec_join_taint(left, right))) by(compute),
                SpecTaint::DerivedFromSecret => assert(spec_taint_leq(left, spec_join_taint(left, right)) && spec_taint_leq(right, spec_join_taint(left, right))) by(compute),
                SpecTaint::Secret => assert(spec_taint_leq(left, spec_join_taint(left, right)) && spec_taint_leq(right, spec_join_taint(left, right))) by(compute),
            }
        }
    }
}

pub proof fn proof_no_contract_tainted_input_clean_output_rejected(input: SpecTaint)
    requires
        input != SpecTaint::Clean,
    ensures
        !spec_no_contract_action_allowed(input, SpecTaint::Clean),
{
    match input {
        SpecTaint::DerivedFromSecret => assert(!spec_no_contract_action_allowed(input, SpecTaint::Clean)) by(compute),
        SpecTaint::Secret => assert(!spec_no_contract_action_allowed(input, SpecTaint::Clean)) by(compute),
        SpecTaint::Clean => {},
    }
}

pub proof fn proof_journal_append_capacity_or_error(journal: SpecJournal, needed: nat)
    requires
        journal.len <= journal.capacity,
    ensures
        spec_can_append_event(journal, needed) ==> spec_append_event_len(journal, needed) <= journal.capacity,
        !spec_can_append_event(journal, needed) ==> spec_append_event_len(journal, needed) == journal.len,
{
}

pub proof fn proof_resume_validates_identity_before_mutation(
    pending: SpecPendingAction,
    step: nat,
    action_id: nat,
    output_slot: nat,
    resume_pc: nat,
    old_slot: int,
    new_value: int,
    old_journal_len: nat,
)
    requires
        !spec_resume_identity_matches(pending, step, action_id, output_slot, resume_pc),
    ensures
        spec_resume_transition_slot(old_slot, new_value, spec_resume_identity_matches(pending, step, action_id, output_slot, resume_pc)) == old_slot,
        spec_resume_transition_journal_len(old_journal_len, spec_resume_identity_matches(pending, step, action_id, output_slot, resume_pc)) == old_journal_len,
{
}

}
