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
