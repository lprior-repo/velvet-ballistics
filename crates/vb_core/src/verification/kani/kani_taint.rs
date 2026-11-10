use crate::errors::{CoreError, CoreResult};
use crate::frame::{read_taint_cell, write_taint_cell};
use crate::ids::SlotIdx;
use crate::value::{Taint, join_taint};

fn taint_from_u8(v: u8) -> Taint {
    match v % 5 {
        0 => Taint::Clean,
        1 => Taint::DerivedFromSecret,
        2 => Taint::Secret,
        3 => Taint::Random,
        _ => Taint::TimeDependent,
    }
}

fn taint_discriminant(t: Taint) -> u8 {
    match t {
        Taint::Clean => 0,
        Taint::DerivedFromSecret => 1,
        Taint::Secret => 2,
        Taint::Random => 3,
        Taint::TimeDependent => 4,
    }
}

fn taint_lte(a: Taint, b: Taint) -> bool {
    taint_discriminant(a) <= taint_discriminant(b)
}

#[kani::proof]
fn join_taint_ge_first_arg() {
    let a_raw = kani::any::<u8>();
    let b_raw = kani::any::<u8>();
    let a = taint_from_u8(a_raw);
    let b = taint_from_u8(b_raw);
    let result = join_taint(a, b);
    kani::assert(taint_lte(a, result), "join_taint(a, b) >= a");
}

#[kani::proof]
fn join_taint_ge_second_arg() {
    let a_raw = kani::any::<u8>();
    let b_raw = kani::any::<u8>();
    let a = taint_from_u8(a_raw);
    let b = taint_from_u8(b_raw);
    let result = join_taint(a, b);
    kani::assert(taint_lte(b, result), "join_taint(a, b) >= b");
}

#[kani::proof]
fn join_taint_idempotent() {
    let a_raw = kani::any::<u8>();
    let a = taint_from_u8(a_raw);
    let result = join_taint(a, a);
    kani::assert(result == a, "join_taint(a, a) == a");
}

#[kani::proof]
fn join_taint_commutative() {
    let a_raw = kani::any::<u8>();
    let b_raw = kani::any::<u8>();
    let a = taint_from_u8(a_raw);
    let b = taint_from_u8(b_raw);
    let result_ab = join_taint(a, b);
    let result_ba = join_taint(b, a);
    kani::assert(
        result_ab == result_ba,
        "join_taint(a, b) == join_taint(b, a)",
    );
}

// These claims concern selected-cell access, not frame allocation or destruction.
#[kani::proof]
fn read_taint_no_panic() {
    let slot_count: u16 = kani::any();
    let slot_raw: u16 = kani::any();
    let slot = SlotIdx::new(slot_raw);
    if slot_count > 0 && slot_raw < slot_count {
        let result = read_taint_cell(slot, Some(true), Some(Taint::Clean));
        assert!(matches!(&result, Ok(Taint::Clean)));
        std::mem::forget(result);
        kani::cover(
            slot_count == u16::MAX && slot_raw == u16::MAX.saturating_sub(1),
            "original maximum count and last slot read",
        );
        kani::cover(
            slot_count == 1 && slot_raw == 0,
            "original single slot read",
        );
    }

    let initialized: Option<bool> = kani::any();
    let marker = kani::any::<bool>().then(|| taint_from_u8(kani::any()));
    let result = read_taint_cell(slot, initialized, marker);
    assert_read_result(result, slot, initialized, marker);
    kani::cover(
        initialized.is_none() && marker.is_some(),
        "missing slot precedes marker",
    );
    kani::cover(
        initialized == Some(false) && marker.is_none(),
        "uninitialized precedes missing marker",
    );
    kani::cover(
        initialized == Some(true) && marker.is_none(),
        "initialized missing marker read",
    );
    kani::cover(slot_raw == u16::MAX, "full-width read error slot");
    cover_read_taints(initialized, marker);
}

