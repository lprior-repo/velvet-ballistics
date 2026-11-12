use super::{MAX_EXPRESSION_STACK_USIZE, OperandTaints, Taint};

impl kani::Arbitrary for OperandTaints {
    fn any() -> Self {
        Self {
            entries: kani::any(),
            pending: kani::any(),
        }
    }
}

#[kani::proof]
#[kani::unwind(66)]
fn kani_operand_taints_put_peek_preserve_other_entries_and_pending() {
    let mut state: OperandTaints = kani::any();
    let before = state.entries;
    let pending = state.pending;
    let index: usize = kani::any();
    let taint: Taint = kani::any();
    let expected = before.get(index).copied().flatten();
    kani::assert(
        state.peek(index) == expected,
        "peek returns the exact original entry",
    );
    kani::assert(state.entries == before, "peek preserves every entry");
    kani::assert(
        state.pending() == pending,
        "peek preserves pending provenance",
    );
    let result = state.put(index, taint);
    kani::assert(
        result == (index < MAX_EXPRESSION_STACK_USIZE).then_some(()),
        "put reports the exact index validity",
    );
    for (position, previous) in before.into_iter().enumerate() {
        let expected = if position == index {
            Some(taint)
        } else {
            previous
        };
        kani::assert(
            state.entries.get(position).copied() == Some(expected),
            "put changes only the selected entry",
        );
        kani::assert(
            state.peek(position) == expected,
            "peek observes each exact post-put entry",
        );
    }
    kani::assert(
        state.peek(index)
            == if index < MAX_EXPRESSION_STACK_USIZE {
                Some(taint)
            } else {
                None
            },
        "peek handles the same full index domain after put",
    );
    kani::assert(
        state.pending() == pending,
        "put preserves pending provenance",
    );
    kani::cover!(index == 0, "put first entry");
    kani::cover!(index == 63, "put last entry");
    kani::cover!(index == 64, "put rejects first invalid index");
    kani::cover!(index == usize::MAX, "put rejects maximal index");
    kani::cover!(index < 64 && taint == Taint::Clean, "put Clean");
    kani::cover!(
        index < 64 && taint == Taint::DerivedFromSecret,
        "put DerivedFromSecret"
    );
    kani::cover!(index < 64 && taint == Taint::Secret, "put Secret");
    kani::cover!(index < 64 && taint == Taint::Random, "put Random");
    kani::cover!(
        index < 64 && taint == Taint::TimeDependent,
        "put TimeDependent"
    );
    kani::cover!(
        before.first() == Some(&None)
            && before.last() == Some(&Some(Taint::Secret))
            && pending == Taint::TimeDependent,
        "independent mixed entries and pending"
    );
}

#[kani::proof]
#[kani::unwind(66)]
fn kani_operand_taints_take_preserves_other_entries_and_pending() {
    let mut state: OperandTaints = kani::any();
    let before = state.entries;
    let pending = state.pending;
    let index: usize = kani::any();
    let expected = before.get(index).copied().flatten();
    kani::assert(
        state.take(index) == expected,
        "take returns the exact original optional tag",
    );
    for (position, previous) in before.into_iter().enumerate() {
        let expected = if position == index { None } else { previous };
        kani::assert(
            state.entries.get(position).copied() == Some(expected),
            "take clears only the selected entry",
        );
        kani::assert(
            state.peek(position) == expected,
            "peek observes each exact post-take entry",
        );
    }
    kani::assert(
        state.pending() == pending,
        "take preserves pending provenance",
    );
    let after = state.entries;
    kani::assert(
        state.peek(index) == None,
        "taken or invalid entry is absent",
    );
    kani::assert(state.take(index) == None, "repeated take returns None");
    kani::assert(
        state.entries == after,
        "repeated take preserves all post-take entries",
    );
    kani::assert(
        state.pending() == pending,
        "repeated take preserves pending provenance",
    );
    kani::cover!(index == 0, "take first entry");
    kani::cover!(index == 63, "take last entry");
    kani::cover!(index == 64, "take rejects first invalid index");
    kani::cover!(index == usize::MAX, "take rejects maximal index");
    kani::cover!(
        index < 64 && expected.is_none(),
        "take an absent valid entry"
    );
    kani::cover!(expected == Some(Taint::Clean), "take Clean");
    kani::cover!(
        expected == Some(Taint::DerivedFromSecret),
        "take DerivedFromSecret"
    );
    kani::cover!(expected == Some(Taint::Secret), "take Secret");
    kani::cover!(expected == Some(Taint::Random), "take Random");
    kani::cover!(expected == Some(Taint::TimeDependent), "take TimeDependent");
}

#[kani::proof]
#[kani::unwind(66)]
fn kani_operand_taints_reset_preserves_all_entries() {
    let mut state: OperandTaints = kani::any();
    let before = state.entries;
    let pending = state.pending;
    state.reset();
    kani::assert(
        state.pending() == Taint::Clean,
        "reset clears pending provenance",
    );
    kani::assert(
        state.entries == before,
        "reset preserves every optional entry",
    );
    for (position, previous) in before.into_iter().enumerate() {
        kani::assert(
            state.peek(position) == previous,
            "peek preserves each entry after reset",
        );
    }
    state.reset();
    kani::assert(
        state.pending() == Taint::Clean,
        "repeated reset stays Clean",
    );
    kani::assert(
        state.entries == before,
        "repeated reset preserves every entry",
    );
    kani::cover!(pending == Taint::Clean, "reset Clean");
    kani::cover!(
        pending == Taint::DerivedFromSecret,
        "reset DerivedFromSecret"
    );
    kani::cover!(pending == Taint::Secret, "reset Secret");
    kani::cover!(pending == Taint::Random, "reset Random");
    kani::cover!(pending == Taint::TimeDependent, "reset TimeDependent");
    kani::cover!(
        before.iter().all(Option::is_none),
        "reset all absent entries"
    );
    kani::cover!(
        before.iter().all(Option::is_some),
        "reset all present entries"
    );
}
