# vb-6jn77 compiled digest contract

## Domain

- `WorkflowSource` is normalized authoring semantics. YAML comments and presentation formatting are not identity.
- `semantic_source_identity` identifies normalized source semantics that lowering omits, using the compiler's canonical source digest.
- `compiled_digest` identifies the complete serialized compiled artifact payload, with only its self-digest field set to zero while hashing.
- Raw source-spec provenance, when present in an artifact envelope, remains a separate digest of the exact source bytes.
- `None` semantic identity represents compiled-IR-only or explicitly legacy data; it is not evidence that source-backed compilation was performed.

## Invariants

1. Source compilation carries the canonical semantic source identity into the serialized compiled payload.
2. Source-backed artifact verification requires a present semantic identity and checks that it agrees with the normalized source.
3. The compiler digest is recomputed from the complete serialized payload. It is never replaced by the semantic source digest.
4. Equivalent source semantics compile to the same identity despite comments or formatting changes.
5. Workflow version, trigger discriminator/parameters, and nested Repeat Set output identifiers affect the compiled digest.
6. Legacy payload decoding is explicit. Evidence schema 3 rejects schema-2 artifacts as `RequiresMigration` before decoding old `WorkflowParts`; missing identity cannot silently authorize a source-backed artifact.
7. Existing equality assertions remain strict. Tests may not be ignored or weakened to accept digest collisions.

## Representation and wire migration

Append `semantic_source_identity: Option<WorkflowDigest>` to serialized `WorkflowParts`; populate it for source compilation from `canonical_digest(WorkflowSource)`. Keep artifact-envelope raw source provenance separate. Every Rust struct literal must state `Some` or `None` explicitly. Postcard's positional wire format will not use a serde EOF/default guess: accepted-artifact evidence moves from schema 2 to schema 3, and old schema-2 artifacts return `RequiresMigration` before decoding their old WorkflowParts bytes. The artifact owner will retain a real schema-2 wire fixture and verify schema-3 roundtrip plus canonical digest recomputation. Source-backed checked artifacts require a source mapping proven by the compiler; `Some` supplied to an IR-only verifier does not establish source provenance.

## Focused proof obligations

- Recompile identical normalized source and match the compiled digest.
- Change version, trigger type/parameters, or Repeat body Set output name and require distinct compiled digests.
- Reformat or comment equivalent YAML and require the same digest.
- Zero only the serialized payload's digest field, recompute BLAKE3, and match `CompiledWorkflow::digest()`.
- Verify old-wire decode/re-encode and admission behavior according to the coordinated compatibility decision.
- Preserve property coverage while keeping each source file under 300 lines.