fn cover_read_taints(initialized: Option<bool>, marker: Option<Taint>) {
    kani::cover(
        initialized == Some(true) && marker == Some(Taint::Clean),
        "read clean",
    );
    kani::cover(
        initialized == Some(true) && marker == Some(Taint::DerivedFromSecret),
        "read derived",
    );
    kani::cover(
        initialized == Some(true) && marker == Some(Taint::Secret),
        "read secret",
    );
    kani::cover(
        initialized == Some(true) && marker == Some(Taint::Random),
        "read random",
    );
    kani::cover(
        initialized == Some(true) && marker == Some(Taint::TimeDependent),
        "read time",
    );
}

#[kani::proof]
fn write_taint_no_panic() {
    let slot_count: u16 = kani::any();
    let slot_raw: u16 = kani::any();
    let slot = SlotIdx::new(slot_raw);
    let requested = taint_from_u8(kani::any());
    if slot_count > 0 && slot_raw < slot_count {
        let mut marker = Taint::Clean;
        let result = write_taint_cell(slot, Some(true), Some(&mut marker), requested);
        assert!(matches!(&result, Ok(())));
        assert_eq!(marker, requested);
        std::mem::forget(result);
        kani::cover(
            slot_count == u16::MAX && slot_raw == u16::MAX.saturating_sub(1),
            "original maximum count and last slot write",
        );
        kani::cover(
            slot_count == 1 && slot_raw == 0,
            "original single slot write",
        );
    }

    let initialized: Option<bool> = kani::any();
    let present: bool = kani::any();
    let before = taint_from_u8(kani::any());
    let mut marker = before;
    let target = if present { Some(&mut marker) } else { None };
    let result = write_taint_cell(slot, initialized, target, requested);
    let succeeded = matches!(&result, Ok(()));
    assert_write_result(result, slot, initialized, present);
    assert_eq!(marker, if succeeded { requested } else { before });
    kani::cover(
        initialized.is_none() && present && before != requested,
        "missing slot write preserves marker",
    );
    kani::cover(
        initialized == Some(false) && !present,
        "uninitialized write precedes missing marker",
    );
    kani::cover(
        initialized == Some(false) && present && before != requested,
        "uninitialized write preserves marker",
    );
    kani::cover(
        initialized == Some(true) && !present,
        "initialized missing marker write",
    );
    kani::cover(slot_raw == u16::MAX, "full-width write error slot");
    cover_written_taints(initialized == Some(true) && present, requested);
}

fn cover_written_taints(success: bool, requested: Taint) {
    kani::cover(success && requested == Taint::Clean, "write clean");
    kani::cover(
        success && requested == Taint::DerivedFromSecret,
        "write derived",
    );
    kani::cover(success && requested == Taint::Secret, "write secret");
    kani::cover(success && requested == Taint::Random, "write random");
    kani::cover(success && requested == Taint::TimeDependent, "write time");
}

// CoreError also has owned variants; their destruction is outside this claim.
fn assert_read_result(
    result: CoreResult<Taint>,
    slot: SlotIdx,
    initialized: Option<bool>,
    marker: Option<Taint>,
) {
    match &result {
        Ok(actual) => {
            assert_eq!(initialized, Some(true));
            assert_eq!(marker, Some(*actual));
        }
        Err(CoreError::SlotOutOfBounds { slot: actual }) => {
            assert_eq!(*actual, slot);
            assert!(initialized.is_none() || (initialized == Some(true) && marker.is_none()));
        }
        Err(CoreError::SlotUninitialized { slot: actual }) => {
            assert_eq!(*actual, slot);
            assert_eq!(initialized, Some(false));
        }
        Err(_) => kani::assert(false, "read must return the exact slot error variant"),
    }
    std::mem::forget(result);
}

fn assert_write_result(
    result: CoreResult<()>,
    slot: SlotIdx,
    initialized: Option<bool>,
    present: bool,
) {
    match &result {
        Ok(()) => {
            assert_eq!(initialized, Some(true));
            assert!(present);
        }
        Err(CoreError::SlotOutOfBounds { slot: actual }) => {
            assert_eq!(*actual, slot);
            assert!(initialized.is_none() || (initialized == Some(true) && !present));
        }
        Err(CoreError::SlotUninitialized { slot: actual }) => {
            assert_eq!(*actual, slot);
            assert_eq!(initialized, Some(false));
        }
        Err(_) => kani::assert(false, "write must return the exact slot error variant"),
    }
    std::mem::forget(result);
}
