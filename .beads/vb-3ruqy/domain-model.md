# Domain model

InputDeclaration: cold source name, schema and possible future default policy. InputPort: immutable numeric slot and runtime kind. CompilerBindingEnvironment: cold symbol-to-port relation, frozen before lowering. SlotLayout: disjoint partition of input ports, emitted outputs and bounded primitive scratch slots.

SuppliedInput: explicit owned value from a boundary. OmittedInput: absence, not Null. InputDefault: optional declared fallback value, not the schema literal. EffectiveInputs: immutable resolved mapping whose values and ownership are valid before acceptance. RunValueStore: sole owner of materialized string/list/object/blob payloads for that run. InputReceipt: binds effective values to a specific accepted run/artifact.

The effective mapping is an aggregate: it cannot contain duplicate/undeclared slots or handles from another store. Defaults are unsupported and must reject under coordinator D1; owned transport policy remains undecided (D2), so no accepted concrete public API is claimed.
