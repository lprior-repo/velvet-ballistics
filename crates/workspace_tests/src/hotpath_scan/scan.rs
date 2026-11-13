use std::io::Write;
use std::path::Path;

use super::allow::Allowances;
use super::{Failure, ROOTS, TOKENS, Token, increment, legacy_whitespace, normalized_lines, walk};

const COLD_PARTS: &[&str] = &[
    "diagnostic",
    "diagnostics",
    "fixture",
    "fixtures",
    "harness",
    "kani",
    "loom",
    "proof",
    "property",
    "proptest",
    "proptests",
    "support",
    "test",
    "tests",
    "verification",
];

pub(super) struct Scanner<'a, W> {
    allowed: Allowances,
    out: &'a mut W,
    classified: usize,
    justified: usize,
    violations: Vec<String>,
}

impl<'a, W: Write> Scanner<'a, W> {
    pub(super) fn new(allowed: Allowances, out: &'a mut W) -> Self {
        Self {
            allowed,
            out,
            classified: 0,
            justified: 0,
            violations: Vec::new(),
        }
    }

    pub(super) fn run(mut self, root: &Path) -> Result<u8, Failure> {
        ROOTS
            .iter()
            .map(|dir| root.join(dir))
            .filter(|dir| dir.exists())
            .try_for_each(|dir| {
                walk::sources(&dir)?
                    .iter()
                    .try_for_each(|file| self.scan_file(root, file))
            })?;
        self.finish()
    }

    fn scan_file(&mut self, root: &Path, file: &Path) -> Result<(), Failure> {
        let rel = file
            .strip_prefix(root)
            .ok()
            .and_then(|p| p.to_str())
            .ok_or_else(|| Failure::InvalidPath(file.to_path_buf()))?;
        increment(&mut self.classified)?;
        let cold = is_cold(rel);
        writeln!(
            self.out,
            "ClassifiedPath|{}|{rel}",
            if cold { "cold" } else { "hot" }
        )?;
        if cold {
            return Ok(());
        }
        let text = normalized_lines(&std::fs::read_to_string(file)?);
        text.split_terminator('\n')
            .enumerate()
            .try_for_each(|(index, raw)| {
                let number = index.checked_add(1).ok_or(Failure::CountOverflow)?;
                self.scan_line(rel, number, raw)
            })
    }

    fn scan_line(&mut self, rel: &str, number: usize, raw: &str) -> Result<(), Failure> {
        let text = raw.split_once("//").map_or(raw, |(before, _)| before);
        TOKENS
            .iter()
            .copied()
            .filter(|token| text.contains(token.text()))
            .try_for_each(|token| self.record_match(rel, number, text, token))
    }

    fn record_match(
        &mut self,
        rel: &str,
        number: usize,
        text: &str,
        token: Token,
    ) -> Result<(), Failure> {
        let name = token.text();
        if self.allowed.contains(rel, token) {
            increment(&mut self.justified)?;
            writeln!(self.out, "JustifiedException|{name}|{rel}|line={number}")?;
        } else {
            let text = text
                .split(legacy_whitespace)
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            self.violations
                .push(format!("ViolationFound|{name}|{rel}|line={number}|{text}"));
        }
        Ok(())
    }

    fn finish(self) -> Result<u8, Failure> {
        self.violations
            .iter()
            .try_for_each(|line| writeln!(self.out, "{line}"))?;
        writeln!(
            self.out,
            "ScanSummary|hot_roots=vb_core,vb_runtime,vb_storage,vb_ipc|classified={}|violations={}|justified={}",
            self.classified,
            self.violations.len(),
            self.justified
        )?;
        self.out.flush()?;
        Ok(if self.violations.is_empty() { 0 } else { 2 })
    }
}

fn is_cold(rel: &str) -> bool {
    rel.split(|ch: char| matches!(ch, '/' | '.' | '_' | '-') || legacy_whitespace(ch))
        .any(|part| COLD_PARTS.contains(&part))
}
