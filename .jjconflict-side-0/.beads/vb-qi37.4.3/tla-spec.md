# TLA+ Temporal Model Plan

## Boundary
- Model states: `Requested`, `Admitted`, `HeaderPersisted`, `Acknowledged`, `Active`, `Rejected`.
- Actions: `Submit`, `AdmissionAccept`, `AdmissionReject`, `PersistHeader`, `Ack`, `InsertActive`, `StorageFail`.
- Safety: `Ack => HeaderPersisted`; `Active => HeaderPersisted`; `StorageFail before PersistHeader => not Ack`.
- Liveness: accepted submissions eventually acknowledge or return a typed error under fair storage append.
- Deadlock freedom: every nonterminal submission has an enabled persist, reject, or fail action.
- Evidence command: `moon run :verify-proof` once TLA model exists; current obligation is to add/execute scoped model or approved waiver.
