//! Moon task profile binding — resolves Moon CI task profile references.
//!
//! Bead: vb-esq9.1 | State: 5 (proof-writer)
//! Pure core: maps a task's profile reference to a BindingResult.

use crate::profile_contract::MASTER_PROFILE_CONTRACT;
use crate::profile_contract::errors::ContractGap;
use crate::profile_contract::types::ProfileName;
use crate::profile_contract::validation::validate_against_master;
use crate::profile_contract::workspace::WorkspaceProfileSet;

/// The kind of profile reference a Moon task uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileRefKind {
    /// Explicit --profile argument, e.g., --profile hardened
    Explicit(ProfileName),
    /// Implicit bench (cargo bench)
    ImplicitBench,
    /// Implicit release (--release flag)
    ImplicitRelease,
}

/// A binding between a Moon task and its Cargo profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoonTaskProfileBinding {
    pub task_name: &'static str,
    pub profile_ref: ProfileRefKind,
    pub in_pipeline: bool,
    pub run_in_ci: bool,
}

/// The result of binding a Moon task's profile reference to the workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingResult {
    /// Profile exists and satisfies master contract.
    ExistsAndValid,
    /// Profile exists but has master contract gaps.
    ExistsButGapped(Vec<ContractGap>),
    /// Profile does not exist in the workspace profile set.
    Missing,
    /// Profile reference is "maxperf" — intentionally absent, task is deferred.
    DeferredScope,
}

/// Bind a Moon task's profile reference to the current workspace profile set.
///
/// - Tasks with "maxperf" in task_name AND run_in_ci=false → DeferredScope
/// - If the profile ref is a valid ProfileName and exists in the set → check gaps
/// - If the profile ref is a valid ProfileName but missing → Missing
/// - ImplicitBench maps to ProfileName::Bench
/// - ImplicitRelease maps to ProfileName::Release
pub fn bind_moon_task(
    binding: &MoonTaskProfileBinding,
    set: &WorkspaceProfileSet,
) -> BindingResult {
    // Deferred-scope check: tasks referencing maxperf that are not in CI
    // are deferred per master §2:166, §22:1085-1098.
    // The detection uses task_name because "maxperf" as a profile name
    // is rejected by ProfileName construction. Tasks that reference
    // maxperf conceptually (pgo-instrument-build, pgo-optimized-build,
    // or any task with "maxperf" in its name) get deferred.
    if binding.task_name.contains("maxperf") && !binding.run_in_ci {
        return BindingResult::DeferredScope;
    }

    let profile_name = match &binding.profile_ref {
        ProfileRefKind::Explicit(name) => *name,
        ProfileRefKind::ImplicitBench => ProfileName::Bench,
        ProfileRefKind::ImplicitRelease => ProfileName::Release,
    };

    if !set.has(profile_name) {
        return BindingResult::Missing;
    }

    let gaps = validate_against_master(set, &MASTER_PROFILE_CONTRACT);

    if gaps.is_empty() {
        BindingResult::ExistsAndValid
    } else {
        BindingResult::ExistsButGapped(gaps)
    }
}
