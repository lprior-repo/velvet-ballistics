// Kani harness modules for expression fuzz target verification
// Beads: vb-jpq7.35 (expression), vb-jpq7.35 REPAIR-1

#[cfg(kani)]
pub mod vb_jpq7_35_arithmetic;
#[cfg(kani)]
pub mod vb_jpq7_35_bytecode_bound;
#[cfg(kani)]
pub mod vb_jpq7_35_parser_depth;
#[cfg(kani)]
pub mod vb_jpq7_35_stack;
#[cfg(kani)]
pub mod vb_jpq7_35_token_bound;
#[cfg(kani)]
pub mod vb_xo50x_builtin_eval;
#[cfg(kani)]
pub mod vb_bc33k_type_enforcer;
