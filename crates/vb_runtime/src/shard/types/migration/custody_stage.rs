// Bounded queue-owned staging with retained refusal custody.
use std::collections::VecDeque;

/// Hands one staged entry to its destination; `Err` returns the unaccepted entry.
///
/// Concrete queue destinations implement this directly so the refusal value is
/// not returned from a closure with a large `Err` type.
pub(crate) trait Accept<T> {
    fn accept(self, entry: T) -> Result<(), T>;
}

impl<T, F: FnOnce(T) -> Result<(), T>> Accept<T> for F {
    fn accept(self, entry: T) -> Result<(), T> {
        self(entry)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum StageCreateError {
    InvalidCapacity,
    Allocation,
}

// The owning queue retains this object through every refusal.
// All allocation is in construction, before the first command is removed.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TransferProgress {
    Empty,
    Accepted,
    Refused,
}

pub(crate) struct MigrationCustody<T> {
    entries: VecDeque<T>,
    limit: usize,
    in_flight: Option<T>,
}

impl<T> MigrationCustody<T> {
    pub(crate) fn try_new(limit: usize, maximum: usize) -> Result<Self, StageCreateError> {
        if limit == 0 || limit > maximum {
            return Err(StageCreateError::InvalidCapacity);
        }
        let mut entries = VecDeque::new();
        entries
            .try_reserve_exact(limit)
            .map_err(|_| StageCreateError::Allocation)?;
        Ok(Self {
            entries,
            limit,
            in_flight: None,
        })
    }

    pub(crate) fn fill_one(&mut self, take: impl FnOnce() -> Option<T>) -> TransferProgress {
        if self.in_flight.is_some() || self.entries.len() >= self.limit {
            return TransferProgress::Refused;
        }
        let Some(entry) = take() else {
            return TransferProgress::Empty;
        };
        self.entries.push_back(entry);
        TransferProgress::Accepted
    }

    #[cfg(test)]
    pub(crate) fn push_back(&mut self, entry: T) -> Result<(), T> {
        if self.in_flight.is_some() || self.entries.len() >= self.limit {
            return Err(entry);
        }
        self.entries.push_back(entry);
        Ok(())
    }

    #[cfg(test)]
    fn restore_front(&mut self, entry: T) -> Result<(), T> {
        if self.in_flight.is_some() || self.entries.len() >= self.limit {
            return Err(entry);
        }
        self.entries.push_front(entry);
        Ok(())
    }

    // The callback returns the unaccepted value as `Err`, e.g. ArrayQueue::push(...).
    // No other stage insertion can occur while this exclusive borrow is held.
    pub(crate) fn transfer_front(&mut self, accept: impl Accept<T>) -> TransferProgress {
        if self.in_flight.is_none() {
            self.in_flight = self.entries.pop_front();
        }
        let Some(entry) = self.in_flight.take() else {
            return TransferProgress::Empty;
        };
        match accept.accept(entry) {
            Ok(()) => TransferProgress::Accepted,
            Err(entry) => {
                self.in_flight = Some(entry);
                TransferProgress::Refused
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn in_flight(&self) -> Option<&T> {
        self.in_flight.as_ref()
    }

    #[cfg(test)]
    fn pop_front(&mut self) -> Option<T> {
        self.in_flight.take().or_else(|| self.entries.pop_front())
    }

    pub(crate) fn entries(&self) -> impl Iterator<Item = &T> {
        self.in_flight.iter().chain(self.entries.iter())
    }

    pub(crate) fn len(&self) -> usize {
        self.entries
            .len()
            .saturating_add(usize::from(self.in_flight.is_some()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_destination_push_can_restore_original_front() -> Result<(), StageCreateError> {
        let mut stage = MigrationCustody::try_new(2, 2)?;
        assert_eq!(stage.push_back(11), Ok(()));
        assert_eq!(stage.push_back(22), Ok(()));
        let refused = stage.pop_front();
        assert_eq!(refused, Some(11));
        if let Some(entry) = refused {
            assert_eq!(stage.restore_front(entry), Ok(()));
        }
        assert_eq!(stage.entries().copied().collect::<Vec<_>>(), vec![11, 22]);
        assert_eq!(stage.len(), 2);
        Ok(())
    }

    #[test]
    fn stage_full_returns_ownership_without_changing_contents() -> Result<(), StageCreateError> {
        let mut stage = MigrationCustody::try_new(1, 1)?;
        assert_eq!(stage.push_back(String::from("first")), Ok(()));
        let refused = stage.push_back(String::from("still owned"));
        assert_eq!(refused, Err(String::from("still owned")));
        assert_eq!(stage.pop_front(), Some(String::from("first")));
        assert_eq!(stage.pop_front(), None);
        Ok(())
    }

    #[test]
    fn refused_transfer_retains_front_until_accepted() -> Result<(), StageCreateError> {
        struct Item(u32);
        let mut stage = MigrationCustody::try_new(2, 2)?;
        assert!(stage.push_back(Item(11)).is_ok());
        assert!(stage.push_back(Item(22)).is_ok());
        assert_eq!(stage.transfer_front(Err), TransferProgress::Refused);
        assert_eq!(stage.in_flight().map(|item| item.0), Some(11));
        assert_eq!(stage.len(), 2);
        assert_eq!(stage.transfer_front(Err), TransferProgress::Refused);
        assert_eq!(
            stage.entries().map(|item| item.0).collect::<Vec<_>>(),
            vec![11, 22]
        );
        let mut accepted = None;
        assert_eq!(
            stage.transfer_front(|item| {
                accepted = Some(item);
                Ok(())
            }),
            TransferProgress::Accepted
        );
        assert_eq!(accepted.map(|item| item.0), Some(11));
        assert_eq!(stage.pop_front().map(|item| item.0), Some(22));
        Ok(())
    }
}

#[cfg(test)]
include!("custody_supplemental_tests.rs");
