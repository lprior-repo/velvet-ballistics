# Ticket-key proof projection repair

The proof projection and mathematical specification must preserve the native four-input ticket-key recipe: u64 run, u16 run attempt, u64 sequence and u16 action, with the same operation order and wrapping u128 arithmetic. Native ticket formats, identities and implementation remain unchanged.

The reproducer calls the actual compiled public core function and the actual assigned projection. Original projection must compile and disagree; repaired projection must agree across the retained typed corner and mixed vectors. The registered Verus target must pass, with the seven inherited external/specification bridges explicitly retained as TRUSTED and conditional. No global key equivalence, injectivity, canonical policy-order closure or full-engine acceptance is claimed.

Land only the two reviewed proof files and this contract on current main, preserving all incoming source. Independent CODE/TEST review, current-main citation/drift/binding checks, exact source/evidence hashes and normal remote push are required. The runtime-delegate citation is rebound numerically to current main without changing executable bytes.
