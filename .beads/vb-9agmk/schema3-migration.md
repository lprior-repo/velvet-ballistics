# Schema3 semantic-source payload migration

Based on source owner31e5182d0d739b345e1ea608c495033c2f1bf869 (not independently reviewed here). Evidence version3 binds and roundtrips WorkflowParts.semantic_source_identity, with explicit Some for source-backed compilation and None for IR-only construction. Schema2 returns MigrationRequired{from:2,to:3} before decoding its old positional WorkflowParts layout. Unverified legacy claims require1->3 regeneration. Unknown versions reject unsupported.

The actual schema2 artifact and IR emitted from57f are decoded as historical AcceptedArtifact wire data. The old IR fails new WorkflowParts decode because its identity field is absent; the verifier nevertheless returns the exact migration error first, including policy-specific verification. Schema3 tests preserve source identity, bytes and workflow digest, preserve explicit None, and reject an altered semantic identity retaining its old workflow digest even after envelope checksum rebinding.

Actual focused25tests and strict storage source Clippy pass. Commands/exits/stdout/stderr: schema3-focused and schema3-storage-clippy. No compatibility inferred from serde defaults. No broad CI or independent source/test acceptance claimed.
