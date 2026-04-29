STATUS: APPROVED

# Black Hat QA Review: vb-65v repaired handle-based SlotValue

## Contract verdict

Approved. The four previous blockers are fixed in the current implementation. The repair stopped pretending that lengths and field counts are arena handles, locked `FiniteF64` behind checked construction/deserialization, removed compiler tests that blessed fake handles, and renamed the public hot IR branch variant to explicit slot-based choose semantics.

## Command evidence

- `/home/lewis/.cargo/bin/cargo fmt --all -- --check` — exit 0, no formatting drift.
- `/home/lewis/.cargo/bin/cargo test -p vb-core` — exit 0, 20 passed across vb-core suites/doc-tests.
- `/home/lewis/.cargo/bin/cargo test --workspace --all-targets` — exit 0, workspace/all-target tests passed, including compiler rejection tests and bench test targets.
- `/home/lewis/.cargo/bin/cargo clippy --workspace --all-targets --all-features -- -D warnings` — exit 0, no warnings promoted to errors.

## Previous blocker validation

1. `FiniteF64` is no longer publicly forgeable.
   - File: `crates/vb-core/src/value.rs:22`
   - The tuple field is private: `pub struct FiniteF64(f64);`.
   - File: `crates/vb-core/src/value.rs:27-40`
   - Construction goes through `FiniteF64::new`, which rejects non-finite values, and access goes through `get`.
   - File: `crates/vb-core/src/value.rs:52-60`
   - Serde deserialization also routes through `FiniteF64::new`; NaN/Inf cannot sneak in through decode.

2. The compiler no longer fabricates `BlobId`, `ListId`, or `ObjectId` from payload shape.
   - File: `crates/vb-compiler/src/lib.rs:1747-1762`
   - `save` lowering only permits the single explicit `value` field; arbitrary object bodies are rejected.
   - File: `crates/vb-compiler/src/lib.rs:1910-1939`
   - string, representation, sequence, and mapping constants now return `CompileError::UnsupportedConstantValue` instead of minting bogus handles from string length, list length, or object field count.

3. Tests no longer bless fake compiler handles.
   - File: `crates/vb-compiler/src/lib.rs:1953-2004`
   - The old tests that expected `ObjectId::new(field_count)` are gone. They now assert rejection for object/string/nested `save` payloads until real arenas exist.
   - Residual `ObjectId::new`, `ListId::new`, and `BlobId::new` uses in vb-core tests are direct ID roundtrip/engine-value tests, not compiler claims that payloads were preserved. That is acceptable for this blocker.

4. Public IR choose semantics are explicit.
   - File: `crates/vb-core/src/workflow.rs:146-154`
   - Public hot IR now exposes `CompiledNodeKind::ChooseSlot`, with a `condition: SlotIdx` field. The ambiguous legacy `CompiledNodeKind::Choose` variant is gone.
   - File: `crates/vb-compiler/src/lib.rs:1780-1784`
   - The compiler emits `ChooseSlot`, so the public IR and compiler agree on slot-based semantics.

## Phase findings

### Phase 1: Contract & Bead Parity

Passed. The repaired implementation matches the vb-65v acceptance target: handle-only `SlotValue` shape remains intact, fake compiler handle allocation is removed, and unsupported payload-bearing constants fail closed instead of corrupting semantics.

### Phase 2: Farley Engineering Rigor

Passed for the reviewed blockers. The compiler now makes the absence of arenas explicit by rejecting unsupported values. Tests assert the externally visible behavior: rejection instead of data loss.

### Phase 3: NASA-Level Functional Rust

Passed for the repaired scope. `FiniteF64` is parsed into a trusted type at the boundary, illegal non-finite float state is no longer constructible through the public API, and public choose IR documents the condition source as a slot.

### Phase 4: Ruthless Simplicity & DDD

Passed. The repair deletes the cute fake-identity trick and replaces it with a boring, correct refusal until real arenas exist.

### Phase 5: Bitter Truth

Passed. This is not a full arena implementation, but it no longer lies. Rejecting unsupported constants is the honest engineering move.

## Residual risk

- Real symbol/blob/list/object arenas are still absent. That is not a blocker for this re-review because the implementation now fails closed rather than fabricating handles.

BRUTAL VERDICT: APPROVED. The previous rejection blockers are actually repaired.
