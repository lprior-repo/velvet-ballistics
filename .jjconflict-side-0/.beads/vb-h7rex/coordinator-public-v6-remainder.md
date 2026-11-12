# Remaining focused V6 defects

Actual PUBLIC_V6_01 four cases passed; checkpoint filter13 unit cases plus existing integrations passed, strict library Clippy passed, original retry suites2+10+4 and admission95 passed. This is candidate evidence, not acceptance.

New forge_admission helper still uses two forbidden expect calls. All scenario/recovery RuntimeJournal wrappers are still shared_journaled even for explicit Strict policy despite requested matching strict persistence. Must use shared_strict for RuntimePolicy::Strict and shared_journaled for Journaled in BOTH original and reopened handles; keep ShardConfig.policy explicit. These are bounded fixes in new test only. V6 claims must not substitute generic green Clippy for canonical source/test requirements.
