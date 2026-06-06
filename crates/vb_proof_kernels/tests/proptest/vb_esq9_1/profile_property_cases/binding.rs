//! Moon task profile binding properties.

use proptest::prelude::*;
use vb_proof_kernels::profile_contract::{
    ProfileName,
    binding::{BindingResult, MoonTaskProfileBinding, ProfileRefKind, bind_moon_task},
};

use super::strategies::arb_workspace_profile_set;

proptest! {
    #[test]
    fn prop_moon_task_profile_binding_correct(ws in arb_workspace_profile_set()) {
        assert_hardened_binding(&ws);
        assert_bench_binding(&ws);
        assert_deferred_scope_bindings(&ws);
    }
}

fn assert_hardened_binding(ws: &vb_proof_kernels::profile_contract::WorkspaceProfileSet) {
    let binding = MoonTaskProfileBinding {
        task_name: "hardened-build",
        profile_ref: ProfileRefKind::Explicit(ProfileName::Hardened),
        in_pipeline: true,
        run_in_ci: true,
    };
    match bind_moon_task(&binding, ws) {
        BindingResult::ExistsAndValid | BindingResult::ExistsButGapped(_) => {
            assert!(ws.has(ProfileName::Hardened));
        }
        BindingResult::Missing => assert!(!ws.has(ProfileName::Hardened)),
        BindingResult::DeferredScope => {}
    }
}

fn assert_bench_binding(ws: &vb_proof_kernels::profile_contract::WorkspaceProfileSet) {
    let binding = MoonTaskProfileBinding {
        task_name: "bench-build",
        profile_ref: ProfileRefKind::ImplicitBench,
        in_pipeline: true,
        run_in_ci: true,
    };
    match bind_moon_task(&binding, ws) {
        BindingResult::ExistsAndValid | BindingResult::ExistsButGapped(_) => {
            assert!(ws.has(ProfileName::Bench));
        }
        BindingResult::Missing => assert!(!ws.has(ProfileName::Bench)),
        BindingResult::DeferredScope => {}
    }
}

fn assert_deferred_scope_bindings(ws: &vb_proof_kernels::profile_contract::WorkspaceProfileSet) {
    let deferred_binding = MoonTaskProfileBinding {
        task_name: "pgo-maxperf-build",
        profile_ref: ProfileRefKind::Explicit(ProfileName::Release),
        in_pipeline: false,
        run_in_ci: false,
    };
    assert!(matches!(
        bind_moon_task(&deferred_binding, ws),
        BindingResult::DeferredScope
    ));

    let maxperf_named = MoonTaskProfileBinding {
        task_name: "maxperf",
        profile_ref: ProfileRefKind::ImplicitRelease,
        in_pipeline: false,
        run_in_ci: false,
    };
    assert!(matches!(
        bind_moon_task(&maxperf_named, ws),
        BindingResult::DeferredScope
    ));

    let maxperf_active = MoonTaskProfileBinding {
        task_name: "maxperf",
        profile_ref: ProfileRefKind::ImplicitRelease,
        in_pipeline: false,
        run_in_ci: true,
    };
    assert!(!matches!(
        bind_moon_task(&maxperf_active, ws),
        BindingResult::DeferredScope
    ));
}
