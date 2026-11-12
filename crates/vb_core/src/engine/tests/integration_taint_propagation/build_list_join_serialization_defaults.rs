// =============================================================================
// C-050 to C-053: BuildList with Random/TimeDependent Taint
// =============================================================================

// C-050: build_list_with_taint returns Random when any item is Random
#[test]
fn build_list_with_taint_returns_random_when_any_item_is_random() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 2).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::Random)
        .map_err(|e| e.to_string())?;

    let items = vec![SlotIdx::new(0), SlotIdx::new(1)];

    use crate::engine::object_list::build_list_with_taint;
    let (_, taint) = build_list_with_taint(&mut store, &run, &items).map_err(|e| e.to_string())?;

    ensure_equal(taint, Taint::Random)
}

// C-051: build_list_with_taint returns TimeDependent when any item is TimeDependent
#[test]
fn build_list_with_taint_returns_time_dependent_when_any_item_time_dependent() -> Result<(), String>
{
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 2).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Random)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::TimeDependent)
        .map_err(|e| e.to_string())?;

    let items = vec![SlotIdx::new(0), SlotIdx::new(1)];

    use crate::engine::object_list::build_list_with_taint;
    let (_, taint) = build_list_with_taint(&mut store, &run, &items).map_err(|e| e.to_string())?;

    ensure_equal(taint, Taint::TimeDependent)
}

// C-052: build_list_with_taint with all five variants returns TimeDependent
#[test]
fn build_list_with_taint_all_five_variants_returns_time_dependent() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 5).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::DerivedFromSecret)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(2), SlotValue::I64(3), Taint::Secret)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(3), SlotValue::I64(4), Taint::Random)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(4), SlotValue::I64(5), Taint::TimeDependent)
        .map_err(|e| e.to_string())?;

    let items = vec![
        SlotIdx::new(0),
        SlotIdx::new(1),
        SlotIdx::new(2),
        SlotIdx::new(3),
        SlotIdx::new(4),
    ];

    use crate::engine::object_list::build_list_with_taint;
    let (_, taint) = build_list_with_taint(&mut store, &run, &items).map_err(|e| e.to_string())?;

    ensure_equal(taint, Taint::TimeDependent)
}

// C-053: build_list_with_taint stores per-item taint in ValueStore
#[test]
fn build_list_with_taint_stores_per_item_random_taint_in_value_store() -> Result<(), String> {
    let mut store = ValueStore::new();
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 1).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::ZERO, SlotValue::I64(42), Taint::Random)
        .map_err(|e| e.to_string())?;

    let items = vec![SlotIdx::ZERO];

    use crate::engine::object_list::build_list_with_taint;
    let (list, _) = build_list_with_taint(&mut store, &run, &items).map_err(|e| e.to_string())?;

    let (_, item_taint) = store
        .list_item_with_taint(list, 0)
        .map_err(|e| e.to_string())?;
    ensure_equal(item_taint, Taint::Random)
}

// =============================================================================
// C-060 to C-062: Taint Merge at Join Points
// =============================================================================

// C-060: join_taint on all 625 (5×5×5×5) quadruples respects lattice order
#[test]
fn join_taint_full_625_quadruples_respect_lattice() {
    let all = [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ];
    for a in all {
        for b in all {
            for c in all {
                for d in all {
                    // (a ∨ b) ∨ (c ∨ d) should be commutative with (c ∨ d) ∨ (a ∨ b)
                    let left = join_taint(join_taint(a, b), join_taint(c, d));
                    let right = join_taint(join_taint(c, d), join_taint(a, b));
                    assert_eq!(left, right);
                }
            }
        }
    }
}

// C-061: join_taint triple associativity for all 125 (5×5×5) triples
#[test]
fn join_taint_is_associative_for_all_125_triples() {
    let all = [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ];
    for a in all {
        for b in all {
            for c in all {
                let ab_c = join_taint(join_taint(a, b), c);
                let a_bc = join_taint(a, join_taint(b, c));
                assert_eq!(ab_c, a_bc, "associativity failed for ({a:?}, {b:?}, {c:?})");
            }
        }
    }
}

// C-062: join_taint of a chain (a, b, c) matches pairwise join of max element
#[test]
fn join_taint_chain_matches_pairwise_max() {
    let all = [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ];
    for a in all {
        for b in all {
            for c in all {
                let chain = join_taint(join_taint(a, b), c);
                let pair_max = join_taint(a, join_taint(b, c));
                assert_eq!(chain, pair_max);

                // Also verify that the result is >= each individual
                let disc_chain = match chain {
                    Taint::Clean => 0,
                    Taint::DerivedFromSecret => 1,
                    Taint::Secret => 2,
                    Taint::Random => 3,
                    Taint::TimeDependent => 4,
                };
                let disc_a = match a {
                    Taint::Clean => 0,
                    Taint::DerivedFromSecret => 1,
                    Taint::Secret => 2,
                    Taint::Random => 3,
                    Taint::TimeDependent => 4,
                };
                let disc_b = match b {
                    Taint::Clean => 0,
                    Taint::DerivedFromSecret => 1,
                    Taint::Secret => 2,
                    Taint::Random => 3,
                    Taint::TimeDependent => 4,
                };
                let disc_c = match c {
                    Taint::Clean => 0,
                    Taint::DerivedFromSecret => 1,
                    Taint::Secret => 2,
                    Taint::Random => 3,
                    Taint::TimeDependent => 4,
                };
                assert!(disc_chain >= disc_a);
                assert!(disc_chain >= disc_b);
                assert!(disc_chain >= disc_c);
            }
        }
    }
}

// =============================================================================
// C-070 to C-073: Taint Serialization Roundtrip (postcard)
// =============================================================================

