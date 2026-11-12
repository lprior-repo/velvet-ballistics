use std::path::PathBuf;

#[derive(Debug)]
pub struct ScanResult {
    pub pattern: String,
    pub description: String,
    pub file: PathBuf,
    pub line: usize,
    pub line_content: String,
}
#[derive(Debug)]
pub struct ScanSummary {
    pub total_findings: usize,
    pub findings_by_pattern: std::collections::HashMap<String, usize>,
    pub files_with_findings: usize,
    pub crates_scanned: Vec<String>,
}

impl ScanSummary {
    pub fn new() -> Self {
        Self {
            total_findings: 0,
            findings_by_pattern: std::collections::HashMap::new(),
            files_with_findings: 0,
            crates_scanned: Vec::new(),
        }
    }

    pub fn add_finding(&mut self, pattern: &str) {
        self.total_findings = self.total_findings.saturating_add(1);
        let count = self
            .findings_by_pattern
            .entry(pattern.to_string())
            .or_insert(0);
        *count = count.saturating_add(1);
    }
}
