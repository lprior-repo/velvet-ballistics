# vb-7r7jv domain model (coordination draft)

Master §§77.6-77.7 controls this bead. Base57f307dac5dd. The bead is claimed;
production work waits for root coordination of the boundary described here.

An ActionContract declares action ABI/resources/effects. An ActionManifest adds
semantic schemas and timeout policy. An executor performs effects and has a
registered identity and version. An action pack binds the exact manifest and
executor metadata with a separately produced conformance record. A receipt is
a durable observation of a specific ticket/input/result at a specific executor.
A content digest identifies bytes; it does not establish that a test executed.

IdempotencyClass and RecoveryClass are independent enums from master77.6.
PureDeterministic needs no external conformance claim. A manifest's key declaration
is only KeyRequiredDeclared. KeyEnforcedByExecutor is available solely for the
registered adapter whose actual duplicate-ticket behavior was exercised. Provider
contracts and signatures are outside this minimal lane. Compensation never
implies idempotency; non-compensatable effects default to Blocked.

Suggested first adapter: a bounded local desired-state register backed by a
dedicated Fjall database. One atomic batch writes the target value and its
receipt. This creates a real effect beyond the runtime frame, and a restart can
reconcile the durable receipt without another write. Its guarantee is confined
to this adapter/database; it says nothing about HTTP APIs or other providers.
