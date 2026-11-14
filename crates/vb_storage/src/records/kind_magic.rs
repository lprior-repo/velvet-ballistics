#![forbid(unsafe_code)]
//! Record-kind classification against the record-family magic.

use super::kind::RecordKind;

impl RecordKind {
    /// Returns true when this kind is valid for the supplied record-family magic.
    #[must_use]
    pub const fn belongs_to_magic(self, magic: u32) -> bool {
        match magic {
            crate::constants::MAGIC_WORKFLOW_SOURCE => matches!(self, Self::WorkflowSource),
            crate::constants::MAGIC_COMPILED_ARTIFACT => matches!(self, Self::CompiledIr),
            crate::constants::MAGIC_JOURNAL_EVENT => self.is_journal_event(),
            crate::constants::MAGIC_SNAPSHOT => matches!(self, Self::Snapshot),
            crate::constants::MAGIC_BLOB => matches!(self, Self::Blob),
            crate::constants::MAGIC_INDEX_RECORD => {
                matches!(
                    self,
                    Self::RunHeader | Self::RunInputs | Self::RunOwnedInputs | Self::IndexUpdate
                )
            }
            _ => false,
        }
    }
}
