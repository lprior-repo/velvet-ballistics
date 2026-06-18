//! Retired Verus artifact for `vb_boundary_inventory` validation obligations.
//!
//! The previous version of this file declared local mirror types and local
//! predicates for OBL-BI-001 through OBL-BI-006. Those predicates were not
//! mechanically bound to `crates/vb_boundary_inventory` production functions
//! or production types, and several contradicted production behavior. Keeping
//! them would preserve a vacuum-proof claim.
//!
//! This file intentionally retains no validation specs, proof declarations,
//! assumed standard-library declarations, trusted external bodies, or
//! production-model claim. The registry now marks OBL-BI-* as pending
//! production binding instead of deductively verified. Future proof work must
//! either:
//!
//! - bind directly to production functions/types with an auditable Verus
//!   mechanism, or
//! - introduce a shared production proof kernel that production code calls and
//!   Verus checks against the same source.
//!
//! Until then, `verus --crate-type=lib` over this file is only a syntax/trust
//! regression check for the retired artifact; it is not production proof
//! evidence for any OBL-BI obligation.

use vstd::prelude::*;

verus! {}
