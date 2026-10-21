use crate::constants::{MAGIC_INDEX_RECORD, MAX_RUN_INPUT_BYTES};
use crate::{JournalError, RunInputsRecord, codec::encode_record};

impl crate::JournalWriteBatch<'_> {
    pub(crate) fn put_run_inputs(&mut self, record: &RunInputsRecord) -> Result<(), JournalError> {
        let encoded = crate::keys::run_header_key(record.run()).and_then(|key| {
            encode_record(
                MAGIC_INDEX_RECORD,
                crate::RecordKind::RunInputs,
                record.run().get(),
                record,
                MAX_RUN_INPUT_BYTES,
            )
            .map(|value| (key, value))
        });
        match encoded {
            Ok((key, value)) => {
                self.inner.insert(&self.journal.run_inputs, key, value);
                Ok(())
            }
            Err(error) => {
                self.aborted = true;
                Err(error)
            }
        }
    }
}
