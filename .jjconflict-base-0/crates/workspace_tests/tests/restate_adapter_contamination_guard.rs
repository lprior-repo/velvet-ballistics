#![forbid(unsafe_code)]

use proptest::prelude::*;
use xtask::{WorkspaceManifest, XtaskCommandError, assert_runtime_dependency_boundary};

fn crate_name(case: u8) -> &'static str {
    match case % 5 {
        0 => "vb_core",
        1 => "vb_runtime",
        2 => "vb_storage",
        3 => "vb_ipc",
        _ => "vb_cli",
    }
}

fn dependency_name(case: u8) -> &'static str {
    match case % 8 {
        0 => "serde_json",
        1 => "reqwest",
        2 => "hyper",
        3 => "toml",
        4 => "clap",
        5 => "xtask",
        6 => "serde",
        _ => "postcard",
    }
}

fn protected_runtime(case: u8) -> bool {
    (case % 5) < 4
}

fn forbidden_runtime_dependency(case: u8) -> bool {
    (case % 8) < 6
}

proptest! {
    #[test]
    fn forbidden_manifest_dependencies_reject_aliases(crate_case in any::<u8>(), dep_case in any::<u8>()) {
        let crate_name = crate_name(crate_case);
        let dependency_name = dependency_name(dep_case);
        let manifest = WorkspaceManifest::from_edges([(crate_name, dependency_name)]);
        let result = assert_runtime_dependency_boundary(&manifest);

        if protected_runtime(crate_case) && forbidden_runtime_dependency(dep_case) {
            prop_assert!(matches!(
                result,
                Err(XtaskCommandError::DependencyBoundaryViolation { .. })
            ), "expected DependencyBoundaryViolation for protected runtime forbidden dependency");
        } else {
            prop_assert!(result.is_ok());
        }
    }
}
