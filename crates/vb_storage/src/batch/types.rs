#![forbid(unsafe_code)]
use std::collections::HashSet;
use crate::constants::JOURNAL_KEY_BYTES;
use crate::journal::FjallJournal;
pub const DEFAULT_JOURNAL_BATCH_BYTE_LIMIT: u64 = 1_048_576;
pub struct JournalWriteBatch<'j> {
    pub(super) inner: fjall::OwnedWriteBatch,
    pub(super) journal: &'j FjallJournal,
    #[allow(dead_code)]
    pub(super) staged_event_keys: HashSet<[u8; JOURNAL_KEY_BYTES]>,
    pub(super) aborted: bool,
    pub(super) staged_bytes: u64,
    pub(super) byte_limit: Option<u64>,
    pub(super) _not_send_or_sync: core::marker::PhantomData<*mut FjallJournal>,
}
impl<'j> JournalWriteBatch<'j> {
    pub fn new(journal: &'j FjallJournal) -> Self {
        Self {
            inner: journal.database.batch(),
            journal,
            staged_event_keys: HashSet::new(),
            aborted: false,
            staged_bytes: 0,
            byte_limit: Some(DEFAULT_JOURNAL_BATCH_BYTE_LIMIT),
            _not_send_or_sync: core::marker::PhantomData,
        }
    }
    #[must_use]
    pub fn len(&self) -> usize { if self.aborted { 0 } else { self.inner.len() } }
    #[must_use]
    pub fn is_empty(&self) -> bool { self.len() == 0 }
    #[must_use]
    pub fn is_aborted(&self) -> bool { self.aborted }
    #[must_use]
    pub fn staged_event_bytes(&self) -> u64 { self.staged_bytes }
    #[must_use]
    pub fn byte_limit(&self) -> Option<u64> { self.byte_limit }
}
