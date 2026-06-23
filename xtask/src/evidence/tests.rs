// This file is included via `include!()` from `xtask/src/evidence.rs`.
// Inner attributes (`#![...]`) are not valid in this context because the
// include site is in the middle of the parent module; outer attributes
// (`#[...]`) on the next item work correctly.
#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::as_conversions,
        clippy::arithmetic_side_effects,
        clippy::indexing_slicing,
        clippy::let_underscore_must_use,
        clippy::panic,
        clippy::panic_in_result_fn
    )]
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn gate_profile_evidence_files_and_gates_are_stable() {
        assert_eq!(GateProfile::AiFast.evidence_file(), "ai-fast.yaml");
        assert_eq!(GateProfile::AiDeep.evidence_file(), "ai-deep.yaml");
        assert_eq!(GateProfile::AiRelease.evidence_file(), "ai-release.yaml");
        assert_eq!(
            GateProfile::AiFast.gates(),
            &[
                "fmt",
                "check",
                "clippy",
                "nextest",
                "forbidden-scan",
                "hotpath-scan"
            ]
        );
        assert_eq!(
            GateProfile::AiDeep.gates(),
            &["miri", "mutants", "llvm-cov", "fuzz-build"]
        );
        assert!(GateProfile::AiRelease.gates().contains(&"maxperf"));
    }

    #[test]
    fn evidence_path_stays_under_bead_directory() {
        assert_eq!(
            evidence_path("vb-kkvb", "fmt"),
            PathBuf::from(".evidence/vb-kkvb/fmt.yaml")
        );
    }

    #[test]
    fn failed_gate_explains_failure_with_hint_and_repair() {
        let evidence = GateEvidence {
            kind: "gate-evidence".to_string(),
            gate_name: "fmt".to_string(),
            command: "cargo fmt".to_string(),
            exit_code: 1,
            log: PathBuf::from("fmt.log"),
            status: GateStatus::Fail,
            why_failed: None,
        };
        let why = explain_failure(&evidence);
        assert!(why.is_some(), "failed evidence explains failure");
        if let Some(why) = why {
            assert_eq!(why.gate_name, "fmt");
            assert!(!why.hint.is_empty());
            assert!(!why.repair_command.is_empty());
        }
    }

    #[test]
    fn release_bead_id_accepts_only_supported_release_bead() {
        assert_eq!(ReleaseBeadId::parse("vb-nf2u"), Ok(ReleaseBeadId::VbNf2u));
        assert!(ReleaseBeadId::parse("vb-other").is_err());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // BND-13: explain_failure returns WhyFailed for GateStatus::Fail,
    //          None for Pass/Skipped
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn explain_failure_returns_none_when_status_is_pass() {
        let evidence = GateEvidence {
            kind: "gate-evidence".to_string(),
            gate_name: "fmt".to_string(),
            command: "cargo fmt".to_string(),
            exit_code: 0,
            log: PathBuf::from("fmt.log"),
            status: GateStatus::Pass,
            why_failed: None,
        };
        let why = explain_failure(&evidence);
        assert_eq!(
            why, None,
            "Pass status must produce None from explain_failure"
        );
    }

    #[test]
    fn explain_failure_returns_none_when_status_is_skipped() {
        let evidence = GateEvidence {
            kind: "gate-evidence".to_string(),
            gate_name: "miri".to_string(),
            command: "cargo +nightly miri test".to_string(),
            exit_code: 0,
            log: PathBuf::from("miri.log"),
            status: GateStatus::Skipped {
                reason: "miri not available on this platform".to_string(),
            },
            why_failed: None,
        };
        let why = explain_failure(&evidence);
        assert_eq!(
            why, None,
            "Skipped status must produce None from explain_failure"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // BND-14: validate_evidence_dir returns MissingEvidence for each absent gate
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn validate_evidence_dir_returns_missing_evidence_error_for_each_absent_gate() {
        // Given: a temp directory with no gate files
        let dir = tempfile::tempdir().expect("create temp dir");
        let missing_gates = &["fmt", "clippy", "test"];

        // When: validate_evidence_dir is called with required gates
        let result = validate_evidence_dir(dir.path(), missing_gates);

        // Then: returns exactly one MissingEvidence error per absent gate
        let errors = result.unwrap();
        assert_eq!(
            errors.len(),
            missing_gates.len(),
            "must produce one MissingEvidence per absent gate file"
        );
        for gate in missing_gates {
            assert!(
                errors.iter().any(|e| {
                    matches!(
                        e,
                        Error::MissingEvidence { gate: g, .. } if g == gate
                    )
                }),
                "must have MissingEvidence error for gate '{}'",
                gate
            );
        }
    }

    #[test]
    fn validate_evidence_dir_returns_empty_vec_when_all_gates_present() {
        use std::fs;

        // Given: a temp directory with all required gate files present
        let dir = tempfile::tempdir().expect("create temp dir");
        let required_gates = &["fmt", "clippy"];
        for gate in required_gates {
            let path = dir.path().join(format!("{gate}.yaml"));
            fs::write(&path, "status: pass\n").expect("write gate file");
        }

        // When: validate_evidence_dir is called
        let result = validate_evidence_dir(dir.path(), required_gates);

        // Then: returns Ok with empty error vec
        assert!(
            result.as_ref().unwrap().is_empty(),
            "no errors when all gate files are present"
        );
    }

    #[test]
    fn validate_evidence_dir_returns_partial_errors_when_some_gates_missing() {
        use std::fs;

        // Given: a temp directory with only "fmt.yaml" present
        let dir = tempfile::tempdir().expect("create temp dir");
        fs::write(dir.path().join("fmt.yaml"), "status: pass\n").expect("write fmt gate file");
        let required_gates = &["fmt", "clippy", "test"];

        // When: validate_evidence_dir is called
        let result = validate_evidence_dir(dir.path(), required_gates);

        // Then: returns exactly 2 MissingEvidence errors (clippy and test)
        let errors = result.unwrap();
        assert_eq!(errors.len(), 2);
        assert!(errors
            .iter()
            .any(|e| { matches!(e, Error::MissingEvidence { gate: g, .. } if g == "clippy") }));
        assert!(errors
            .iter()
            .any(|e| { matches!(e, Error::MissingEvidence { gate: g, .. } if g == "test") }));
    }
}
