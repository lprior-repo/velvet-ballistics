# Proposed type contracts

Use existing SlotIdx, InputSlotKind, WorkflowDigest and RunId at numeric boundaries. A cold InputName smart constructor enforces the master identifier grammar; it never enters the engine frame.

Proposed states: UnboundDeclarations -> BoundInputPorts -> LoweredArtifact; UntrustedInputPayload -> ValidatedOwnedInputs -> EffectiveInputs -> MaterializedRunInputs -> AcceptedRun. Private constructors carry invariants; only a completely materialized mapping may seed a runnable frame. Storage preparation may retain owned initialization bytes before creating a live run, but validation must finish before acceptance.

Represent InputSupply as Omitted | Supplied(OwnedInputValue), preserving explicit Null inside OwnedInputValue. Represent a declared default as Absent | Present(ValidatedOwnedValue); future default/requiredness types are illustrative only: coordinator D1 currently requires unsupported default keys to reject. Resource sizes use checked bounded newtypes. An owned flat graph uses validated indices/acyclicity and bounded depth; runtime SlotValue handles are produced only by materialization.

A FrozenSlotLayout must prove all declared input slots are unique, in-range and disjoint from every writable output/scratch slot unless an explicitly modeled overwrite is part of the language. A BoundReference resolves to a numeric slot/accessor or a compiled expression, preserving the field's semantic type. No implicit fallback from a failed source reference to numeric parsing.

Current public seed_input_slots accepts raw pairs and does not provide these ownership/declaration guarantees by itself. Existing CompiledInputSlot has no default/backing fields. These representable illegal states must be closed in coordinated implementation; this analysis does not claim otherwise.