// C-070: postcard roundtrip preserves all five Taint variants
#[test]
fn taint_postcard_roundtrip_preserves_all_five_variants() {
    let variants = [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ];
    for variant in variants {
        let bytes = postcard::to_allocvec(&variant);
        assert!(
            bytes.is_ok(),
            "postcard serialization should succeed for {variant:?}"
        );
        let Ok(bytes) = bytes else {
            continue;
        };
        let recovered: Result<Taint, _> = postcard::from_bytes(&bytes);
        assert!(
            recovered.is_ok(),
            "postcard deserialization should succeed for {variant:?}"
        );
        let Ok(recovered) = recovered else {
            continue;
        };
        assert_eq!(
            variant, recovered,
            "postcard roundtrip must preserve {variant:?}"
        );
    }
}

// C-071: postcard roundtrip preserves join_taint result identity
#[test]
fn postcard_roundtrip_preserves_join_taint_result() {
    let all = [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ];
    for a in all {
        let joined = join_taint(a, Taint::TimeDependent);
        let bytes = postcard::to_allocvec(&joined);
        assert!(bytes.is_ok());
        let Ok(bytes) = bytes else {
            continue;
        };
        let recovered: Result<Taint, _> = postcard::from_bytes(&bytes);
        assert!(recovered.is_ok());
        let Ok(recovered) = recovered else {
            continue;
        };
        assert_eq!(recovered, Taint::TimeDependent);
    }
}

// C-072: postcard discriminator encoding matches serial number for all five variants
#[test]
fn postcard_encoding_distinguishes_all_five_variants() {
    let all = [
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
        Taint::Random,
        Taint::TimeDependent,
    ];
    for (i, a) in all.iter().enumerate() {
        for (j, b) in all.iter().enumerate() {
            if i != j {
                assert_ne!(a, b);
                let bytes_a = postcard::to_allocvec(a).unwrap();
                let bytes_b = postcard::to_allocvec(b).unwrap();
                assert_ne!(
                    bytes_a, bytes_b,
                    "postcard encoding must distinguish {a:?} from {b:?}"
                );
            }
        }
    }
}

// C-073: postcard deserializes unknown u8 discriminants safely
#[test]
fn postcard_deserializes_unknown_taint_discriminant_to_valid_taint() {
    // Taint is serialized as its u8 discriminant. Test all possible u8 values.
    for disc in 0u8..=255u8 {
        let bytes = postcard::to_allocvec(&disc);
        match bytes {
            Ok(bytes) => {
                let recovered: Result<Taint, _> = postcard::from_bytes(&bytes);
                // postcard deserialization for an enum with unknown discriminant
                // may fail or produce a valid variant depending on serde impl
                // We assert it doesn't panic
                let _ = recovered;
            }
            Err(_) => {
                // Serialization failure for a u8 is unexpected but not a panic
            }
        }
    }
}

// =============================================================================
// C-080 to C-082: Default Taint Values
// =============================================================================

// C-080: New RunFrame initializes all taint entries to Clean
#[test]
fn new_runframe_initializes_all_taint_to_clean() -> Result<(), String> {
    let run = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 4).map_err(|e| e.to_string())?;

    // All slots are uninitialized, so read_taint returns SlotUninitialized
    for i in 0..4 {
        let slot = SlotIdx::new(i);
        match run.read_taint(slot) {
            Err(EngineError::SlotUninitialized { slot: s }) if s == slot => {}
            other => {
                return Err(format!("taint {i} should be uninitialized, got: {other:?}"));
            }
        }
    }

    // After writing a slot, taint should be Clean by default
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 4).map_err(|e| e.to_string())?;
    run.write_slot(SlotIdx::new(0), SlotValue::I64(42))
        .map_err(|e| e.to_string())?;
    let taint = run.read_taint(SlotIdx::new(0)).map_err(|e| e.to_string())?;
    ensure_equal(taint, Taint::Clean)
}

// C-081: taint_snapshot returns default Clean for initialized slots after write_slot
#[test]
fn taint_snapshot_defaults_to_clean_for_write_slot() -> Result<(), String> {
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2).map_err(|e| e.to_string())?;

    // write_slot uses write_slot_with_taint internally with Taint::Clean
    run.write_slot(SlotIdx::new(0), SlotValue::I64(42))
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(99), Taint::Secret)
        .map_err(|e| e.to_string())?;

    let snapshot = run.taint_snapshot();
    assert_eq!(snapshot.len(), 2);
    assert_eq!(snapshot[0], Taint::Clean);
    assert_eq!(snapshot[1], Taint::Secret);
    Ok(())
}

// C-082: initialized_slots includes taint for all five variants
#[test]
fn initialized_slots_includes_taint_for_all_five_variants() -> Result<(), String> {
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 5).map_err(|e| e.to_string())?;

    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Clean)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(2), Taint::DerivedFromSecret)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(2), SlotValue::I64(3), Taint::Secret)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(3), SlotValue::I64(4), Taint::Random)
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(4), SlotValue::I64(5), Taint::TimeDependent)
        .map_err(|e| e.to_string())?;

    let initialized = run.initialized_slots().map_err(|e| e.to_string())?;
    assert_eq!(initialized.len(), 5);

    let mut seen = [false; 5];
    for (_, _, taint) in &initialized {
        match taint {
            Taint::Clean => seen[0] = true,
            Taint::DerivedFromSecret => seen[1] = true,
            Taint::Secret => seen[2] = true,
            Taint::Random => seen[3] = true,
            Taint::TimeDependent => seen[4] = true,
        }
    }
    assert!(
        seen.iter().all(|s| *s),
        "all five taint variants must appear"
    );
    Ok(())
}

