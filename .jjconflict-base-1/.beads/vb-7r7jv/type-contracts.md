# Proposed type contracts

Retain ActionContract's current layout; adding fields to every literal is
unnecessary for a separate accepted action pack. New core data has no network,
filesystem, JSON or YAML dependency. Newtypes distinguish ExecutorIdentity,
ExecutorVersionDigest, ManifestDigest, ReceiptSchemaDigest, ActionPackDigest,
InputDigest, ReceiptDigest and ConformanceReportDigest. Private fields prevent
public construction of CheckedActionPack or ExecutedConformance.

RawActionPack is deserializable and untrusted. Checking binds exact contract bytes,
manifest schemas, executor identity/version, CapabilityScopeSpec and TimeoutPolicy.
PureOnly and ExternalWithConformance are separate variants rather than booleans
or a bag of optional digests. ExternalWithConformance contains a trusted resolved
conformance record and explicit IdempotencyClass/RecoveryClass. A report hash
alone is insufficient to construct this variant.

ReceiptIdentity includes executor version, action pack digest, full ticket
run/step/seq/action/key, resource scope and exact input digest. Attempts may differ
for same logical key only if key/ticket semantics explicitly allow it; never
recompute a supposedly stable retry key silently. Same key with different input,
resource or pack is ReceiptConflict. Values and receipt payloads are length-
bounded before decode/allocation. Capacity exhaustion returns an error; it never
evicts durable receipts and later repeats an effect.

Recovery authorization is an enum: NoRecoveryNeeded for pure; ReconcileReceipt
for the atomic local register; RetrySameKey only when same-key behavior is
actually enforced; ManualGate/Compensate require additional semantics and remain
unsupported. Blocked is the default for unclassified/non-compensatable writes.
