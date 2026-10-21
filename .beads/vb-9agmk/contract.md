# vb-9agmk artifact contract
C1 Checked evidence can only be constructed after the named production checks run.
C2 Evidence binds canonical workflow identity, exact emitted IR bytes, contracts,
resource policy, and actual source bytes when supplied; identity roles never alias.
C3 Missing contracts, invalid IR, unsafe declared retry semantics, excessive budgets
and a stale workflow digest reject before a candidate result is returned.
C4 Compiler results claim only compiler validations; external action behavior is
classified as an assumption; durability/replay/recovery stay unclaimed.
C5 Accepted ledger schema records all ten master 77.4 minimum obligations with
truthful closure/provenance and is included in accepted-envelope content binding.
C6 Runtime consumers independently validate hostile payload and evidence. Existing
artifacts require explicit migration/fail-closed behavior when schema changes.
Compiler work owns C1-C4; C5-C6 need shared storage/runtime coordination and remain
part of bead acceptance. No full proof-planning pipeline is invoked.

C7 The checked action registry must match numeric runtime dispatch: dense ordered
IDs, unique IDs, and validated names. A sparse or duplicate registry cannot mint
a compiler candidate or accepted envelope.
C8 Strict/Journaled execution binds the supplied serialized WorkflowParts and
exact supplied contracts to the independently verified accepted artifact before
public enqueue and again before processing allocates run state.

Independent review repairs preserve all existing validation order and gates;
new binding checks run after those checks. Runtime source scope was granted by
parent for runtime.rs, shard/lifecycle.rs, shard/lifecycle/chunk_001.rs and new
shard/lifecycle/execution_binding.rs. Runtime recovery production is untouched.
