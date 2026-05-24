#![forbid(unsafe_code)]
//! Proptest property tests for core types: FiniteF64, Taint lattice, ValueStore, SlotValue.

use proptest::prelude::*;
use vb_core::Taint::{Clean, DerivedFromSecret, Secret};
use vb_core::errors::CoreError;
use vb_core::ids::{BlobId, ListId, ObjectId, SymbolId};
use vb_core::value::{FiniteF64, SlotValue, Taint, join_taint};
use vb_core::value_store::ValueStore;

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

fn any_taint() -> impl Strategy<Value = Taint> {
    prop_oneof![Just(Clean), Just(DerivedFromSecret), Just(Secret)]
}

fn any_finite_bits() -> impl Strategy<Value = u64> {
    // Generate a random u64 and clear the exponent-all-ones pattern that
    // encodes NaN or infinity.  This guarantees a finite f64 every time.
    (any::<u64>()).prop_map(|bits| {
        let exp = (bits >> 52) & 0x7FF;
        if exp == 0x7FF {
            // Replace NaN/Inf exponent with a normal number exponent.
            (bits & !(0x7FF_u64 << 52)) | (0x3FF_u64 << 52)
        } else {
            bits
        }
    })
}

fn any_non_clean_taint() -> impl Strategy<Value = Taint> {
    prop_oneof![Just(DerivedFromSecret), Just(Secret)]
}

/// Helper that inserts into the store by type index (0=symbol, 1=list,
/// 2=object, 3=blob), returning the exact deterministic arena ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InsertedArenaId {
    Symbol(SymbolId),
    List(ListId),
    Object(ObjectId),
    Blob(BlobId),
}

fn insert_by_type(store: &mut ValueStore, typ: u8) -> Result<InsertedArenaId, CoreError> {
    match typ % 4 {
        0 => store.insert_symbol("s").map(InsertedArenaId::Symbol),
        1 => store
            .insert_list(vec![].into_boxed_slice())
            .map(InsertedArenaId::List),
        2 => store
            .insert_object(vec![].into_boxed_slice())
            .map(InsertedArenaId::Object),
        _ => store
            .insert_blob(bytes::Bytes::new())
            .map(InsertedArenaId::Blob),
    }
}

fn expected_id_for_next_insert(
    store: &ValueStore,
    typ: u8,
) -> Result<InsertedArenaId, proptest::test_runner::TestCaseError> {
    match typ % 4 {
        0 => u32::try_from(store.symbol_count())
            .map(|id| InsertedArenaId::Symbol(SymbolId::new(id)))
            .map_err(|_| proptest::test_runner::TestCaseError::fail("symbol count exceeds u32")),
        1 => u32::try_from(store.list_count())
            .map(|id| InsertedArenaId::List(ListId::new(id)))
            .map_err(|_| proptest::test_runner::TestCaseError::fail("list count exceeds u32")),
        2 => u32::try_from(store.object_count())
            .map(|id| InsertedArenaId::Object(ObjectId::new(id)))
            .map_err(|_| proptest::test_runner::TestCaseError::fail("object count exceeds u32")),
        _ => u64::try_from(store.blob_count())
            .map(|id| InsertedArenaId::Blob(BlobId::new(id)))
            .map_err(|_| proptest::test_runner::TestCaseError::fail("blob count exceeds u64")),
    }
}

// ---------------------------------------------------------------------------
// 1. FiniteF64 properties
// ---------------------------------------------------------------------------

