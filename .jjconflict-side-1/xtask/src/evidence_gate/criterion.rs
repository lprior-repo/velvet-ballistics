use super::BenchmarkEvidence;

/// Known kernel benchmark groups that must have evidence.
pub fn required_kernel_groups() -> &'static [&'static str] {
    &[
        "yaml_parse",
        "compile_validate",
        "expression",
        "runtime_core",
        "storage_ipc",
    ]
}

/// Parse criterion benchmark output into BenchmarkEvidence records.
pub fn parse_criterion_output(output: &str) -> Vec<BenchmarkEvidence> {
    let mut records = Vec::new();
    for line in output.lines() {
        if line.contains("time:") && line.contains("BenchmarkId") {
            let name = extract_benchmark_name(line).unwrap_or_default();
            records.push(BenchmarkEvidence {
                name,
                baseline: Some("vb-current".to_string()),
                result: extract_time_value(line),
                environment: None,
                command: None,
            });
        }
    }
    records
}

fn extract_benchmark_name(line: &str) -> Option<String> {
    let (_, rest) = line.split_once("BenchmarkId(")?;
    let (name, _) = rest.split_once(')')?;
    Some(name.to_string())
}

fn extract_time_value(line: &str) -> Option<String> {
    let (_, rest) = line.split_once("time: [")?;
    let (time, _) = rest.split_once(']')?;
    Some(time.to_string())
}

/// Enrich benchmark evidence with environment and command metadata.
pub fn enrich_benchmark_evidence(
    evidence: &mut BenchmarkEvidence,
    command: &str,
    toolchain: &str,
    host_cpu: &str,
) {
    evidence.command = Some(command.to_string());
    evidence.environment = Some(format!("toolchain={toolchain};host_cpu={host_cpu}"));
}
