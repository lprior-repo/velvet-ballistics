mod dequeue_count_replacement_tests {
    use super::*;
    use std::sync::Mutex;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    struct Fixture {
        pending: Mutex<IndexMap<RunId, u32>>,
        admitted: Mutex<IndexMap<RunId, u32>>,
        submissions: Mutex<IndexMap<RunId, u32>>,
    }

    impl Fixture {
        fn new() -> RuntimeResult<Self> {
            Ok(Self {
                pending: Mutex::new(migration_index_buffer(3)?),
                admitted: Mutex::new(migration_index_buffer(3)?),
                submissions: Mutex::new(migration_index_buffer(3)?),
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

    fn populate(entries: &mut IndexMap<RunId, u32>, count: Option<u32>) {
        let _previous = entries.insert(RunId::new(10), 8);
        if let Some(value) = count {
            let _previous = entries.insert(RunId::new(20), value);
        }
        let _previous = entries.insert(RunId::new(30), 9);
    }

    fn expected(count: Option<u32>) -> Vec<(RunId, u32)> {
        let mut entries = vec![(RunId::new(10), 8)];
        if let Some(value) = count.filter(|count| *count != 1) {
            entries.push((RunId::new(20), value.saturating_sub(1)));
        }
        entries.push((RunId::new(30), 9));
        entries
    }

    fn contents(entries: &IndexMap<RunId, u32>) -> Vec<(RunId, u32)> {
        entries.iter().map(|(&run, &count)| (run, count)).collect()
    }

    #[test]
    fn pending_decrement_preserves_missing_zero_last_and_large_counts() -> TestResult {
        for count in [None, Some(0), Some(1), Some(2), Some(u32::MAX)] {
            let fixture = Fixture::new()?;
            let mut indexes = fixture.indexes()?;
            populate(&mut indexes.pending, count);
            populate(&mut indexes.admitted, Some(12));
            populate(&mut indexes.submissions, Some(7));
            let admitted = contents(&indexes.admitted);
            let submissions = contents(&indexes.submissions);
            indexes.remove_pending(RunId::new(20));
            assert_eq!(contents(&indexes.pending), expected(count));
            if count == Some(1) {
                assert_eq!(
                    contents(&indexes.admitted),
                    vec![(RunId::new(10), 8), (RunId::new(30), 9)]
                );
            } else {
                assert_eq!(contents(&indexes.admitted), admitted);
            }
            assert_eq!(contents(&indexes.submissions), submissions);
        }
        Ok(())
    }

    #[test]
    fn submission_decrement_preserves_other_indexes_and_order() -> TestResult {
        for count in [None, Some(0), Some(1), Some(2), Some(u32::MAX)] {
            let fixture = Fixture::new()?;
            let mut indexes = fixture.indexes()?;
            populate(&mut indexes.pending, Some(11));
            populate(&mut indexes.admitted, Some(12));
            populate(&mut indexes.submissions, count);
            let pending = contents(&indexes.pending);
            let admitted = contents(&indexes.admitted);
            indexes.remove_submission(RunId::new(20));
            assert_eq!(contents(&indexes.submissions), expected(count));
            assert_eq!(contents(&indexes.pending), pending);
            assert_eq!(contents(&indexes.admitted), admitted);
        }
        Ok(())
    }
}
