mod enqueue_index_update_tests {
    use super::*;
    use std::sync::Mutex;

    type TestResult = Result<(), Box<dyn std::error::Error>>;
    type Entries = IndexMap<RunId, u32>;

    struct Fixture {
        pending: Mutex<Entries>,
        admitted: Mutex<Entries>,
        submissions: Mutex<Entries>,
    }

    impl Fixture {
        fn new() -> RuntimeResult<Self> {
            Ok(Self {
                pending: Mutex::new(migration_index_buffer(7)?),
                admitted: Mutex::new(migration_index_buffer(7)?),
                submissions: Mutex::new(migration_index_buffer(7)?),
            })
        }

        fn indexes(&self) -> Result<PendingQueueIndexes<'_>, Box<dyn std::error::Error>> {
            Ok(PendingQueueIndexes {
                pending: self.pending.lock().map_err(|_| "pending lock")?,
                admitted: self.admitted.lock().map_err(|_| "admitted lock")?,
                submissions: self.submissions.lock().map_err(|_| "submissions lock")?,
            })
        }
    }

    fn contents(entries: &Entries) -> Vec<(RunId, u32)> {
        entries.iter().map(|(&run, &value)| (run, value)).collect()
    }

    fn expected(changed: u32, value: u32) -> Vec<(RunId, u32)> {
        (0_u32..7)
            .map(|key| {
                (
                    RunId::new(u64::from(key)),
                    if key == changed { value } else { key },
                )
            })
            .collect()
    }

    #[test]
    fn occupied_updates_preserve_every_key_order_and_unrelated_value() -> TestResult {
        let fixture = Fixture::new()?;
        let mut indexes = fixture.indexes()?;
        for key in 0_u32..7 {
            let run = RunId::new(u64::from(key));
            let _old = indexes.pending.insert(run, key);
            let _old = indexes.admitted.insert(run, key);
            let _old = indexes.submissions.insert(run, key);
        }
        indexes.update(PendingIndexUpdate {
            pending: Some((RunId::new(2), 21)),
            admitted_depth: Some((RunId::new(3), 31)),
            submission: Some((RunId::new(4), 41)),
        });
        assert_eq!(contents(&indexes.pending), expected(2, 21));
        assert_eq!(contents(&indexes.admitted), expected(3, 31));
        assert_eq!(contents(&indexes.submissions), expected(4, 41));
        Ok(())
    }

    #[test]
    fn missing_updates_append_and_none_fields_preserve_other_indexes() -> TestResult {
        let fixture = Fixture::new()?;
        let mut indexes = fixture.indexes()?;
        indexes.update(PendingIndexUpdate {
            pending: Some((RunId::new(10), 1)),
            admitted_depth: Some((RunId::new(10), 9)),
            submission: Some((RunId::new(10), 1)),
        });
        indexes.update(PendingIndexUpdate {
            pending: Some((RunId::new(20), 2)),
            admitted_depth: Some((RunId::new(20), 7)),
            submission: Some((RunId::new(20), 2)),
        });
        assert_eq!(
            contents(&indexes.pending),
            vec![(RunId::new(10), 1), (RunId::new(20), 2)]
        );
        assert_eq!(
            contents(&indexes.admitted),
            vec![(RunId::new(10), 9), (RunId::new(20), 7)]
        );
        assert_eq!(
            contents(&indexes.submissions),
            vec![(RunId::new(10), 1), (RunId::new(20), 2)]
        );
        let admitted = contents(&indexes.admitted);
        let submissions = contents(&indexes.submissions);
        indexes.update(PendingIndexUpdate {
            pending: Some((RunId::new(10), 3)),
            admitted_depth: None,
            submission: None,
        });
        assert_eq!(
            contents(&indexes.pending),
            vec![(RunId::new(10), 3), (RunId::new(20), 2)]
        );
        assert_eq!(contents(&indexes.admitted), admitted);
        assert_eq!(contents(&indexes.submissions), submissions);
        let pending = contents(&indexes.pending);
        indexes.update(PendingIndexUpdate {
            pending: None,
            admitted_depth: None,
            submission: None,
        });
        assert_eq!(contents(&indexes.pending), pending);
        assert_eq!(contents(&indexes.admitted), admitted);
        assert_eq!(contents(&indexes.submissions), submissions);
        Ok(())
    }
}
