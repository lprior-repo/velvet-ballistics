# Parked Verus specs

Files here are Verus specs whose binding target no longer exists. They are
kept for the model they contain, not verified, and not covered by any gate.

## contracts_as_data_spec.rs

Parked 2026-11-12 per the lane-C decision recorded in the hammer log (park,
do not bridge). The spec is for contracts-as-data (vb-6f02); its binding
target `xtask/src/contracts.rs` was deleted by commits b24d9122
("Rip out xtask entirely") and 492415fc. The file's own header still claims
its spec functions bind to those exec fns; that claim is stale.

Why parked, not fixed: a 2026-11-08 lane-C investigation fixed 14 type
errors locally (unpushed commit b8fc8991, "fix(verus): repair 14 type errors
in contracts_as_data_spec [PARTIAL, DO NOT MERGE]", present only in the
lane-C hammer clone) and recorded 27 deeper Verus-compat errors remaining.
Bridging would take hours on a file no gate covers. Verified 2026-11-12 on
main c5e922a7: zero entries in `contracts/proof_obligations.yaml`, so it is
not a verify-verus target; the citation audit scans
`verification/verus/*.rs` only; the moon verus task globs
`verification/verus/**` only; the source-length ledger holds no row for
this path.

Restore path: remodel with a Seq-based model including a
sorting-determinism proof, or use trusted `assume_specification` bridges,
then re-bind to live production code and re-add a proof-obligation entry.
Do not move it back into `verification/verus/` or `contracts/verus/`
without that re-binding.
