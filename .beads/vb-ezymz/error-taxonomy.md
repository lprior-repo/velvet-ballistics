# error-taxonomy

Existing InputMappingError owns CLI decode/count/kind failures. Existing JournalError InputSchemaMismatch handles retained mapping inconsistency; InvalidRunId rejects zero; RunAlreadyExists rejects collision; codec/digest errors preserve corruption; Fjall and BatchAborted preserve storage failure. No string domain errors are introduced.

See contract.md C1-C7.
