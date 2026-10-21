# vb-3ruqy consumer, default and owned-input contract draft

Status: analysis only, pending coordinator domain decisions and ownership handoff. No Rust implementation or behavior tests added. Snapshot analyzed: cd6a8e8596259c20e0159492bbca041f5de152e7 plus formatter-only child aea57884bace3519a4f3098201e57ed5c6097179. This branch does not include admission9f0557 or future semantic-source-identity field work.

Authority: master section8 requires every source reference compiled to SlotIdx/AccessorIdx before runtime. Section24 primitive contracts identify numeric input reads; section66 binds accepted inputs/artifacts. Existing closed vb-cu3ax covers final Finish only. This draft does not broaden that closure or authorize edits to part_01 during the CI owner's concurrent repair.

I1. Declare and bind every input before lowering its first consumer. Preserve declaration order for positional transport. Allocate a unique input port per declaration, disjoint from every compiler-generated/internal write slot. A layout reservation pass must include compound-body scratch slots and explicit numeric slot compatibility; moving current allocation to the beginning alone can collide with step-index-derived output slots. Names exist only in the cold compiler environment; emitted consumer operands are numeric.

I2. One cold reference-resolution contract must serve each supported input-consuming field, including nested bodies. Finish output, for_each input, collect source, aggregate/reduce input, wait event/deadline/timeout, ask prompt/timeout and choose conditions must resolve through their appropriate value/expression semantics. A slot-valued operand and an expression operand are different domain cases; do not replace expression lowering with string prefix substitution. Set currently accepts integer constants only; input-derived Set values need explicit primitive/expression lowering scope. Unsupported consumers fail before acceptance with a typed source diagnostic rather than falling back to a slot number or Null. Unknown declaration, wrong kind, unsupported accessor and slot exhaustion remain distinct failures.

I3. Coordinator decision: no input-default behavior is added without a normative language contract. Reject unsupported longform schema keys, including default, with an exact typed diagnostic; never silently ignore them. Omission, explicit Null and an empty collection are distinct values. A schema declaration is not a runtime value. Any supported default must be type-checked, bounded, compiled into numeric owned initialization data, included in artifact identity, and available after source removal. The authoritative master text inspected does not specify input-default syntax, requiredness, or precedence. Keep current omitted/prefix transport semantics explicit; do not invent requiredness or precedence. Any future default feature requires a separate approved language change.

I4. Public owned inputs carry bounded payloads, not unbacked arena IDs. Proposed owned scalar/string/list/object/blob values must validate recursively or via a bounded flat representation, then materialize into the destination run's ValueStore before any consumer. Every nested handle must resolve in that same store and retain its declared taint; symbols/object keys also require payload backing. No forged external ListId/ObjectId/BlobId/SymbolId becomes trusted merely because its numeric value is small. The existing handle-only CLI wire remains compatible for genuinely usable values; unsupported owned values reject until an explicitly versioned payload exists.

I5. Resolve and validate the complete effective input mapping before mutating a live frame or publishing an accepted run. Source/default identity and actual effective values are separate digest bindings. Persist effective typed input payloads and their receipt digest atomically with the accepted numeric artifact in durable submission. Recovery loads artifact plus retained values/backing, verifies all bindings, materializes the same values, then drives numeric IR without YAML compilation. Snapshot-restored values must not be overwritten by replaying defaults or original inputs after execution has begun.

I6. Resource limits cover declarations, all reserved slots, nested input depth, total owned entries/bytes and materialization cost. Use checked sizing and typed budget errors. No unsafe, unchecked arithmetic/indexing, panic fallback or runtime source-name lookup. No speed claim is made. Exact wire version/API changes require coordinated compiler/core/transport/storage/runtime owners and old-byte fixtures; serde default/skip is not an assumed positional postcard migration.

Current observations (static evidence, not new execution claims):

| Boundary | Snapshot behavior | Consequence |
| --- | --- | --- |
| compile_source part_01 + finish_inputs | Input ports bind immediately before final Finish, otherwise allocate after steps | Earlier consumers do not share named bindings |
| lower_canonical_set | Parses integer string and emits SetConst to step-index slot | Moving input allocation alone risks overlap and cannot enable expression input use |
| slot_from_text + primitive callers | Parses i64 then checked u16 | $input.name fails for listed non-Finish slot operands |
| kind_from_input_schema | Reads shorthand or mapping is; other mapping entries are not processed | Default metadata has no compiled initialization path |
| CompiledInputSlot | slot + kind only | No requiredness/default/arena backing in this IR type |
| map_runtime_inputs | Exact postcard decoding, positional prefix accepted, empty bytes yield no values | Omission is currently permitted; no default application |
| seed_input_slots | Checks kind only if declaration found, then writes clean SlotValue | This helper alone does not establish declared-only ownership or arena validity |
| runtime lifecycle chunk_001 | Seeds frame, later creates fresh bounded ValueStore | Bare submitted handles have no imported payload backing |

Open domain decisions, owned by coordination rather than silently chosen here:

D1 resolved by coordination: implement actual master-declared references; reject unsupported longform schema keys including default. Do not invent default precedence or requiredness. Any future default policy is separate language work.
D2. Define owned input wire/version and taint authority. A bounded flat owned-value graph avoids unbounded recursive public input traversal. Determine limits and whether legacy valid symbols may reference preloaded workflow symbols or must carry text.
D3. Coordinate numeric slot layout/resolver hunks with CI part_01 and compiler digest owner. The direct WorkflowParts semantic identity field migration is approved and owned by compiler CI; this lane must not edit that digest hunk. Include reserved scratch slots for each compound primitive and expression consumer scope. Source identity/evidence-v3 coordination is separate work, not implemented here.

Required acceptance observations for later implementation: real public compiler lowering must expose numeric operands for each consumer; supplied/default/omitted cases must produce exact values and taints through production execution; owned nested values must survive persistence/reopen/materialization; malformed refs, wrong kinds, forged handles and resource exhaustion must reject without partial admission. These are domain acceptance conditions, not a test/proof plan or evidence of completed behavior.
