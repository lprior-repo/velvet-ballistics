# vb-eirmk.60 exhaustive mapping decision

The user explicitly selected exhaustive enforcement of MASTER65 Idempotency implications. This addendum replaces the design report's unresolved mapping question with a blocking decision; all other design and verification limits remain.

MASTER65:3157 permits IdempotentExternal only with ExternalRead or ExternalWrite. The historical registered executor requires IdempotentExternal + Writes + KeyRequired (crates/vb_storage/src/executor/catalog.rs:201-216). Its concrete implementation opens and writes local Fjall keyspaces (register.rs:31-39,168-199). Therefore the proposed canonical LocalWrite projection with unchanged IdempotentExternal is not permitted under the accepted exhaustive policy.

Do not relabel this executor ExternalWrite to evade the implication, infer a different Idempotency, or modify retained contract bytes, ABI, manifest, pack version or receipt identity. Its canonical projection and new canonical execution route remain blocked pending a separately authorized contract/design resolution. A historical identity verification/reconciliation lane does not itself resolve canonical classification or authorize canonical execution.

The private cold identity boundary remains a viable mechanical design for retaining bytes and existing identities. The native alias probes and source review do not qualify an accepted canonical runtime projection. Generic Writes and distinct Sends/Creates/Destroys records remain cold and unresolved unless an authoritative action-specific classification satisfies the exhaustive implications.

The bounded current-format Unsafe retry refusal is assessed separately. It does not require canonical enum or format changes and cannot close canonical vb-eirmk.60. Parent retains tracker ownership; vb-eirmk.60 remains IN_PROGRESS and open. No production edits or bead status changes were made for this addendum.
