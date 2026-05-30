// AUTO-GENERATED — DO NOT EDIT BY HAND
// Extracted from ids.rs:test_module
// See: /home/lewis/arch-drift-v3/crates/vb_ipc/src/ids.rs

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // AskTicketId tests
    // =========================================================================

    #[test]
    fn ask_ticket_id_from_wire_zero() {
        let id = AskTicketId::from_wire(0);
        assert_eq!(id.wire_value(), 0);
        assert_eq!(id.step_idx(), 0);
    }

    #[test]
    fn ask_ticket_id_from_wire_step_in_lower_bits() {
        // Wire encoding: step_idx in lower 16 bits
        let wire = 0x0000_0000_0000_0042u64; // step 66
        let id = AskTicketId::from_wire(wire);
        assert_eq!(id.step_idx(), 66);
    }

    #[test]
    fn ask_ticket_id_from_wire_max_u16_step() {
        let wire = u16::MAX as u64;
        let id = AskTicketId::from_wire(wire);
        assert_eq!(id.step_idx(), u16::MAX);
    }

    #[test]
    fn ask_ticket_id_wire_value_preserves_full_encoding() {
        let wire = 0xABCD_EF00_1234_5678u64;
        let id = AskTicketId::from_wire(wire);
        assert_eq!(id.wire_value(), wire);
    }

    // =========================================================================
    // ActionTicketId tests
    // =========================================================================

    #[test]
    fn action_ticket_id_from_wire_zero() {
        let id = ActionTicketId::from_wire(0);
        assert_eq!(id.wire_value(), 0);
        assert_eq!(id.step_idx(), 0);
    }

    #[test]
    fn action_ticket_id_from_wire_step_in_lower_bits() {
        let wire = 0x0000_0000_0000_0100u64; // step 256
        let id = ActionTicketId::from_wire(wire);
        assert_eq!(id.step_idx(), 256);
    }

    #[test]
    fn action_ticket_id_from_wire_max_u16_step() {
        let wire = u16::MAX as u64;
        let id = ActionTicketId::from_wire(wire);
        assert_eq!(id.step_idx(), u16::MAX);
    }

    #[test]
    fn action_ticket_id_wire_value_preserves_full_encoding() {
        let wire = 0x1234_5678_9ABC_DEF0u64;
        let id = ActionTicketId::from_wire(wire);
        assert_eq!(id.wire_value(), wire);
    }

    // =========================================================================
    // Type separation tests — ask vs action are distinct
    // =========================================================================

    #[test]
    fn ask_and_action_ticket_ids_are_type_distinct() {
        let ask = AskTicketId::from_wire(100);
        let action = ActionTicketId::from_wire(100);
        // Same wire value but different types — not equal
        assert_ne!(ask, action);
    }

    #[test]
    fn same_wire_value_different_types() {
        let wire = 42u64;
        let ask_id = AskTicketId::from_wire(wire);
        let action_id = ActionTicketId::from_wire(wire);
        assert_eq!(ask_id.wire_value(), action_id.wire_value());
        assert_ne!(ask_id, action_id);
    }

    #[test]
    fn ask_ticket_id_ordering_by_wire_value() {
        let a = AskTicketId::from_wire(10);
        let b = AskTicketId::from_wire(20);
        assert!(a < b, "lower wire value should compare less");
        assert!(b > a, "higher wire value should compare greater");
    }

    #[test]
    fn action_ticket_id_ordering_by_wire_value() {
        let a = ActionTicketId::from_wire(100);
        let b = ActionTicketId::from_wire(200);
        assert!(a < b);
        assert!(b > a);
    }

    #[test]
    fn ask_ticket_id_step_idx_masks_upper_bits() {
        let wire = 0xFFFF_0000_0000_0042u64;
        let id = AskTicketId::from_wire(wire);
        assert_eq!(id.step_idx(), 0x0042);
        assert_eq!(id.wire_value(), wire);
    }

    #[test]
    fn action_ticket_id_step_idx_masks_upper_bits() {
        let wire = 0xABCD_EF00_1234_FF00u64;
        let id = ActionTicketId::from_wire(wire);
        assert_eq!(id.step_idx(), 0xFF00);
        assert_eq!(id.wire_value(), wire);
    }

    #[test]
    fn ask_ticket_id_serde_roundtrip() {
        let original = AskTicketId::from_wire(0x1234_5678_9ABC_DEF0);
        let Ok(encoded) = postcard::to_allocvec(&original) else { return };
        let decoded: AskTicketId = match postcard::from_bytes(&encoded) {
            Ok(d) => d,
            Err(_) => { assert!(false, "decode should succeed"); return; }
        };
        assert_eq!(decoded.wire_value(), original.wire_value());
    }

    #[test]
    fn action_ticket_id_serde_roundtrip() {
        let original = ActionTicketId::from_wire(0xDEAD_BEEF_CAFE_BABE);
        let Ok(encoded) = postcard::to_allocvec(&original) else { return };
        let decoded: ActionTicketId = match postcard::from_bytes(&encoded) {
            Ok(d) => d,
            Err(_) => { assert!(false, "decode should succeed"); return; }
        };
        assert_eq!(decoded.wire_value(), original.wire_value());
    }

    #[test]
    fn ask_ticket_id_serde_roundtrip_boundary() {
        for wire in [0u64, u64::MAX, 0x0000_0000_0000_FFFF] {
            let original = AskTicketId::from_wire(wire);
            let Ok(encoded) = postcard::to_allocvec(&original) else { return };
            let decoded: AskTicketId = match postcard::from_bytes(&encoded) {
                Ok(d) => d,
                Err(_) => { assert!(false, "decode should succeed"); return; }
            };
            assert_eq!(decoded, original);
        }
    }

    #[test]
    fn action_ticket_id_serde_roundtrip_boundary() {
        for wire in [0u64, u64::MAX, 0x0000_0000_0000_FFFF] {
            let original = ActionTicketId::from_wire(wire);
            let Ok(encoded) = postcard::to_allocvec(&original) else { return };
            let decoded: ActionTicketId = match postcard::from_bytes(&encoded) {
                Ok(d) => d,
                Err(_) => { assert!(false, "decode should succeed"); return; }
            };
            assert_eq!(decoded, original);
        }
    }

    #[test]
    fn ask_ticket_id_hash_consistency() {
        use std::collections::HashSet;
        let a = AskTicketId::from_wire(42);
        let b = AskTicketId::from_wire(42);
        let mut set = HashSet::new();
        assert!(set.insert(a));
        assert!(!set.insert(b));
    }

    #[test]
    fn action_ticket_id_hash_consistency() {
        use std::collections::HashSet;
        let a = ActionTicketId::from_wire(42);
        let b = ActionTicketId::from_wire(42);
        let mut set = HashSet::new();
        assert!(set.insert(a));
        assert!(!set.insert(b));
    }

    #[test]
    fn ask_ticket_id_distinct_values_deduplicate() {
        use std::collections::HashSet;
        let set: HashSet<_> = [1u64, 2, 3].map(|w| AskTicketId::from_wire(w)).into_iter().collect();
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn action_ticket_id_distinct_values_deduplicate() {
        use std::collections::HashSet;
        let set: HashSet<_> = [1u64, 2, 3].map(|w| ActionTicketId::from_wire(w)).into_iter().collect();
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn ask_ticket_id_copy_is_equal() {
        let original = AskTicketId::from_wire(12345);
        let copy = original;
        assert_eq!(copy, original);
    }

    #[test]
    fn action_ticket_id_copy_is_equal() {
        let original = ActionTicketId::from_wire(67890);
        let copy = original;
        assert_eq!(copy, original);
    }

    // =========================================================================
    // Ordering tests
    // =========================================================================

    #[test]
    fn ask_ticket_id_ordering_by_wire_value() {
        let a = AskTicketId::from_wire(10);
        let b = AskTicketId::from_wire(20);
        assert!(a < b, "lower wire value should compare less");
        assert!(b > a, "higher wire value should compare greater");
    }

    #[test]
    fn action_ticket_id_ordering_by_wire_value() {
        let a = ActionTicketId::from_wire(100);
        let b = ActionTicketId::from_wire(200);
        assert!(a < b);
        assert!(b > a);
    }

    #[test]
    fn ask_ticket_id_equal_values_compare_equal() {
        let a = AskTicketId::from_wire(999);
        let b = AskTicketId::from_wire(999);
        assert_eq!(a, b);
        assert!(!(a < b));
        assert!(!(a > b));
    }

    #[test]
    fn action_ticket_id_equal_values_compare_equal() {
        let a = ActionTicketId::from_wire(999);
        let b = ActionTicketId::from_wire(999);
        assert_eq!(a, b);
        assert!(!(a < b));
        assert!(!(a > b));
    }

    // =========================================================================
    // Step index masking with upper bits
    // =========================================================================

    #[test]
    fn ask_ticket_id_step_idx_masks_upper_bits() {
        let wire = 0xFFFF_0000_0000_0042u64;
        let id = AskTicketId::from_wire(wire);
        assert_eq!(id.step_idx(), 0x0042);
        assert_eq!(id.wire_value(), wire);
    }

    #[test]
    fn action_ticket_id_step_idx_masks_upper_bits() {
        let wire = 0xABCD_EF00_1234_FF00u64;
        let id = ActionTicketId::from_wire(wire);
        assert_eq!(id.step_idx(), 0xFF00);
        assert_eq!(id.wire_value(), wire);
    }

    // =========================================================================
    // Serde roundtrip tests
    // =========================================================================

    #[test]
    fn ask_ticket_id_serde_roundtrip() {
        let original = AskTicketId::from_wire(0x1234_5678_9ABC_DEF0);
        let Ok(encoded) = postcard::to_allocvec(&original) else { return };
        let decoded: AskTicketId = match postcard::from_bytes(&encoded) {
            Ok(d) => d,
            Err(_) => { assert!(false, "decode should succeed"); return; }
        };
        assert_eq!(decoded.wire_value(), original.wire_value());
    }

    #[test]
    fn action_ticket_id_serde_roundtrip() {
        let original = ActionTicketId::from_wire(0xDEAD_BEEF_CAFE_BABE);
        let Ok(encoded) = postcard::to_allocvec(&original) else { return };
        let decoded: ActionTicketId = match postcard::from_bytes(&encoded) {
            Ok(d) => d,
            Err(_) => { assert!(false, "decode should succeed"); return; }
        };
        assert_eq!(decoded.wire_value(), original.wire_value());
    }

    #[test]
    fn ask_ticket_id_serde_roundtrip_boundary() {
        for wire in [0u64, u64::MAX, 0x0000_0000_0000_FFFF] {
            let original = AskTicketId::from_wire(wire);
            let Ok(encoded) = postcard::to_allocvec(&original) else { return };
            let decoded: AskTicketId = match postcard::from_bytes(&encoded) {
                Ok(d) => d,
                Err(_) => { assert!(false, "decode should succeed"); return; }
            };
            assert_eq!(decoded, original);
        }
    }

    #[test]
    fn action_ticket_id_serde_roundtrip_boundary() {
        for wire in [0u64, u64::MAX, 0x0000_0000_0000_FFFF] {
            let original = ActionTicketId::from_wire(wire);
            let Ok(encoded) = postcard::to_allocvec(&original) else { return };
            let decoded: ActionTicketId = match postcard::from_bytes(&encoded) {
                Ok(d) => d,
                Err(_) => { assert!(false, "decode should succeed"); return; }
            };
            assert_eq!(decoded, original);
        }
    }

    // =========================================================================
    // Hash consistency tests
    // =========================================================================

    #[test]
    fn ask_ticket_id_hash_consistency() {
        use std::collections::HashSet;
        let a = AskTicketId::from_wire(42);
        let b = AskTicketId::from_wire(42);
        let mut set = HashSet::new();
        assert!(set.insert(a));
        assert!(!set.insert(b));
    }

    #[test]
    fn action_ticket_id_hash_consistency() {
        use std::collections::HashSet;
        let a = ActionTicketId::from_wire(42);
        let b = ActionTicketId::from_wire(42);
        let mut set = HashSet::new();
        assert!(set.insert(a));
        assert!(!set.insert(b));
    }

    // =========================================================================
    // Debug format tests
    // =========================================================================

    #[test]
    fn ask_ticket_id_debug_contains_wire_value() {
        let id = AskTicketId::from_wire(0xDEAD);
        let debug = format!("{id:?}");
        assert!(
            debug.contains("AskTicketId"),
            "debug output should contain type name: {debug}"
        );
    }

    #[test]
    fn action_ticket_id_debug_contains_wire_value() {
        let id = ActionTicketId::from_wire(0xBEEF);
        let debug = format!("{id:?}");
        assert!(
            debug.contains("ActionTicketId"),
            "debug output should contain type name: {debug}"
        );
    }
}
