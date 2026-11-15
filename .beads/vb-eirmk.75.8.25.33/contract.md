# Compiled IR digest

MASTER §63 requires the existing verify report `artifact.ir_digest_hex` to identify the actual Postcard-encoded compiled WorkflowParts emitted by `compile --emit ir`. Workflow/source digest conventions, output schema, profiles, runtime behavior and persisted formats remain unchanged.

Use the existing serializer and BLAKE3 dependency. Serialization failure follows the existing typed IR validation error route. Public CLI regression checks compare emitted file bytes with the reported digest for distinct valid workflows across existing profiles, while preserving the structural workflow digest field. No GPU or benchmark work.
