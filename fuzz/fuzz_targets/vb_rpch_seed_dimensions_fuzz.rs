#![no_main]
#![forbid(unsafe_code)]
#![allow(clippy::panic)]

use libfuzzer_sys::fuzz_target;
use vb_core::RunId;
use vb_storage::recovery::replay::summary::recovery_dimension_count_from_index;

fuzz_target!(|data: &[u8]| {
    let max_index = match data.first().copied() {
        Some(0) | None => None,
        Some(_) if data.len() >= 3 => Some(u16::from_le_bytes([data[1], data[2]])),
        Some(value) => Some(u16::from(value)),
    };
    let result = recovery_dimension_count_from_index(max_index, RunId::new(1));
    match (max_index, result) {
        (None, Ok(0)) => {}
        (Some(u16::MAX), Err(_)) => {}
        (Some(index), Ok(count)) => assert_eq!(count, index + 1),
        _ => panic!("invalid seed dimension outcome"),
    }
});
