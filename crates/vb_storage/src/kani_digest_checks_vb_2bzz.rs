#![forbid(unsafe_code)]
//! Kani proof harnesses for digest-check functions exposed by vb-2bzz.
//!
//! Targets the pure mismatch selectors used by `check_action_abi_digests`
//! and `check_policy_digests`. Every harness uses `kani::any()` to generate
//! digest inputs so proofs cover the modeled digest space.
//!
//! Obligations: PPI-005 ... PPI-014.

mod action;
mod policy;
mod support;
