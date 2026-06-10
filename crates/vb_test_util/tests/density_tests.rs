//! Cross-module integration tests for `vb_test_util` to satisfy
//! the 5x test-density contract (vb-tdst) and to act as behavior
//! tests for the deterministic fixture, seed, and Fjall test harness.

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

#[test]
fn fixture_capacity_value_field_accessible() {
    let cap = FixtureCapacity::new(128).unwrap();
    assert_eq!(cap.value, 128);
}

#[test]
fn fixture_capacity_clone_eq() {
    let a = FixtureCapacity::new(256).unwrap();
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn fixture_capacity_debug_includes_value() {
    let cap = FixtureCapacity::new(64).unwrap();
    let s = format!("{:?}", cap);
    assert!(s.contains("FixtureCapacity"));
    assert!(s.contains("64"));
}

// -- FixtureBuilder ----------------------------------------------------------

#[test]
fn fixture_builder_with_capacity_accepts_valid() {
    let cap = FixtureCapacity::new(64).unwrap();
    let result = FixtureBuilder::with_capacity(cap);
    assert!(result.is_ok());
}

#[test]
fn fixture_builder_build_bytes_returns_capacity_length() {
    let cap = FixtureCapacity::new(100).unwrap();
    let builder = FixtureBuilder::with_capacity(cap).unwrap();
    let bytes = builder.build_bytes(1);
    assert_eq!(bytes.len(), 100);
}

#[test]
fn fixture_builder_build_bytes_zero_seed_nonempty() {
    let cap = FixtureCapacity::new(32).unwrap();
    let builder = FixtureBuilder::with_capacity(cap).unwrap();
    let bytes = builder.build_bytes(0);
    assert_eq!(bytes.len(), 32);
}

#[test]
fn fixture_builder_build_bytes_determinism_same_seed() {
    let cap = FixtureCapacity::new(64).unwrap();
    let b1 = FixtureBuilder::with_capacity(cap).unwrap();
    let b2 = FixtureBuilder::with_capacity(cap).unwrap();
    let bytes_a = b1.build_bytes(7);
    let bytes_b = b2.build_bytes(7);
    assert_eq!(bytes_a, bytes_b);
}

#[test]
fn fixture_builder_build_bytes_different_seeds_diverge() {
    let cap = FixtureCapacity::new(64).unwrap();
    let b1 = FixtureBuilder::with_capacity(cap).unwrap();
    let b2 = FixtureBuilder::with_capacity(cap).unwrap();
    let bytes_a = b1.build_bytes(1);
    let bytes_b = b2.build_bytes(2);
    assert_ne!(bytes_a, bytes_b);
}

#[test]
fn fixture_builder_build_bytes_max_capacity() {
    let cap = FixtureCapacity::new(FixtureCapacity::MAX_CAPACITY).unwrap();
    let builder = FixtureBuilder::with_capacity(cap).unwrap();
    let bytes = builder.build_bytes(42);
    assert_eq!(bytes.len(), FixtureCapacity::MAX_CAPACITY);
}

#[test]
fn fixture_builder_with_capacity_minimum_one_byte() {
    let cap = FixtureCapacity::new(1).unwrap();
    let builder = FixtureBuilder::with_capacity(cap).unwrap();
    let bytes = builder.build_bytes(0);
    assert_eq!(bytes.len(), 1);
}

// -- SeededBytes -------------------------------------------------------------

#[test]
fn seeded_bytes_determinism_across_calls() {
    let a = SeededBytes::<32>::new(42).unwrap();
    let b = SeededBytes::<32>::new(42).unwrap();
    assert_eq!(a, b);
}

#[test]
fn seeded_bytes_different_seeds_produce_different_bytes() {
    let a = SeededBytes::<32>::new(1).unwrap();
    let b = SeededBytes::<32>::new(2).unwrap();
    assert_ne!(a, b);
}

#[test]
fn seeded_bytes_zero_size_returns_none() {
    let result = SeededBytes::<0>::new(42);
    assert!(result.is_none());
}

#[test]
fn seeded_bytes_field_accessible() {
    let s = SeededBytes::<8>::new(0).unwrap();
    assert_eq!(s.bytes.len(), 8);
}

#[test]
fn seeded_bytes_clone_eq() {
    let a = SeededBytes::<16>::new(99).unwrap();
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn seeded_bytes_debug_includes_struct_name() {
    let s = SeededBytes::<4>::new(0).unwrap();
    let debug = format!("{:?}", s);
    assert!(debug.contains("SeededBytes"));
}

#[test]
fn seeded_bytes_partial_eq_negative() {
    let a = SeededBytes::<8>::new(1).unwrap();
    let b = SeededBytes::<8>::new(2).unwrap();
    assert_ne!(a, b);
}

// -- TempKeyspace ------------------------------------------------------------

#[test]
fn temp_keyspace_open_succeeds() {
    let temp = TempKeyspace::open().unwrap();
    let path = temp.path();
    assert!(path.exists());
    assert!(path.is_dir());
}

#[test]
fn temp_keyspace_path_is_directory() {
    let temp = TempKeyspace::open().unwrap();
    assert!(temp.path().is_dir());
}

#[test]
fn temp_keyspace_database_handle_accessible() {
    let temp = TempKeyspace::open().unwrap();
    let _db = temp.database();
    let _path = temp.path();
}

#[test]
fn temp_keyspace_cleanup_on_drop() {
    let temp = TempKeyspace::open().unwrap();
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
        let temp = TempKeyspace::open().unwrap();
        let path = temp.path().to_path_buf();
        assert!(paths.insert(path));
    }
}

#[test]
fn temp_keyspace_path_preserved_across_database_access() {
    let temp = TempKeyspace::open().unwrap();
    let p1 = temp.path().to_path_buf();
    let _db = temp.database();
    let p2 = temp.path().to_path_buf();
    assert_eq!(p1, p2);
}
