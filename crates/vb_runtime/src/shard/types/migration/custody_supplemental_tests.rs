// Append as a sibling of the existing tests module in custody_stage.rs.
#[cfg(test)]
mod custody_supplemental_tests {
    use super::{MigrationCustody, StageCreateError, TransferProgress};
    use std::cell::Cell;
    use std::rc::Rc;

    struct DropCounter {
        identity: Box<u32>,
        drops: Rc<Cell<usize>>,
    }

    impl DropCounter {
        fn new(id: u32, drops: &Rc<Cell<usize>>) -> Self {
            Self {
                identity: Box::new(id),
                drops: Rc::clone(drops),
            }
        }

        fn identity(&self) -> *const u32 {
            self.identity.as_ref() as *const u32
        }
    }

    impl Drop for DropCounter {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }

    #[test]
    fn invalid_capacity_is_rejected_before_staging() {
        for (limit, maximum) in [(0, 0), (0, 1), (2, 1), (1, 0)] {
            assert!(matches!(
                MigrationCustody::<u32>::try_new(limit, maximum),
                Err(StageCreateError::InvalidCapacity)
            ));
        }
    }

    #[test]
    fn full_restore_returns_same_owned_value() -> Result<(), StageCreateError> {
        let first_drops = Rc::new(Cell::new(0));
        let second_drops = Rc::new(Cell::new(0));
        let mut stage = MigrationCustody::try_new(1, 1)?;
        let first = DropCounter::new(11, &first_drops);
        let first_identity = first.identity();
        assert!(stage.push_back(first).is_ok());
        let second = DropCounter::new(22, &second_drops);
        let second_identity = second.identity();
        let returned = stage.restore_front(second).err();
        assert_eq!(
            returned.as_ref().map(DropCounter::identity),
            Some(second_identity)
        );
        assert_eq!(
            stage
                .entries()
                .map(DropCounter::identity)
                .collect::<Vec<_>>(),
            vec![first_identity]
        );
        assert_eq!(stage.len(), 1);
        assert_eq!(first_drops.get(), 0);
        assert_eq!(second_drops.get(), 0);
        drop(returned);
        assert_eq!(second_drops.get(), 1);
        assert_eq!(first_drops.get(), 0);
        drop(stage);
        assert_eq!(first_drops.get(), 1);
        Ok(())
    }

    #[test]
    fn repeated_refusal_preserves_identity_and_drops_only_on_owner_release()
    -> Result<(), StageCreateError> {
        let first_drops = Rc::new(Cell::new(0));
        let second_drops = Rc::new(Cell::new(0));
        let extra_drops = Rc::new(Cell::new(0));
        let first = DropCounter::new(11, &first_drops);
        let second = DropCounter::new(22, &second_drops);
        let first_identity = first.identity();
        let second_identity = second.identity();
        let mut stage = MigrationCustody::try_new(3, 3)?;
        assert!(stage.push_back(first).is_ok());
        assert!(stage.push_back(second).is_ok());
        for _ in 0..3 {
            assert_eq!(stage.transfer_front(|entry| Some(entry)), TransferProgress::Refused);
            assert_eq!(
                stage.in_flight().map(DropCounter::identity),
                Some(first_identity)
            );
            assert_eq!(
                stage
                    .entries()
                    .map(DropCounter::identity)
                    .collect::<Vec<_>>(),
                vec![first_identity, second_identity]
            );
            assert_eq!(stage.len(), 2);
            assert_eq!(first_drops.get(), 0);
            assert_eq!(second_drops.get(), 0);
        }
        // Spare capacity does not permit insertion while a front item is refused.
        let extra = DropCounter::new(33, &extra_drops);
        let extra_identity = extra.identity();
        let returned = stage.push_back(extra).err();
        assert_eq!(
            returned.as_ref().map(DropCounter::identity),
            Some(extra_identity)
        );
        let returned = returned.and_then(|item| stage.restore_front(item).err());
        assert_eq!(
            returned.as_ref().map(DropCounter::identity),
            Some(extra_identity)
        );
        assert_eq!(extra_drops.get(), 0);
        drop(returned);
        assert_eq!(extra_drops.get(), 1);
        let mut accepted = None;
        assert_eq!(
            stage.transfer_front(|item| {
                accepted = Some(item);
                None
            }),
            TransferProgress::Accepted
        );
        assert_eq!(
            accepted.as_ref().map(DropCounter::identity),
            Some(first_identity)
        );
        assert!(stage.in_flight().is_none());
        assert_eq!(stage.len(), 1);
        assert_eq!(
            stage
                .entries()
                .map(DropCounter::identity)
                .collect::<Vec<_>>(),
            vec![second_identity]
        );
        assert_eq!(first_drops.get(), 0);
        assert_eq!(second_drops.get(), 0);
        drop(accepted);
        assert_eq!(first_drops.get(), 1);
        let second = stage.pop_front();
        assert_eq!(
            second.as_ref().map(DropCounter::identity),
            Some(second_identity)
        );
        assert_eq!(stage.len(), 0);
        let called = Cell::new(false);
        assert_eq!(
            stage.transfer_front(|item| {
                called.set(true);
                Some(item)
            }),
            TransferProgress::Empty
        );
        assert!(!called.get());
        drop(stage);
        assert_eq!(second_drops.get(), 0);
        drop(second);
        assert_eq!(second_drops.get(), 1);
        Ok(())
    }
}
