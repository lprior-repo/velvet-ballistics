// Bounded rollback state machine. Requires target FIFO staged first.
// The owner retains both custody buffers and this state across refusal.
use super::custody_stage::{Accept, MigrationCustody, TransferProgress};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RollbackPhase {
    TargetPrefix,
    SourcePrefix,
    SourceTail,
    Complete,
}

pub(crate) struct MigrationRollback {
    phase: RollbackPhase,
    target_original_remaining: usize,
}

impl MigrationRollback {
    pub(crate) fn prepare<T>(
        target: &MigrationCustody<T>,
        original_target_len: usize,
    ) -> Option<Self> {
        if original_target_len > target.len() {
            return None;
        }
        Some(Self {
            phase: RollbackPhase::TargetPrefix,
            target_original_remaining: original_target_len,
        })
    }

    // Each invocation transfers at most one envelope; refusal retains it.
    // Without refusal, completion needs original combined command count + 3 calls.
    // Refusal consumes a call; retain this state and both buffers for later work.
    pub(crate) fn step<T>(
        &mut self,
        source: &mut MigrationCustody<T>,
        target: &mut MigrationCustody<T>,
        push_source: impl Accept<T>,
        push_target: impl Accept<T>,
    ) -> TransferProgress {
        match self.phase {
            RollbackPhase::TargetPrefix => self.restore_target_prefix(target, push_target),
            RollbackPhase::SourcePrefix => {
                let progress = target.transfer_front(push_source);
                if progress == TransferProgress::Empty {
                    self.phase = RollbackPhase::SourceTail;
                }
                progress
            }
            RollbackPhase::SourceTail => {
                let progress = source.transfer_front(push_source);
                if progress == TransferProgress::Empty {
                    self.phase = RollbackPhase::Complete;
                }
                progress
            }
            RollbackPhase::Complete => TransferProgress::Empty,
        }
    }

    fn restore_target_prefix<T>(
        &mut self,
        target: &mut MigrationCustody<T>,
        push_target: impl Accept<T>,
    ) -> TransferProgress {
        let Some(remaining) = self.target_original_remaining.checked_sub(1) else {
            self.phase = RollbackPhase::SourcePrefix;
            return TransferProgress::Empty;
        };
        let progress = target.transfer_front(push_target);
        if progress == TransferProgress::Accepted {
            self.target_original_remaining = remaining;
        }
        progress
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.phase == RollbackPhase::Complete
    }
}

#[cfg(test)]
mod tests {
    use super::super::custody_stage::StageCreateError;
    use super::*;

    #[test]
    fn rollback_restores_both_original_fifos_after_refusal() -> Result<(), StageCreateError> {
        let mut source = MigrationCustody::try_new(2, 2)?;
        let mut target = MigrationCustody::try_new(2, 2)?;
        assert_eq!(source.push_back(22), Ok(()));
        assert_eq!(target.push_back(10), Ok(()));
        assert_eq!(target.push_back(11), Ok(()));
        let mut rollback =
            MigrationRollback::prepare(&target, 1).ok_or(StageCreateError::InvalidCapacity)?;
        let mut restored_source = Vec::new();
        let mut restored_target = Vec::new();
        assert_eq!(
            rollback.step(&mut source, &mut target, Err, Err,),
            TransferProgress::Refused
        );
        assert_eq!(target.len(), 2);
        for _ in 0..6 {
            rollback.step(
                &mut source,
                &mut target,
                |item| {
                    restored_source.push(item);
                    Ok(())
                },
                |item| {
                    restored_target.push(item);
                    Ok(())
                },
            );
        }
        assert!(rollback.is_complete());
        assert_eq!(restored_source, vec![11, 22]);
        assert_eq!(restored_target, vec![10]);
        assert_eq!(source.len(), 0);
        assert_eq!(target.len(), 0);
        Ok(())
    }

    #[test]
    fn refusal_in_source_prefix_and_tail_retains_custody_and_retries_in_order()
    -> Result<(), StageCreateError> {
        let mut source = MigrationCustody::try_new(2, 2)?;
        let mut target = MigrationCustody::try_new(2, 2)?;
        assert_eq!(source.push_back(22), Ok(()));
        assert_eq!(target.push_back(10), Ok(()));
        assert_eq!(target.push_back(11), Ok(()));
        let mut rollback =
            MigrationRollback::prepare(&target, 1).ok_or(StageCreateError::InvalidCapacity)?;
        let mut restored_source = Vec::new();
        let mut restored_target = Vec::new();
        macro_rules! step_ok {
            () => {
                rollback.step(
                    &mut source,
                    &mut target,
                    |item| {
                        restored_source.push(item);
                        Ok(())
                    },
                    |item| {
                        restored_target.push(item);
                        Ok(())
                    },
                )
            };
        }
        assert_eq!(step_ok!(), TransferProgress::Accepted);
        assert_eq!(step_ok!(), TransferProgress::Empty);
        assert_eq!(
            rollback.step(&mut source, &mut target, Err, Err),
            TransferProgress::Refused
        );
        assert_eq!(target.len(), 1);
        assert!(restored_source.is_empty());
        assert_eq!(step_ok!(), TransferProgress::Accepted);
        assert_eq!(step_ok!(), TransferProgress::Empty);
        assert_eq!(
            rollback.step(&mut source, &mut target, Err, Err),
            TransferProgress::Refused
        );
        assert_eq!(source.len(), 1);
        assert!(!rollback.is_complete());
        assert_eq!(step_ok!(), TransferProgress::Accepted);
        assert_eq!(step_ok!(), TransferProgress::Empty);
        assert!(rollback.is_complete());
        assert_eq!(restored_target, vec![10]);
        assert_eq!(restored_source, vec![11, 22]);
        Ok(())
    }

    #[test]
    fn invalid_prefix_length_leaves_custody_unchanged() -> Result<(), StageCreateError> {
        let mut target = MigrationCustody::try_new(1, 1)?;
        assert_eq!(target.push_back(10), Ok(()));
        assert!(MigrationRollback::prepare(&target, 2).is_none());
        assert_eq!(target.entries().copied().collect::<Vec<_>>(), vec![10]);
        Ok(())
    }
}
