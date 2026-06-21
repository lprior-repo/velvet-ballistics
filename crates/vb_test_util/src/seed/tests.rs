use super::*;

#[test]
fn seeded_bytes_determinism() {
    let a = match SeededBytes::<32>::new(42) {
        Some(v) => v,
        None => panic!("SeededBytes::<32>::new(42) must succeed for non-zero N"),
    };
    let b = match SeededBytes::<32>::new(42) {
        Some(v) => v,
        None => panic!("SeededBytes::<32>::new(42) must succeed for non-zero N"),
    };
    assert_eq!(a.bytes, b.bytes);
}

#[test]
fn seeded_bytes_different_seeds() {
    let a = match SeededBytes::<32>::new(42) {
        Some(v) => v,
        None => panic!("SeededBytes::<32>::new(42) must succeed for non-zero N"),
    };
    let b = match SeededBytes::<32>::new(43) {
        Some(v) => v,
        None => panic!("SeededBytes::<32>::new(43) must succeed for non-zero N"),
    };
    assert_ne!(a.bytes, b.bytes);
}

#[test]
fn seeded_bytes_zero_capacity() {
    let result = SeededBytes::<0>::new(42);
    assert!(result.is_none());
}

#[test]
fn seeded_bytes_single_byte() {
    let a = match SeededBytes::<1>::new(0) {
        Some(v) => v,
        None => panic!("SeededBytes::<1>::new(0) must succeed"),
    };
    let b = match SeededBytes::<1>::new(0) {
        Some(v) => v,
        None => panic!("SeededBytes::<1>::new(0) must succeed"),
    };
    assert_eq!(a.bytes, b.bytes);
    assert_eq!(a.bytes.len(), 1);
}
