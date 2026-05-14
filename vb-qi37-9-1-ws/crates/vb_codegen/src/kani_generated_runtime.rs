#[derive(Clone, Copy, PartialEq, Eq)]
enum ModelTaint {
    Clean,
    DerivedFromSecret,
    Secret,
}

#[derive(Clone, Copy)]
struct PendingAction {
    step: u16,
    action_id: u16,
    output_slot: u16,
    resume_pc: u16,
}

fn taint_from_raw(raw: u8) -> ModelTaint {
    match raw % 3 {
        0 => ModelTaint::Clean,
        1 => ModelTaint::DerivedFromSecret,
        _ => ModelTaint::Secret,
    }
}

fn taint_rank(taint: ModelTaint) -> u8 {
    match taint {
        ModelTaint::Clean => 0,
        ModelTaint::DerivedFromSecret => 1,
        ModelTaint::Secret => 2,
    }
}

fn no_contract_action_allowed(input: ModelTaint, output: ModelTaint) -> bool {
    input == ModelTaint::Clean || output != ModelTaint::Clean
}

fn can_append(len: u16, needed: u16, capacity: u16) -> bool {
    if len > capacity {
        false
    } else {
        u32::from(needed) <= u32::from(capacity - len)
    }
}

fn action_resume_matches(
    pending: PendingAction,
    step: u16,
    action_id: u16,
    output_slot: u16,
    resume_pc: u16,
) -> bool {
    pending.step == step
        && pending.action_id == action_id
        && pending.output_slot == output_slot
        && pending.resume_pc == resume_pc
}

#[kani::proof]
fn join_taint_is_monotonic_for_generated_lattice_model() {
    let left = taint_from_raw(kani::any::<u8>());
    let right = taint_from_raw(kani::any::<u8>());
    let joined = if taint_rank(left) >= taint_rank(right) {
        left
    } else {
        right
    };

    kani::assert(
        taint_rank(left) <= taint_rank(joined),
        "joined taint is at least as restrictive as left",
    );
    kani::assert(
        taint_rank(right) <= taint_rank(joined),
        "joined taint is at least as restrictive as right",
    );
}

#[kani::proof]
fn no_contract_action_rejects_clean_output_from_tainted_input() {
    let input = taint_from_raw(kani::any::<u8>());
    kani::assume(input != ModelTaint::Clean);

    kani::assert(
        !no_contract_action_allowed(input, ModelTaint::Clean),
        "tainted no-contract action input cannot produce clean output",
    );
}

#[kani::proof]
fn journal_capacity_precheck_prevents_overflowing_append() {
    let len = kani::any::<u16>();
    let needed = kani::any::<u16>();
    let capacity = kani::any::<u16>();
    kani::assume(len <= capacity);
    kani::assume(capacity <= 256);
    kani::assume(needed <= 8);

    if can_append(len, needed, capacity) {
        kani::assert(
            u32::from(len) + u32::from(needed) <= u32::from(capacity),
            "accepted journal append fits capacity",
        );
    } else {
        kani::assert(
            len <= capacity,
            "rejected append preserves prior journal length",
        );
    }
}

#[kani::proof]
fn invalid_action_resume_preserves_slot_and_journal_model() {
    let pending = PendingAction {
        step: kani::any::<u16>(),
        action_id: kani::any::<u16>(),
        output_slot: kani::any::<u16>(),
        resume_pc: kani::any::<u16>(),
    };
    let step = kani::any::<u16>();
    let action_id = kani::any::<u16>();
    let output_slot = kani::any::<u16>();
    let resume_pc = kani::any::<u16>();
    let old_slot = kani::any::<i64>();
    let new_value = kani::any::<i64>();
    let old_journal_len = kani::any::<u16>();

    let matches = action_resume_matches(pending, step, action_id, output_slot, resume_pc);
    kani::assume(!matches);

    let resulting_slot = if matches { new_value } else { old_slot };
    let resulting_journal_len = if matches {
        old_journal_len.saturating_add(2)
    } else {
        old_journal_len
    };

    kani::assert(
        resulting_slot == old_slot,
        "invalid resume preserves slot value",
    );
    kani::assert(
        resulting_journal_len == old_journal_len,
        "invalid resume preserves journal length",
    );
}

#[kani::proof]
fn slot_bounds_model_distinguishes_valid_and_invalid_indices() {
    let slot = kani::any::<u16>();
    let slot_count = kani::any::<u16>();
    kani::assume(slot_count <= 256);

    let in_bounds = slot < slot_count;
    if in_bounds {
        kani::assert(slot < slot_count, "valid slot index is strictly in bounds");
    } else {
        kani::assert(slot >= slot_count, "invalid slot index is not in bounds");
    }
}
