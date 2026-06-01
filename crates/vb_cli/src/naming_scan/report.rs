use std::fs;
use std::path::Path;

use super::ordering::compare_finding;
use super::types::*;

pub fn render_scan_report(report: &ScanReport) -> Result<RenderedReport, NamingScanError> {
    let mut findings = report.findings.clone();
    findings.sort_by(compare_finding);
    let body = render_body(report, findings);
    if let Some(destination) = &report.report_destination {
        write_report(destination, &body)?;
    }
    Ok(RenderedReport { body })
}

fn render_body(report: &ScanReport, findings: Vec<NamingFinding>) -> String {
    if findings.is_empty() {
        return format!(
            "canonical spelling scan: 0 findings; selected={}; scanned={}\n",
            report.selected_input_count, report.scanned_text_input_count
        );
    }
    findings
        .iter()
        .map(render_finding)
        .collect::<Vec<String>>()
        .join("")
}

fn render_finding(finding: &NamingFinding) -> String {
    format!(
        "{}:{}:{} {:?} -> {}\n",
        finding.path,
        finding.line.get(),
        finding.column.get(),
        finding.spelling_class,
        finding.remediation
    )
}

fn write_report(destination: &Path, body: &str) -> Result<(), NamingScanError> {
    if !destination_parent_exists(destination) {
        return Err(NamingScanError::ReportWriteFailed {
            path: destination.to_path_buf(),
            source: "parent directory does not exist".to_owned(),
        });
    }
    fs::write(destination, body).map_err(|source| NamingScanError::ReportWriteFailed {
        path: destination.to_path_buf(),
        source: source.to_string(),
    })
}

fn destination_parent_exists(destination: &Path) -> bool {
    match destination.parent() {
        Some(parent) if parent.as_os_str().is_empty() => true,
        Some(parent) => parent.is_dir(),
        None => true,
    }
}
