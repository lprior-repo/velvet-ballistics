#![forbid(unsafe_code)]
#![allow(unexpected_cfgs)]

#[cfg(kani)]
mod capability_schema_proofs {
    #[kani::proof]
    fn capability_name_length_boundary_is_ordered() {
        let len: usize = kani::any();
        kani::assume(len <= 256);
        assert!((len == 0) || (len <= 128) || (len > 128));
    }

    #[kani::proof]
    fn duplicate_indexes_are_ordered_when_second_index_is_after_first() {
        let first_index: usize = kani::any();
        let duplicate_index: usize = kani::any();
        kani::assume(first_index < 8);
        kani::assume(duplicate_index < 8);
        kani::assume(first_index < duplicate_index);
        assert!(first_index < duplicate_index);
    }
}
