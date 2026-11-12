use super::{
    ApiSurfaceRecord, AuditResult, BenchmarkEvidence, BloatRecord, EvidenceBundle,
    EvidenceGateFailure, SemverRecord, enrich_benchmark_evidence, parse_criterion_output,
    required_kernel_groups,
};

#[test]
fn benchmark_evidence_complete_has_all_fields() {
    let evidence = BenchmarkEvidence {
        name: "test".to_string(),
        baseline: Some("vb-current".to_string()),
        result: Some("100 ns".to_string()),
        environment: Some("toolchain=nightly;cpu=x86_64".to_string()),
        command: Some("cargo bench".to_string()),
    };
    assert!(evidence.is_complete());
}

#[test]
fn benchmark_evidence_missing_baseline_is_incomplete() {
    let evidence = BenchmarkEvidence {
        name: "test".to_string(),
        baseline: None,
        result: Some("100 ns".to_string()),
        environment: Some("env".to_string()),
        command: Some("cmd".to_string()),
    };
    assert!(!evidence.is_complete());
    assert!(!evidence.has_baseline());
}

#[test]
fn benchmark_evidence_empty_baseline_is_incomplete() {
    let evidence = BenchmarkEvidence {
        name: "test".to_string(),
        baseline: Some(String::new()),
        result: Some("100 ns".to_string()),
        environment: Some("env".to_string()),
        command: Some("cmd".to_string()),
    };
    assert!(!evidence.is_complete());
    assert!(!evidence.has_baseline());
}

#[test]
fn evidence_bundle_validates_missing_supply_chain() {
    let bundle = EvidenceBundle::new(
        "nightly-2026-04-28".to_string(),
        "x86_64".to_string(),
        "2026-05-17".to_string(),
    );
    let failures = bundle.validate_gates();
    assert!(
        failures
            .iter()
            .any(|f| matches!(f, EvidenceGateFailure::MissingSupplyChainEvidence))
    );
}

#[test]
fn evidence_bundle_validates_audit_failure_blocks_gate() {
    let mut bundle = EvidenceBundle::new(
        "nightly-2026-04-28".to_string(),
        "x86_64".to_string(),
        "2026-05-17".to_string(),
    );
    bundle.supply_chain_audits.push(AuditResult {
        tool: "cargo-deny".to_string(),
        exit_code: Some(1),
        output_path: None,
        passed: false,
        notes: "license failure".to_string(),
    });
    let failures = bundle.validate_gates();
    assert!(
        failures
            .iter()
            .any(|f| matches!(f, EvidenceGateFailure::AuditFailure { .. }))
    );
}

#[test]
fn evidence_bundle_validates_missing_benchmark_baseline_blocks_speed_claim() {
    let mut bundle = EvidenceBundle::new(
        "nightly-2026-04-28".to_string(),
        "x86_64".to_string(),
        "2026-05-17".to_string(),
    );
    bundle.benchmark_evidence.push(BenchmarkEvidence {
        name: "fast_bench".to_string(),
        baseline: None,
        result: Some("10 ns".to_string()),
        environment: Some("env".to_string()),
        command: Some("cargo bench".to_string()),
    });
    let failures = bundle.validate_gates();
    assert!(
        failures
            .iter()
            .any(|f| matches!(f, EvidenceGateFailure::MissingBenchmarkBaseline { .. }))
    );
}

#[test]
fn evidence_bundle_passes_with_complete_evidence() {
    let mut bundle = EvidenceBundle::new(
        "nightly-2026-04-28".to_string(),
        "x86_64".to_string(),
        "2026-05-17".to_string(),
    );
    bundle.supply_chain_audits.push(AuditResult {
        tool: "cargo-audit".to_string(),
        exit_code: Some(0),
        output_path: None,
        passed: true,
        notes: String::new(),
    });
    bundle.api_surface = Some(ApiSurfaceRecord {
        crate_name: "vb_core".to_string(),
        version: "0.1.0".to_string(),
        public_item_count: 42,
    });
    bundle.semver_record = Some(SemverRecord {
        crate_name: "vb_core".to_string(),
        current_version: "0.1.0".to_string(),
        previous_version: None,
        breaking_changes: Vec::new(),
    });
    bundle.bloat_analysis = Some(BloatRecord {
        binary_path: "target/release/velvet-ballistics".to_string(),
        total_size_bytes: 5_000_000,
        top_contributors: Vec::new(),
    });
    bundle.benchmark_evidence.push(BenchmarkEvidence {
        name: "yaml_parse".to_string(),
        baseline: Some("vb-current".to_string()),
        result: Some("100 ns".to_string()),
        environment: Some("env".to_string()),
        command: Some("cargo bench".to_string()),
    });
    bundle.kernel_paths_covered = required_kernel_groups()
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert!(bundle.all_gates_pass());
}

#[test]
fn parse_criterion_output_extracts_benchmark_names() {
    let output = "yaml_parse/parse_yaml_small;BenchmarkId(parse_yaml_small); time: [100.0 ns]";
    let records = parse_criterion_output(output);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].name, "parse_yaml_small");
}

#[test]
fn enrich_benchmark_evidence_adds_command_and_environment() {
    let mut evidence = BenchmarkEvidence {
        name: "test".to_string(),
        baseline: Some("vb-current".to_string()),
        result: Some("100 ns".to_string()),
        environment: None,
        command: None,
    };
    enrich_benchmark_evidence(
        &mut evidence,
        "cargo bench -- --save-baseline vb-current",
        "nightly-2026-04-28",
        "x86_64-unknown-linux-gnu",
    );
    assert!(evidence.command.is_some());
    assert!(evidence.environment.is_some());
    assert!(
        evidence
            .command
            .as_ref()
            .is_some_and(|c| c.contains("cargo bench"))
    );
    assert!(
        evidence
            .environment
            .as_ref()
            .is_some_and(|e| e.contains("toolchain=nightly-2026-04-28"))
    );
}

#[test]
fn required_kernel_groups_lists_all_kernel_paths() {
    let groups = required_kernel_groups();
    assert!(groups.contains(&"yaml_parse"));
    assert!(groups.contains(&"compile_validate"));
    assert!(groups.contains(&"expression"));
    assert!(groups.contains(&"runtime_core"));
    assert!(groups.contains(&"storage_ipc"));
    assert!(!groups.contains(&"generated_mode"));
    assert_eq!(groups.len(), 5);
}

#[test]
fn evidence_gate_failure_display_formats() {
    let failure = EvidenceGateFailure::MissingBenchmarkBaseline {
        benchmarks: vec!["fast_bench".to_string()],
    };
    let display = format!("{failure}");
    assert!(display.contains("fast_bench"));
    assert!(display.contains("baseline"));
}

#[test]
fn audit_failure_with_multiple_tools_lists_all() {
    let failure = EvidenceGateFailure::AuditFailure {
        tools: vec!["cargo-deny".to_string(), "cargo-vet".to_string()],
    };
    let display = format!("{failure}");
    assert!(display.contains("cargo-deny"));
    assert!(display.contains("cargo-vet"));
}
