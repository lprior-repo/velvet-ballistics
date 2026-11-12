//! Historical registry filename; current F1 permits multi-step lexical bodies.
//!
//! Directly verifies the shared production keep_last, checked_sum and strip_type
//! bodies. The whole (slot, inferred type) is retained by keep_last; checked_sum
//! checks usize overflow without restricting inputs. No singleton-reject rule.
//!
//! Native differential tests cover integration ordering and builder effects.
//! Recursive widths, name/layout passes, controls, lowering/type inference and
//! complete emitter correctness are not proved by these calculation contracts.
#[path = "../../crates/vb_compile/src/mod_compile_lowering/generic_body_values.rs"]
mod production;
