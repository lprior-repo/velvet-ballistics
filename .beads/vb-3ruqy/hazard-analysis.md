# Hazards

- Slot aliasing: allocating inputs before steps without reserving all outputs/scratch can overwrite an input; FrozenSlotLayout must establish disjointness (Rust invariant, bounded state).
- Incomplete binding: special-casing Finish leaves earlier/nested consumers with numeric parse errors or wrong operands; one explicit cold resolution contract must cover supported consumers (behavior).
- Default ambiguity: omitted, Null and supplied empty values can collapse; represent supply/default states separately and settle D1 before implementation (domain/API).
- Dangling/cross-run handles: a kind-correct ListId or SymbolId is not owned payload; validate/materialize all nested backing into the destination store (hostile input/ownership).
- Partial materialization/admission: mutating a live frame before full validation can expose partial state; prepare effective values before publication and preserve atomic storage (temporal).
- Recovery overwrite: reapplying defaults after snapshot hydration can erase executed values; distinguish initial acceptance materialization from restored execution state (temporal/storage).
- Codec drift: new public fields break positional postcard and hundreds of struct literals; coordinate versioning and fixture migration, reject unknown schemas (API/release).
- Unbounded payloads: nested owned collections need aggregate byte/entry/depth caps, checked sizing and fallible allocation (hostile input/resources).

No new concurrency or unsafe implementation is proposed. Existing storage atomicity and deterministic engine semantics remain external trusted boundaries, not newly proven properties. No latency or throughput claim is made.
