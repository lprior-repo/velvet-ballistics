//! Live generic-body result projection, directly bound to executable source.
//!
//! strip_type preserves every error payload E and None, and projects Some((S,T))
//! to Some(S). keep_last preserves the complete supplied output/type pair.
//! These are universal generic contracts, not a shadow CompileError classifier.
//!
//! Native differential tests retain exact CompileError variants/payloads, first
//! propagated errors and partial builder effects against the original emitter.
//! Error emission, callee implementations and formatting are not Verus-proved.
//! Legacy cfg(test) emitter tests remain legacy; they supply no live closure.
#[path = "../../crates/vb_compile/src/mod_compile_lowering/generic_body_values.rs"]
mod production;
