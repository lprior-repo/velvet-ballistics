#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use vb_storage::recovery::replay::core::{replay_attempt_is_current, replay_attempt_is_stale};

fuzz_target!(|data: &[u8]| {
    let attempt = data.first().copied().map(u16::from);
    let max_attempt = data.get(1).copied().map_or(1, u16::from);
    let observed = attempt.unwrap_or(1);
    assert_eq!(replay_attempt_is_current(attempt, max_attempt), observed >= max_attempt);
    assert_eq!(replay_attempt_is_stale(attempt, max_attempt), observed < max_attempt);
});
