# Frozen typed-policy foundation scope

Local5090 authored production evaluator/types; local3090 cleaned new test constructs only. Root authored metadata/VCS, no product Rust. Source manifest foundation-source-freeze-v2.json is the review identity. Final cleanup raw tests report 49 core + 92 runtime matches, not 92 total; includes existing policy-related tests. Strict lib clippy actual -- -D warnings and pinned fmt commands exited 0.

Contract wording corrections: configure is infallible (plain actor state assignment); input digest uses streaming domain-separated BLAKE3 over canonical caller-supplied bytes, taint and all eight ticket fields, not postcard encoding in the evaluator. Compatibility influence is currently NotEvaluated; the shared InfluenceState has NotEvaluated/Inactive/Active, no embedded code. These are explicit bounded foundation choices, not parent acceptance.

Public policy records are facts, not executor grants. PolicyDigest wraps caller-supplied bytes; the constructor does not itself verify artifacts. Admission binding must source the actual accepted policy digest during parent integration. No fabricated default identity is introduced by the evaluator. Emergency activation is monotonic for this evaluator lifetime only; durable recovery and actor wiring remain parent obligations. The module is not called by dispatch yet.

No complete runtime authorization, admission pin persistence, emergency persistence, retry reauthorization, executor-boundary enforcement, historical replay or v1 closure is claimed. Old handoffs and intermediate overwritten/bare gate logs are historical and do not replace final raw gate evidence. Reviews must independently inspect source and test strength and reject concrete defects.
