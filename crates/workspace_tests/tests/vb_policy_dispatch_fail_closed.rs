#![forbid(unsafe_code)]
//! Policy dispatch fail-closed tests (vb-l5ru0).
//!
//! Tests actual public Runtime APIs: live policy evaluation, emergency
//! deny overlay, and strict policy port failure propagation. All three
//! tests use genuine accepted artifacts, pending action tickets, and
//! real FjallRegister effects (or lack thereof). No fake state or cache
//! substitutes.

#[path = "vb_policy_dispatch_fail_closed/fixtures.rs"]
mod fixtures;

use fixtures::*;

use std::sync::Arc;
use vb_core::action_pack::RecoveryClass;
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx};
use vb_core::policy::{
    CurrentPolicyIdentity, DecisionCode, DecisionRecord, InfluenceState, PinnedAdmissionPolicy,
    PolicyDigest, PolicyLayer, PolicyRule, RuntimePolicy,
};
use vb_core::value::{SlotValue, Taint};
use vb_runtime::RuntimeError;
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::{RegisteredExecutionError, Runtime};
use vb_storage::executor::{ExecutorCatalog, FjallRegister};
use vb_storage::{EventSeq, FjallConfig, FjallJournal};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[path = "vb_policy_dispatch_fail_closed/suspended.rs"]
mod suspended;

#[path = "vb_policy_dispatch_fail_closed/emergency.rs"]
mod emergency;

#[path = "vb_policy_dispatch_fail_closed/strict_failure.rs"]
mod strict_failure;
