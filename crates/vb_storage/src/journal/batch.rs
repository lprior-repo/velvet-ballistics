use crate::{
    batch::JournalWriteBatch,
    journal::FjallJournal,
};

impl FjallJournal {
    /// Creates a new atomic cross-keyspace write batch.
    pub fn batch(&self) -> JournalWriteBatch<'_> {
        JournalWriteBatch::new(self)
    }
}
