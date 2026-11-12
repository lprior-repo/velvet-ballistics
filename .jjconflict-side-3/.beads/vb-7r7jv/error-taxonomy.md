# Railway error taxonomy

Parse boundary: UnknownPackSchema, LegacyPackMigrationRequired, PayloadTooLarge,
InvalidExecutorIdentity, InvalidTimeout, InvalidCapabilityScope.

Binding boundary: ManifestDigestMismatch, ExecutorUnavailable, ExecutorVersionMismatch,
ReceiptSchemaMismatch, ConformanceMissing, ConformanceNotExecuted,
ConformanceReportMismatch, ActionContractMismatch, ActionPackDigestMismatch.

Classification boundary: UnsupportedRecoveryClass, RecoveryClassIncompatible,
ExternalRetryEvidenceMissing. Unknown/non-compensatable external behavior defaults
to Blocked, rather than permissive retry or invented compensatability.

Execution boundary: TicketNotScheduled, TicketIdentityMismatch, GrantDenied,
ReceiptConflict, CapacityExceeded, DurableCommitFailed, ReceiptDecodeFailed.
Errors retain actual ticket/pack identities; zero or dummy identities are forbidden.
Persistence failure cannot issue a durable receipt or fabricate a completion.
