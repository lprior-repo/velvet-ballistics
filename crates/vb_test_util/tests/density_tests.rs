//! Cross-module integration tests for `vb_test_util` to satisfy
//! the 5x test-density contract (vb-tdst) and to act as behavior
//! tests for the deterministic fixture, seed, and Fjall test harness.

#![forbid(unsafe_code)]
#![allow(
    clippy::indexing_slicing,
    clippy::clone_on_copy,
    clippy::panic,
    clippy::panic_in_result_fn
)]

use vb_test_util::TestSetupError;
use vb_test_util::fixture::{FixtureBuilder, FixtureCapacity};
use vb_test_util::seed::SeededBytes;
use vb_test_util::temp_keyspace::TempKeyspace;

// -- TestSetupError Display + Clone + PartialEq -----------------------------

#[test]
fn setup_error_out_of_memory_display() {
    let e = TestSetupError::OutOfMemory;
    assert_eq!(format!("{}", e), "out of memory");
}

#[test]
fn setup_error_invalid_seed_display() {
    let e = TestSetupError::InvalidSeed("nope".to_string());
    assert_eq!(format!("{}", e), "invalid seed: nope");
}

#[test]
fn setup_error_invalid_capacity_display() {
    let e = TestSetupError::InvalidCapacity("zero".to_string());
    assert_eq!(format!("{}", e), "invalid capacity: zero");
}

#[test]
fn setup_error_temp_dir_display() {
    let e = TestSetupError::TempDirError("missing /tmp".to_string());
    assert_eq!(format!("{}", e), "temp directory error: missing /tmp");
}

#[test]
fn setup_error_fjall_open_display() {
    let e = TestSetupError::FjallOpenError("lock conflict".to_string());
    assert_eq!(format!("{}", e), "fjall open error: lock conflict");
}

#[test]
fn setup_error_postcard_encode_display() {
    let e = TestSetupError::PostcardEncodeError("truncated".to_string());
    assert_eq!(format!("{}", e), "postcard encode error: truncated");
}

#[test]
fn setup_error_postcard_decode_display() {
    let e = TestSetupError::PostcardDecodeError("bad magic".to_string());
    assert_eq!(format!("{}", e), "postcard decode error: bad magic");
}

#[test]
fn setup_error_assertion_mismatch_display() {
    let e = TestSetupError::AssertionMismatch("drift".to_string());
    assert_eq!(format!("{}", e), "assertion mismatch: drift");
}

