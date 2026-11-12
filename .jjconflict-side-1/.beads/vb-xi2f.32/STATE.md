State: 2

Explore complete. codebase-map.md and delivery-scope.jsonl written.

Key finding: The canonical_digest function (in two copies: mod_compile_lowering/part_05.rs
and compile/mod.rs) does NOT hash Wait primitive fields (event, timeout). It only hashes
the string "wait". This means two workflows with different wait conditions produce the
same digest. The fix requires adding a Wait match arm in digest_step_primitive in both
files that hashes both event and timeout fields.

Artifacts ready for rust-contract → proof-planner → proof-writer → holzman-rust pipeline.
