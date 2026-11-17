mod planned_depth_update_tests {
    use super::*;

    #[test]
    fn existing_planned_depth_replacement_preserves_keys_and_order() -> RuntimeResult<()> {
        let mut depths = migration_index_buffer(7)?;
        for key in 0_u64..7 {
            replace_or_insert_planned_depth(&mut depths, RunId::new(key), 64);
        }
        let before = depths.keys().copied().collect::<Vec<_>>();
        replace_or_insert_planned_depth(&mut depths, RunId::new(3), 12);
        assert_eq!(depths.len(), 7);
        assert_eq!(depths.keys().copied().collect::<Vec<_>>(), before);
        for key in 0_u64..7 {
            assert_eq!(
                depths.get(&RunId::new(key)).copied(),
                Some(if key == 3 { 12 } else { 64 })
            );
        }
        Ok(())
    }

    #[test]
    fn absent_planned_depth_is_inserted_after_existing_keys() -> RuntimeResult<()> {
        let mut depths = migration_index_buffer(2)?;
        replace_or_insert_planned_depth(&mut depths, RunId::new(9), 17);
        replace_or_insert_planned_depth(&mut depths, RunId::new(4), 5);
        assert_eq!(
            depths
                .iter()
                .map(|(&run, &depth)| (run, depth))
                .collect::<Vec<_>>(),
            vec![(RunId::new(9), 17), (RunId::new(4), 5)]
        );
        Ok(())
    }
}
