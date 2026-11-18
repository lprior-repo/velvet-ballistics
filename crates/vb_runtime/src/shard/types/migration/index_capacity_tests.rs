mod migration_index_capacity_tests {
    use super::*;

    #[test]
    fn eager_index_capacity_covers_full_bound_and_reuse_without_growth() -> RuntimeResult<()> {
        for limit in [1_u16, 2, 3, 8, 17, 1024] {
            let mut entries = migration_index_buffer(usize::from(limit))?;
            let allocated = entries.capacity();
            assert!(allocated >= usize::from(limit));
            for run in 0..limit {
                assert_eq!(
                    entries.insert(RunId::new(u64::from(run)), u32::from(run)),
                    None
                );
            }
            assert_eq!(entries.len(), usize::from(limit));
            assert_eq!(entries.capacity(), allocated);
            for run in 0..limit {
                assert_eq!(
                    entries.swap_remove(&RunId::new(u64::from(run))),
                    Some(u32::from(run))
                );
            }
            assert_eq!(entries.capacity(), allocated);
            for run in 0..limit {
                assert_eq!(
                    entries.insert(RunId::new(u64::from(run)), u32::from(run)),
                    None
                );
            }
            entries.clear();
            assert_eq!(entries.capacity(), allocated);
        }
        Ok(())
    }
}
