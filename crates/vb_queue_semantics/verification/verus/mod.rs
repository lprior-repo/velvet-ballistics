// Verus-native exec-bridge proofs for vb_queue_semantics.
// queue_semantics_specs was deleted: it was a disconnected spec mirror
// that proved tautologies about abstract int-typed functions with no
// extern_spec! bindings or reveal_with_fuel to production code.
// All production bindings are in queue_semantics_exec_bridges.rs.
#[cfg(verus)]
pub mod queue_semantics_exec_bridges;