proptest! {
    /// Any bit-pattern that represents a finite f64 can be successfully
    /// constructed as a FiniteF64.
    #[test]
    fn finite_f64_accepts_all_finite_bit_patterns(bits in any_finite_bits()) {
        let val = f64::from_bits(bits);
        prop_assert!(val.is_finite(), "strategy must only produce finite values");
        let result = FiniteF64::new(val).map(|finite| finite.get());
        prop_assert_eq!(result, Ok(val));
    }

    /// NaN bit-patterns (quiet and signaling, positive and negative) are
    /// always rejected by FiniteF64::new.
    #[test]
    fn finite_f64_rejects_all_nan_variants(payload in 1u64..0x0040_0000_0000_0000u64) {
        // Construct a NaN by setting the exponent to all-ones and ensuring
        // a non-zero mantissa.  Test both sign bits.
        for sign in [0u64, 1u64 << 63] {
            let nan_bits = sign | 0x7FF0_0000_0000_0000 | (payload & 0x000F_FFFF_FFFF_FFFF);
            let nan_val = f64::from_bits(nan_bits);
            if nan_val.is_nan() {
                prop_assert_eq!(
                    FiniteF64::new(nan_val),
                    Err(CoreError::NonFiniteNumber),
                    "NaN with bits {:#018X} must be rejected",
                    nan_bits,
                );
            }
        }
    }

    /// +/- infinity are always rejected regardless of sign.
    #[test]
    fn finite_f64_rejects_positive_and_negative_infinity(sign in 0u64..=1u64) {
        let inf_bits = (sign << 63) | 0x7FF0_0000_0000_0000u64;
        let val = f64::from_bits(inf_bits);
        if val.is_infinite() {
            prop_assert_eq!(
                FiniteF64::new(val),
                Err(CoreError::NonFiniteNumber),
                "infinity must be rejected"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 2. Taint lattice properties
// ---------------------------------------------------------------------------

proptest! {
    /// join_taint is commutative: join(a, b) == join(b, a).
    #[test]
    fn taint_join_is_commutative(a in any_taint(), b in any_taint()) {
        prop_assert_eq!(join_taint(a, b), join_taint(b, a));
    }

    /// join_taint is associative: join(join(a, b), c) == join(a, join(b, c)).
    #[test]
    fn taint_join_is_associative(
        a in any_taint(),
        b in any_taint(),
        c in any_taint(),
    ) {
        prop_assert_eq!(
            join_taint(join_taint(a, b), c),
            join_taint(a, join_taint(b, c))
        );
    }

    /// join_taint is idempotent: join(a, a) == a.
    #[test]
    fn taint_join_is_idempotent(a in any_taint()) {
        prop_assert_eq!(join_taint(a, a), a);
    }

    /// join_taint(Clean, x) == x for any taint level x.
    #[test]
    fn taint_join_clean_is_identity(x in any_taint()) {
        prop_assert_eq!(join_taint(Clean, x), x);
    }

    /// join_taint(Secret, x) == Secret for any non-Clean x.
    #[test]
    fn taint_join_secret_absorbs_non_clean(x in any_non_clean_taint()) {
        prop_assert_eq!(join_taint(Secret, x), Secret);
    }
}

// ---------------------------------------------------------------------------
// 3. ValueStore arena cap properties
// ---------------------------------------------------------------------------

proptest! {
    /// Inserting entries of mixed types until the cap is reached causes the
    /// next insert (of any type) to fail with BudgetExceeded.  The cap is
    /// never exceeded regardless of insertion order or type.
    #[test]
    fn arena_cap_is_never_exceeded_mixed_types(
        cap in 2u16..10u16,
        type_sequence in proptest::collection::vec(0u8..4, 2..20),
    ) {
        let mut store = ValueStore::with_max_slots(cap);
        let cap_u64 = u64::from(cap);

        for typ in type_sequence {
            if store.total_arena_count() >= cap_u64 {
                // Store is full; any insert must fail.
                let result = insert_by_type(&mut store, typ);
                prop_assert_eq!(
                    result.clone(),
                    Err(CoreError::BudgetExceeded {
                        budget: "max_slots",
                        limit: cap_u64,
                    }),
                    "insert past cap must fail with BudgetExceeded, got {:?}",
                    result
                );
            } else {
                let expected = expected_id_for_next_insert(&store, typ)?;
                let result = insert_by_type(&mut store, typ);
                prop_assert_eq!(
                    result.clone(),
                    Ok(expected),
                    "insert below cap must succeed, got {:?}",
                    result
                );
            }
            prop_assert!(
                store.total_arena_count() <= cap_u64,
                "total_arena_count must never exceed cap"
            );
        }
    }

    /// An uncapped store (cap == 0) accepts any number of inserts.
    #[test]
    fn arena_uncapped_allows_unlimited_inserts(count in 1u16..50u16) {
        let mut store = ValueStore::new();
        prop_assert_eq!(store.max_arena_entries(), 0);
        for i in 0..count {
            let label = format!("s{i}");
            let result = store.insert_symbol(label);
            prop_assert_eq!(result, Ok(SymbolId::new(u32::from(i))));
        }
        prop_assert_eq!(
            store.total_arena_count(),
            u64::from(count)
        );
    }

    /// Different value types (symbol, list, object, blob) all count toward
    /// the same cap.
    #[test]
    fn arena_cap_counts_all_types_toward_same_limit(
        n_symbols in 0u16..3u16,
        n_lists in 0u16..3u16,
        n_objects in 0u16..3u16,
        n_blobs in 0u16..3u16,
    ) {
        let total = u64::from(n_symbols)
            .saturating_add(u64::from(n_lists))
            .saturating_add(u64::from(n_objects))
            .saturating_add(u64::from(n_blobs));
        // Use cap = total so all should succeed.  Skip when total == 0 because
        // a cap of 0 means "uncapped" in ValueStore's convention.
        let cap = match u16::try_from(total) {
            Ok(value) => value,
            Err(_) => {
                return Err(proptest::test_runner::TestCaseError::fail(
                    "total arena count exceeds u16",
                ));
            }
        };
        if cap == 0 {
            return Ok(());
        }
        let mut store = ValueStore::with_max_slots(cap);

        for i in 0..n_symbols {
            let r = store.insert_symbol("x");
            prop_assert_eq!(
                r,
                Ok(SymbolId::new(u32::from(i))),
                "symbol insert within cap must return exact SymbolId"
            );
        }
        for i in 0..n_lists {
            let r = store.insert_list(vec![].into_boxed_slice());
            prop_assert_eq!(
                r,
                Ok(ListId::new(u32::from(i))),
                "list insert within cap must return exact ListId"
            );
        }
        for i in 0..n_objects {
            let r = store.insert_object(vec![].into_boxed_slice());
            prop_assert_eq!(
                r,
                Ok(ObjectId::new(u32::from(i))),
                "object insert within cap must return exact ObjectId"
            );
        }
        for i in 0..n_blobs {
            let r = store.insert_blob(bytes::Bytes::new());
            prop_assert_eq!(
                r,
                Ok(BlobId::new(u64::from(i))),
                "blob insert within cap must return exact BlobId"
            );
        }

        prop_assert_eq!(store.total_arena_count(), total);

        // One more insert of any type must fail.
        let result = store.insert_symbol("overflow");
        prop_assert_eq!(
            result.clone(),
            Err(CoreError::BudgetExceeded {
                budget: "max_slots",
                limit: total,
            }),
            "insert past cap must fail, got {:?}",
            result
        );
    }
}

// ---------------------------------------------------------------------------
// 4. SlotValue handle properties
// ---------------------------------------------------------------------------

proptest! {
    /// Symbol values store a SymbolId handle (u32), not an inline string.
    /// Round-tripping the handle through SlotValue::Symbol preserves the ID.
    #[test]
    fn slot_value_symbol_stores_handle_id(id in 0u32..1000u32) {
        let handle = SymbolId::new(id);
        let val = SlotValue::Symbol(handle);
        prop_assert_eq!(val.type_name(), "symbol");
        // Verify the handle is the correct type by matching.
        let SlotValue::Symbol(recovered) = val else {
            return Err(proptest::test_runner::TestCaseError::fail(
                "expected Symbol variant",
            ));
        };
        prop_assert_eq!(recovered, handle);
        prop_assert_eq!(recovered.get(), id);
    }

    /// List values store a ListId handle (u32).
    #[test]
    fn slot_value_list_stores_handle_id(id in 0u32..1000u32) {
        let handle = ListId::new(id);
        let val = SlotValue::List(handle);
        prop_assert_eq!(val.type_name(), "list");
        let SlotValue::List(recovered) = val else {
            return Err(proptest::test_runner::TestCaseError::fail(
                "expected List variant",
            ));
        };
        prop_assert_eq!(recovered, handle);
    }

    /// Object values store an ObjectId handle (u32).
    #[test]
    fn slot_value_object_stores_handle_id(id in 0u32..1000u32) {
        let handle = ObjectId::new(id);
        let val = SlotValue::Object(handle);
        prop_assert_eq!(val.type_name(), "object");
        let SlotValue::Object(recovered) = val else {
            return Err(proptest::test_runner::TestCaseError::fail(
                "expected Object variant",
            ));
        };
        prop_assert_eq!(recovered, handle);
    }

    /// Blob values store a BlobId handle (u64).
    #[test]
    fn slot_value_blob_stores_handle_id(id in 0u64..1000u64) {
        let handle = BlobId::new(id);
        let val = SlotValue::Blob(handle);
        prop_assert_eq!(val.type_name(), "blob");
        let SlotValue::Blob(recovered) = val else {
            return Err(proptest::test_runner::TestCaseError::fail(
                "expected Blob variant",
            ));
        };
        prop_assert_eq!(recovered, handle);
    }

    /// SlotValue handles remain distinct across different handle types --
    /// a SymbolId(0) is never equal to a ListId(0) slot value.
    #[test]
    fn slot_value_handles_are_type_distinct(id in 0u32..100u32) {
        let sym = SlotValue::Symbol(SymbolId::new(id));
        let list = SlotValue::List(ListId::new(id));
        let obj = SlotValue::Object(ObjectId::new(id));
        let blob = SlotValue::Blob(BlobId::new(u64::from(id)));

        prop_assert_ne!(sym, list, "Symbol and List must differ");
        prop_assert_ne!(sym, obj, "Symbol and Object must differ");
        prop_assert_ne!(sym, blob, "Symbol and Blob must differ");
        prop_assert_ne!(list, obj, "List and Object must differ");
        prop_assert_ne!(list, blob, "List and Blob must differ");
        prop_assert_ne!(obj, blob, "Object and Blob must differ");
    }
}
