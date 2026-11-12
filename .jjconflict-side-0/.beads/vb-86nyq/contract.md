# Domain

Compiler fingerprinting for numeric intermediate representation.

# Invariant

The fingerprint binds the complete emitted numeric intermediate representation, including declared input kinds. It is calculated from deterministic binary bytes with the fingerprint field zeroed, then stored in that field.

# Errors

A typed compiler encoding error is returned if serializing the emitted IR fails.

# Acceptance

Independent tests recalculate the fingerprint and verify it changes when an input kind or constant is altered. Strict storage admission must accept real compiler output with its correct IR digest, and reject a stale digest after IR mutation. The legacy authoring fingerprint helper remains separate. No release completion or mathematical proof is claimed.

Coordinator clarification: storage admission here means a regression using the existing storage API; compiler code does not write to a database.
