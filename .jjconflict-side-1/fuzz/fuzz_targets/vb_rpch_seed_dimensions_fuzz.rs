#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use vb_core::RunId;
use vb_storage::recovery::RecoveryError;
use vb_storage::recovery::replay::summary::recovery_dimension_count_from_index;

fuzz_target!(|data: &[u8]| {
    let run = RunId::new(1);
    let max_index = max_index_from_data(data);
    let result = recovery_dimension_count_from_index(max_index, run);
    match (max_index, result) {
        (None, Ok(0)) => {}
        (Some(u16::MAX), Err(RecoveryError::FrameDimensionOverflow { run: error_run })) => {
            assert_eq!(error_run, run);
        }
        (Some(index), Ok(count)) => {
            assert_eq!(count, index.saturating_add(1));
        }
        (case, outcome) => {
            let expected_case = matches!(
                (&case, &outcome),
                (None, Ok(0))
                    | (
                        Some(u16::MAX),
                        Err(RecoveryError::FrameDimensionOverflow { .. })
                    )
                    | (Some(_), Ok(_))
            );
            assert!(
                expected_case,
                "dimension count oracle saw unexpected case {case:?} with outcome {outcome:?}"
            );
        }
    }
});

fn max_index_from_data(data: &[u8]) -> Option<u16> {
    match data.first().copied() {
        Some(0) | None => None,
        Some(_) => two_byte_index(data).or_else(|| data.first().copied().map(u16::from)),
    }
}

fn two_byte_index(data: &[u8]) -> Option<u16> {
    let low = data.get(1).copied()?;
    let high = data.get(2).copied()?;
    Some(u16::from_le_bytes([low, high]))
}
