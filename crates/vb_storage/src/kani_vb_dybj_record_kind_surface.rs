#![cfg(kani)]
#![forbid(unsafe_code)]

//! PO-VB-DYBJ-008: selected RecordKind surface separation.

use crate::RecordKind;

fn selected_record_kind(choice: u8) -> RecordKind {
    if choice & 1_u8 == 0_u8 {
        RecordKind::RunAccepted
    } else {
        RecordKind::RunHeader
    }
}

#[kani::proof]
fn kani_vb_dybj_record_kind_surface_distinction() {
    let choice: u8 = kani::any();
    let kind = selected_record_kind(choice);
    let envelope_id = kind.id();
    let enum_bytes = postcard::to_allocvec(&kind);
    assert!(enum_bytes.is_ok());

    if let Ok(bytes) = enum_bytes {
        if choice & 1_u8 == 0_u8 {
            assert!(bytes.as_slice() == [3_u8]);
            assert!(envelope_id.to_le_bytes() == [10_u8, 0_u8]);
            assert!(bytes.as_slice() != envelope_id.to_le_bytes());
        } else {
            assert!(bytes.as_slice() == [2_u8]);
            assert!(envelope_id.to_le_bytes() == [3_u8, 0_u8]);
            assert!(bytes.as_slice() != envelope_id.to_le_bytes());
        }
    }

    if choice & 1_u8 == 0_u8 {
        assert!(envelope_id == 10_u16);
    } else {
        assert!(envelope_id == 3_u16);
    }
}
