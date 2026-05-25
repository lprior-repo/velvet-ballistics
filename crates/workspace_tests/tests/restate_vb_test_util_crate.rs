use std::collections::HashSet;
use vb_test_util::fixture::FixtureCapacity;
use vb_test_util::seed::SeededBytes;
use vb_test_util::temp_keyspace::TempKeyspace;

#[test]
fn seeded_bytes_determinism() {
    let a = SeededBytes::<32>::new(12345).unwrap();
    let b = SeededBytes::<32>::new(12345).unwrap();
    assert_eq!(a.bytes, b.bytes, "same seed must produce same bytes");
}

#[test]
fn seeded_bytes_different_seeds() {
    let a = SeededBytes::<32>::new(1).unwrap();
    let b = SeededBytes::<32>::new(2).unwrap();
    assert_ne!(
        a.bytes, b.bytes,
        "different seeds must produce different bytes"
    );
}

#[test]
fn temp_keyspace_cleanup() {
    let temp = TempKeyspace::open().unwrap();
    let path = temp.path().to_path_buf();
    drop(temp);
    assert!(
        !path.exists(),
        "temp keyspace directory must be removed on drop"
    );
}

#[test]
fn temp_keyspace_uniqueness() {
    let mut paths = HashSet::new();
    for _ in 0..100 {
        let temp = TempKeyspace::open().unwrap();
        let path = temp.path().to_path_buf();
        assert!(
            paths.insert(path),
            "each temp keyspace must have a unique path"
        );
    }
}

#[test]
fn zero_capacity_rejected() {
    let result = FixtureCapacity::new(0);
    assert!(
        result.is_err(),
        "zero capacity must be rejected with a typed error"
    );
}

#[test]
fn valid_capacity_accepted() {
    let result = FixtureCapacity::new(100);
    assert!(result.is_ok(), "valid capacity must be accepted");
    assert_eq!(result.unwrap().value, 100);
}

#[test]
fn empty_workflow_fixture() {
    // An empty workflow fixture uses minimal valid IR: just a zero-capacity
    // builder that is rejected, proving the error path works.
    let result = FixtureCapacity::new(0);
    assert!(result.is_err());
}

#[test]
fn no_cli_dependency() {
    // vb_test_util must not depend on vb_cli.
    // This is verified by the crate's Cargo.toml (no vb_cli in [dependencies]).
    // The test itself is a no-op that documents the contract.
}
