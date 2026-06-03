//! Tests for the arena allocator.

use super::types::{ArenaError, Generation, SlotHandle, SlotId};
use super::{Arena, SlotSet};

#[test]
fn slot_id_constants() {
    assert!(SlotId::INVALID.is_invalid());
    assert!(!SlotId::new(0).is_invalid());
}

#[test]
fn generation_successor() {
    let g = Generation::INITIAL;
    assert_eq!(g.successor(), Generation(1));
    assert_eq!(Generation::TERMINAL.successor(), Generation::TERMINAL);
    assert!(!g.is_terminal());
}

#[test]
fn arena_allocate_deallocate() {
    let mut arena: Arena<String> = Arena::new();

    let handle1 = arena.allocate("test".to_string()).unwrap();
    assert_eq!(arena.get(handle1).unwrap(), "test");
    assert_eq!(handle1.generation(), Generation::INITIAL);

    arena.deallocate(handle1).unwrap();
    assert!(matches!(
        arena.get(handle1),
        Err(ArenaError::GenerationMismatch)
    ));

    // Reuse slot
    let handle2 = arena.allocate("test2".to_string()).unwrap();
    assert_eq!(handle2.slot_id(), handle1.slot_id());
    assert_eq!(handle2.generation(), handle1.generation().successor());
    assert_eq!(arena.get(handle2).unwrap(), "test2");
    assert!(matches!(
        arena.get(handle1),
        Err(ArenaError::GenerationMismatch)
    ));
}

#[test]
fn arena_contains() {
    let mut arena: Arena<i32> = Arena::new();
    let handle = arena.allocate(42).unwrap();

    assert!(arena.contains(handle));
    assert!(!Arena::<i32>::new().contains(handle));

    arena.deallocate(handle).unwrap();
    assert!(!arena.contains(handle));
}

#[test]
fn slot_set_basic() {
    let mut set = SlotSet::new();
    let handle = set.arena.allocate(()).unwrap();

    assert!(set.contains(handle));
    assert_eq!(set.len(), 1);

    set.insert(handle).unwrap();
    assert_eq!(set.len(), 1);

    set.remove(handle).unwrap();
    assert!(!set.contains(handle));
    assert!(set.is_empty());
}

#[test]
fn slot_set_rejects_invalid_or_gapped_handle() {
    let mut set = SlotSet::new();
    let invalid = SlotHandle::new(SlotId::INVALID, Generation::INITIAL);
    let gapped = SlotHandle::new(SlotId::new(8), Generation::INITIAL);

    assert_eq!(set.insert(invalid), Err(ArenaError::InvalidSlotId));
    assert_eq!(set.insert(gapped), Err(ArenaError::InvalidSlotId));
    assert!(set.is_empty());
}

#[test]
fn slot_set_rejects_stale_reinsert() {
    let mut set = SlotSet::new();
    let handle = SlotHandle::new(SlotId::new(0), Generation::INITIAL);
    let successor = SlotHandle::new(handle.slot_id(), handle.generation().successor());

    set.insert(handle).unwrap();
    set.remove(handle).unwrap();

    assert_eq!(set.insert(handle), Err(ArenaError::GenerationMismatch));
    set.insert(successor).unwrap();
    assert!(set.contains(successor));
}

#[test]
#[ignore = "types lack Default impl — must be rewritten with proper construction"]
fn arena_manager_deallocate_all() {
    todo!(
        "arena_manager_deallocate_all requires Default on RunState, RuntimeState, PendingTimer, FramePool"
    )
}
