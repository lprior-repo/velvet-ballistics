#![forbid(unsafe_code)]
// vb-c4hzw: Legacy unverified admission recovered NEW legal Live effect denial.
//
// A genuine legacy RunAdmission with RunPolicyPin::Unverified is reconstructed
// from an accepted artifact and complete historical event prefix. Current
// Allowed policy cannot authorize its new effect: actual Runtime returns
// Admission-layer Denied with genuine Unverified pin, full independent record
// oracle, unchanged exact pending state and zero effects.

#[path = "vb_c4hzw_legacy_admission/policy_dispatch.rs"]
mod policy_dispatch;

#[path = "vb_c4hzw_legacy_admission/legacy_workflow.rs"]
mod legacy_workflow;

#[path = "vb_c4hzw_legacy_admission/legacy_admission_test.rs"]
mod legacy_admission_test;
