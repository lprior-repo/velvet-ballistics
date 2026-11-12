# Typed contract
Checked compiler results have private fields and read-only accessors. No public
constructor can mint a passed count, durable flag, or complete proof from a digest.
Digest roles remain separate: source spec bytes, canonical self-zeroed workflow,
exact serialized IR payload, serialized action contracts, serialized resource policy.
Evidence uses semantic obligation and closure enums; unperformed obligations must
be represented as unresolved or external assumptions, never RuntimeValidated.
Serialized evidence remains hostile and requires independent consumer validation.
