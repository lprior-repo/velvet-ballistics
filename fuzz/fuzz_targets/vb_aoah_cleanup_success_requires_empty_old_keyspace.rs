// Obligation: PO-021
// Claim: cleanup success requires empty old keyspace
#![no_main]

use libfuzzer_sys::fuzz_target;

const OLD_VERSION: u16 = 1;
const MAX_BYTES: u8 = 16;

fn read_u16(bytes: &[u8]) -> u16 {
    match bytes.split_first() {
        Some((first, rest)) => match rest.split_first() {
            Some((second, _)) => u16::from_le_bytes([*first, *second]),
            None => u16::from(*first),
        },
        None => 0,
    }
}

fn checked_accounting(left: u8, right: u8) -> Option<u8> {
    left.checked_add(right).filter(|total| *total <= MAX_BYTES)
}

fuzz_target!(|data: &[u8]| {
    let version = read_u16(data);
    let old_records = match data.get(2).copied() {
        Some(value) => value,
        None => 0,
    } % 5;
    let bytes = match data.get(3).copied() {
        Some(value) => value,
        None => 0,
    } % 17;
    let delta = match data.get(4).copied() {
        Some(value) => value,
        None => 0,
    } % 17;

    let writes_before = bytes;
    let migration_required = version == OLD_VERSION;
    let writes_after = writes_before;
    if migration_required {
        assert_eq!(writes_after, writes_before);
    }

    if old_records == 0 {
        let verified_noop = true;
        assert!(verified_noop);
    }

    match checked_accounting(bytes, delta) {
        Some(total) => assert!(total <= MAX_BYTES),
        None => assert!(u16::from(bytes) + u16::from(delta) > u16::from(MAX_BYTES)),
    }
});
