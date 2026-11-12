# Hazards
Unconditional booleans and gate counts launder missing checks. Reusing source
and artifact identities breaks recovery. Rehashing a tampered envelope alone does
not certify semantic validity. Self-digests require zeroing only self-reference
fields. A declared external contract cannot prove executor behavior. Decoder and
ledger schema evolution must fail closed. Compiler checks do not prove recovery,
effect separation, grants, replay or successful fsync. Shared-schema edits require
coordination with admission/recovery owners. No async behavior is introduced.
