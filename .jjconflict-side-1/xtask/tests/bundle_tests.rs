// Proof harnesses, property tests, and Miri checks for bundle module.
//
// Kani harnesses: OBL-001 through OBL-004 (run via `cargo kani`)
// Proptest properties: OBL-005 through OBL-007 (run via `cargo test --test bundle_tests`)
// Miri UB check: OBL-008 (run via `cargo +nightly miri test --test bundle_tests`)

use std::path::PathBuf;

use proptest::prelude::*;
use xtask::evidence::*;

include!("bundle_tests/kani_harnesses.rs");
include!("bundle_tests/roundtrip.rs");
include!("bundle_tests/fail_closed.rs");
include!("bundle_tests/path_format.rs");
include!("bundle_tests/miri.rs");
include!("bundle_tests/strategy.rs");
