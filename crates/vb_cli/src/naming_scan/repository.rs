use super::discovery::discover_scan_inputs;
use super::line_scan::count_error;
use super::ordering::compare_finding;
use super::scan_file;
use super::types::*;

pub(crate) fn scan_repository(root: RepoRoot, config: ScanConfig) -> Result<ScanReport, NamingScanError> {
    let inputs = discover_scan_inputs(root.clone(), &config)?;
    let selected_input_count = inputs.len();
    let mut findings = Vec::new();
    let mut scanned_text_input_count = 0usize;
    for input in inputs {
        scanned_text_input_count = scanned_text_input_count
            .checked_add(1)
            .ok_or_else(count_error)?;
        findings.extend(scan_file(input, &config)?);
    }
    findings.sort_by(compare_finding);
    if findings.is_empty() {
        Ok(ScanReport {
            root,
            config_fingerprint: config.config_fingerprint,
            selected_input_count,
            scanned_text_input_count,
            findings,
            report_destination: config.report_destination,
        })
    } else {
        Err(NamingScanError::InvalidCanonicalSpelling { findings })
    }
}
