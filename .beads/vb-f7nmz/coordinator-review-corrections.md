# Exact scope and corrections to the independent replay review

The frozen source e75142a4db43bad541a265ed7cecee0652c206b2 was independently reviewed by LOCAL5090, with actual 19-case test, storage-recovery and runtime-shard execution and scoped Clippy logs. The two production changes and the restored test oracles are accepted in that bounded scope. The reviewer reports contain these factual errors; the source and actual logs take precedence.

- The stale certificate oracle is AdmissionError::ArtifactCertificateStale { digest, accepted_at_seq: 4, required_at_least: 5 }, including exact Display and full before/after journal-vector equality. It is not RecoveryError::StaleCertificateFloor.
- The original eight obligations are six restored admission/replay cases plus TWO strengthened lifecycle guards. The eighth original obligation is given_completed_run_when_lifecycle_retry_uses_stale_key_then_lifecycle_stale_request, with CoreError::LifecycleStaleRequest, exact code/command and complete unchanged journal. The new missing-schedule negative is additional coverage, not its replacement.
- Duplicate retry uses CoreError::LifecycleDuplicateRequest, exact code and command Some("retry"); the reviewer wording LifecycleError::DuplicateRequest is inaccurate.
- There is no separate counters infrastructure test. There are ten retained tests, six restored cases, and three additional missing/wrong/zero ABI cases, for 19 actual named tests.
- The positive completed-action replay test checks tracker.is_resolved(actual_ticket.action, actual_ticket.step), full replayed == before, schedule ABI == completion ABI == expected ABI != zero, schedule/completion counts, terminal event and after == before. The eviction case also resolves the actual first ticket after capacity-one eviction and compares full vectors. These positive producer assertions catch a zero completion ABI regression. The zero-EXPECTED-ABI negative alone would not prove the producer writes a nonzero completion ABI.
- The report phrase drive_run uses preflight.outcome is not an actual field/path in this implementation. The reviewed function validates ownership/preflight and computes ABI before append, then applies validated output, advances state and drives the run.
- No conclusion about the correctness or ABI requirements of untouched/orphan failure-envelope code follows from this review. Those statements are excluded from acceptance; no such code was resurrected or altered.
- Reviewer Clippy logs use -D warnings only. The coordinator canonical lint-src task subsequently exercised the repository full hard-deny list and passed on the integrated product source.
- Drift/binding passes and 72 WEAK mirror bindings are tooling evidence. They do not constitute whole-production proof closure.

Coordinator fresh canonical test lane: 14358 passed, 27 pre-existing skipped, 262.381 seconds. The broader command remains unfinished and has a separate benchmark-policy failure, so this is not an unrestricted CI pass.
