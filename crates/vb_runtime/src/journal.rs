#![forbid(unsafe_code)]
//! Runtime-local journal append port.

include!("journal/chunk_001.rs");
include!("journal/chunk_002.rs");
include!("journal/chunk_003.rs");

#[cfg(test)]
mod tests {
    include!("journal/tests/chunk_001.rs");
    include!("journal/tests/chunk_002.rs");
    include!("journal/tests/chunk_003.rs");
    include!("journal/tests/chunk_004.rs");
    include!("journal/tests/proptest_vb_edvbj_all_24_variants.rs");
    include!("journal/tests/proptest_vb_edvbj_resumed_replay.rs");
}

#[cfg(test)]
mod strict_policy_port_tests {
    include!("journal/tests/strict_policy_port.rs");
}

#[cfg(test)]
mod strict_gate_port_tests {
    include!("journal/tests/strict_gate_port.rs");
}
