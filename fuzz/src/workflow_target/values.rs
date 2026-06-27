//! Slot value fuzz target body.

pub fn fuzz_slot_value_roundtrip(data: &[u8]) {
    let Ok(decoded): Result<vb_core::SlotValue, _> = postcard::from_bytes(data) else {
        return;
    };
    let Ok(re_encoded): Result<Vec<u8>, _> = postcard::to_allocvec(&decoded) else {
        return;
    };
    if data.len() == re_encoded.len() {
        let mut matching = true;
        for i in 0..data.len() {
            if data.get(i) != re_encoded.get(i) {
                matching = false;
                break;
            }
        }
        if matching {
            let Ok(_re_decoded): Result<vb_core::SlotValue, _> = postcard::from_bytes(&re_encoded)
            else {
                return;
            };
        }
    }
    let store = vb_core::ValueStore::new();
    let display = decoded.display_with_store(&store);
    assert!(!display.is_empty());
    let type_name = decoded.type_name();
    assert!(!type_name.is_empty());
    let truthy = decoded.is_true();
    assert_eq!(truthy, decoded.is_true());
}