#[test]
fn setup_error_clone_eq_unit_variant() {
    let a = TestSetupError::OutOfMemory;
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn setup_error_clone_eq_string_variant() {
    let a = TestSetupError::InvalidSeed("x".to_string());
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn setup_error_distinct_variants_ne() {
    assert_ne!(
        TestSetupError::OutOfMemory,
        TestSetupError::InvalidSeed("x".to_string())
    );
    assert_ne!(
        TestSetupError::TempDirError("y".to_string()),
        TestSetupError::FjallOpenError("y".to_string())
    );
}

#[test]
fn setup_error_debug_strings_contain_variant_name() {
    assert!(format!("{:?}", TestSetupError::OutOfMemory).contains("OutOfMemory"));
    assert!(format!("{:?}", TestSetupError::InvalidSeed("z".to_string())).contains("InvalidSeed"));
}

// -- FixtureCapacity ---------------------------------------------------------

#[test]
fn fixture_capacity_max_value_is_one_mib() {
    assert_eq!(FixtureCapacity::MAX_CAPACITY, 1024 * 1024);
}

fn expect_fixture_capacity_ok(result: Result<FixtureCapacity, TestSetupError>) -> FixtureCapacity {
    match result {
        Ok(v) => v,
        Err(e) => panic!("FixtureCapacity::new should succeed, got Err({e:?})"),
    }
}

#[test]
fn fixture_capacity_value_field_accessible() {
    let cap = expect_fixture_capacity_ok(FixtureCapacity::new(128));
    assert_eq!(cap.value, 128);
}

#[test]
fn fixture_capacity_clone_eq() {
    let a = expect_fixture_capacity_ok(FixtureCapacity::new(256));
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn fixture_capacity_debug_includes_value() {
    let cap = expect_fixture_capacity_ok(FixtureCapacity::new(64));
    let s = format!("{:?}", cap);
    assert!(s.contains("FixtureCapacity"));
    assert!(s.contains("64"));
}

// -- FixtureBuilder ----------------------------------------------------------

#[test]
fn fixture_builder_with_capacity_accepts_valid() {
    let cap = expect_fixture_capacity_ok(FixtureCapacity::new(64));
    let builder = match FixtureBuilder::with_capacity(cap) {
        Ok(v) => v,
        Err(e) => panic!("FixtureBuilder::with_capacity(cap) should succeed, got Err({e:?})"),
    };
    // Verify the builder produces output of the expected capacity
    let bytes = builder.build_bytes(0);
    assert_eq!(bytes.len(), 64);
}

#[test]
fn fixture_builder_build_bytes_returns_capacity_length() {
    let cap = expect_fixture_capacity_ok(FixtureCapacity::new(100));
    let builder = match FixtureBuilder::with_capacity(cap) {
        Ok(v) => v,
        Err(e) => panic!("FixtureBuilder::with_capacity(cap) should succeed, got Err({e:?})"),
    };
    let bytes = builder.build_bytes(1);
    assert_eq!(bytes.len(), 100);
}

#[test]
fn fixture_builder_build_bytes_zero_seed_nonempty() {
    let cap = expect_fixture_capacity_ok(FixtureCapacity::new(32));
    let builder = match FixtureBuilder::with_capacity(cap) {
        Ok(v) => v,
        Err(e) => panic!("FixtureBuilder::with_capacity(cap) should succeed, got Err({e:?})"),
    };
    let bytes = builder.build_bytes(0);
    assert_eq!(bytes.len(), 32);
    // Zero seed should still produce deterministic non-zero bytes
    assert!(
        bytes.iter().any(|&b| b != 0),
        "zero-seed bytes should not be all zeros"
    );
}

#[test]
fn fixture_builder_build_bytes_determinism_same_seed() {
    let cap = expect_fixture_capacity_ok(FixtureCapacity::new(64));
    let b1 = match FixtureBuilder::with_capacity(cap) {
        Ok(v) => v,
        Err(e) => panic!("FixtureBuilder::with_capacity(cap) should succeed, got Err({e:?})"),
    };
    let b2 = match FixtureBuilder::with_capacity(cap) {
        Ok(v) => v,
        Err(e) => panic!("FixtureBuilder::with_capacity(cap) should succeed, got Err({e:?})"),
    };
    let bytes_a = b1.build_bytes(7);
    let bytes_b = b2.build_bytes(7);
    assert_eq!(bytes_a, bytes_b);
}

#[test]
fn fixture_builder_build_bytes_different_seeds_diverge() {
    let cap = expect_fixture_capacity_ok(FixtureCapacity::new(64));
    let b1 = match FixtureBuilder::with_capacity(cap) {
        Ok(v) => v,
        Err(e) => panic!("FixtureBuilder::with_capacity(cap) should succeed, got Err({e:?})"),
    };
    let b2 = match FixtureBuilder::with_capacity(cap) {
        Ok(v) => v,
        Err(e) => panic!("FixtureBuilder::with_capacity(cap) should succeed, got Err({e:?})"),
    };
    let bytes_a = b1.build_bytes(1);
    let bytes_b = b2.build_bytes(2);
    assert_ne!(bytes_a, bytes_b);
}

#[test]
fn fixture_builder_build_bytes_max_capacity() {
    let cap = expect_fixture_capacity_ok(FixtureCapacity::new(FixtureCapacity::MAX_CAPACITY));
    let builder = match FixtureBuilder::with_capacity(cap) {
        Ok(v) => v,
        Err(e) => panic!("FixtureBuilder::with_capacity(cap) should succeed, got Err({e:?})"),
    };
    let bytes = builder.build_bytes(42);
    assert_eq!(bytes.len(), FixtureCapacity::MAX_CAPACITY);
}

#[test]
fn fixture_builder_with_capacity_minimum_one_byte() {
    let cap = expect_fixture_capacity_ok(FixtureCapacity::new(1));
    let builder = match FixtureBuilder::with_capacity(cap) {
        Ok(v) => v,
        Err(e) => panic!("FixtureBuilder::with_capacity(cap) should succeed, got Err({e:?})"),
    };
    let bytes = builder.build_bytes(0);
    assert_eq!(bytes.len(), 1);
}

fn expect_seeded_bytes_ok<const N: usize>(result: Option<SeededBytes<N>>) -> SeededBytes<N> {
    match result {
        Some(v) => v,
        None => panic!("SeededBytes::<{N}>::new should succeed for N > 0"),
    }
}

// -- SeededBytes -------------------------------------------------------------

#[test]
fn seeded_bytes_determinism_across_calls() {
    let a = expect_seeded_bytes_ok(SeededBytes::<32>::new(42));
    let b = expect_seeded_bytes_ok(SeededBytes::<32>::new(42));
    assert_eq!(a, b);
}

#[test]
fn seeded_bytes_different_seeds_produce_different_bytes() {
    let a = expect_seeded_bytes_ok(SeededBytes::<32>::new(1));
    let b = expect_seeded_bytes_ok(SeededBytes::<32>::new(2));
    assert_ne!(a, b);
}

#[test]
fn seeded_bytes_zero_size_returns_none() {
    let result = SeededBytes::<0>::new(42);
    assert!(result.is_none());
}

#[test]
fn seeded_bytes_field_accessible() {
    let s = expect_seeded_bytes_ok(SeededBytes::<8>::new(0));
    assert_eq!(s.bytes.len(), 8);
}

#[test]
fn seeded_bytes_clone_eq() {
    let a = expect_seeded_bytes_ok(SeededBytes::<16>::new(99));
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn seeded_bytes_debug_includes_struct_name() {
    let s = expect_seeded_bytes_ok(SeededBytes::<4>::new(0));
    let debug = format!("{:?}", s);
    assert!(debug.contains("SeededBytes"));
}

#[test]
fn seeded_bytes_partial_eq_negative() {
    let a = expect_seeded_bytes_ok(SeededBytes::<8>::new(1));
    let b = expect_seeded_bytes_ok(SeededBytes::<8>::new(2));
    assert_ne!(a, b);
}

#[test]
fn seeded_bytes_large_size() {
    let a = expect_seeded_bytes_ok(SeededBytes::<1024>::new(777));
    let b = expect_seeded_bytes_ok(SeededBytes::<1024>::new(777));
    assert_eq!(a, b);
    assert_eq!(a.bytes.len(), 1024);
}

fn expect_temp_keyspace_ok(result: Result<TempKeyspace, TestSetupError>) -> TempKeyspace {
    match result {
        Ok(v) => v,
        Err(e) => panic!("TempKeyspace::open() should succeed, got Err({e:?})"),
    }
}

// -- TempKeyspace ------------------------------------------------------------

#[test]
fn temp_keyspace_open_succeeds() {
    let temp = expect_temp_keyspace_ok(TempKeyspace::open());
    let path = temp.path();
    assert!(path.exists());
    assert!(path.is_dir());
}

#[test]
fn temp_keyspace_path_is_directory() {
    let temp = expect_temp_keyspace_ok(TempKeyspace::open());
    assert!(temp.path().is_dir());
}

#[test]
fn temp_keyspace_database_handle_accessible() {
    let temp = expect_temp_keyspace_ok(TempKeyspace::open());
    let _db = temp.database();
    let _path = temp.path();
}

#[test]
fn temp_keyspace_cleanup_on_drop() {
    let temp = expect_temp_keyspace_ok(TempKeyspace::open());
    let path = temp.path().to_path_buf();
    assert!(path.exists());
    drop(temp);
    assert!(!path.exists());
}

#[test]
fn temp_keyspace_uniqueness_sequential() {
    use std::collections::HashSet;
    let mut paths = HashSet::new();
    for _ in 0..5 {
        let temp = expect_temp_keyspace_ok(TempKeyspace::open());
        let path = temp.path().to_path_buf();
        assert!(paths.insert(path), "temp keyspaces must have unique paths");
    }
}

#[test]
fn temp_keyspace_path_preserved_across_database_access() {
    let temp = expect_temp_keyspace_ok(TempKeyspace::open());
    let p1 = temp.path().to_path_buf();
    let _db = temp.database();
    let p2 = temp.path().to_path_buf();
    assert_eq!(p1, p2);
}
