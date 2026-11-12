# TLA+ Temporal Model Plan

TLA+ applies only to the temporal classification boundary: admission failure occurs before acknowledgement and must terminate as `Rejected(code)` or `StorageError(code)` without later `Ack`. Evidence command: `moon run :verify-proof` if the shared admission-header model is added; otherwise formal verifier must record a scoped waiver with typed-error tests as compensating evidence.
