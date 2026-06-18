// SPDX-License-Identifier: MIT
#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::dbg_macro
)]

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::convert::TryFrom;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ForbiddenImportName {
    SerdeJson,
    SerdeYaml,
    Hyper,
    Reqwest,
    Axum,
    HashMapStringGeneric,
    TokioSyncMpscUnbounded,
}

impl ForbiddenImportName {
    fn all() -> [Self; 7] {
        [
            Self::SerdeJson,
            Self::SerdeYaml,
            Self::Hyper,
            Self::Reqwest,
            Self::Axum,
            Self::HashMapStringGeneric,
            Self::TokioSyncMpscUnbounded,
        ]
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::SerdeJson => "serde_json",
            Self::SerdeYaml => "serde_yaml",
            Self::Hyper => "hyper",
            Self::Reqwest => "reqwest",
            Self::Axum => "axum",
            Self::HashMapStringGeneric => "HashMap<String,_>",
            Self::TokioSyncMpscUnbounded => "tokio::sync::mpsc::unbounded",
        }
    }

    fn from_allowlist(value: &str) -> Result<Self, String> {
        match value {
            "serde_json" => Ok(Self::SerdeJson),
            "serde_yaml" => Ok(Self::SerdeYaml),
            "hyper" => Ok(Self::Hyper),
            "reqwest" => Ok(Self::Reqwest),
            "axum" => Ok(Self::Axum),
            "HashMap<String,_>" => Ok(Self::HashMapStringGeneric),
            "tokio::sync::mpsc::unbounded" => Ok(Self::TokioSyncMpscUnbounded),
            other => Err(format!("unknown forbidden name '{other}'")),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum ForbiddenImportKind {
    CrateName,
    PathToken,
    TypeExpression,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct MasterRef {
    section: u32,
    line: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct ForbiddenImport {
    name: ForbiddenImportName,
    kind: ForbiddenImportKind,
    master_ref: MasterRef,
}

impl ForbiddenImport {
    fn from_name(name: ForbiddenImportName) -> Self {
        match name {
            ForbiddenImportName::SerdeJson => {
                Self::new(name, ForbiddenImportKind::CrateName, 43, 2058)
            }
            ForbiddenImportName::SerdeYaml => {
                Self::new(name, ForbiddenImportKind::CrateName, 43, 2057)
            }
            ForbiddenImportName::Hyper => Self::new(name, ForbiddenImportKind::CrateName, 43, 2059),
            ForbiddenImportName::Reqwest => {
                Self::new(name, ForbiddenImportKind::CrateName, 43, 2059)
            }
            ForbiddenImportName::Axum => Self::new(name, ForbiddenImportKind::CrateName, 43, 2059),
            ForbiddenImportName::HashMapStringGeneric => {
                Self::new(name, ForbiddenImportKind::TypeExpression, 43, 2060)
            }
            ForbiddenImportName::TokioSyncMpscUnbounded => {
                Self::new(name, ForbiddenImportKind::PathToken, 43, 2056)
            }
        }
    }

    fn new(name: ForbiddenImportName, kind: ForbiddenImportKind, section: u32, line: u32) -> Self {
        Self {
            name,
            kind,
            master_ref: MasterRef { section, line },
        }
    }

    fn matches_line(self, code: &str, compact: &str) -> bool {
        match self.kind {
            ForbiddenImportKind::CrateName => code.contains(self.name.as_str()),
            ForbiddenImportKind::PathToken => matches_tokio_mpsc_unbounded(compact),
            ForbiddenImportKind::TypeExpression => compact.contains("HashMap<String,"),
        }
    }

    fn has_valid_master_ref(self) -> bool {
        self.master_ref.section > 0 && self.master_ref.line > 0
    }
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum HotCrateName {
    VbCore,
    VbRuntime,
    VbStorage,
    VbIpC,
}

impl HotCrateName {
    fn all() -> [Self; 4] {
        [Self::VbCore, Self::VbRuntime, Self::VbStorage, Self::VbIpC]
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::VbCore => "vb_core",
            Self::VbRuntime => "vb_runtime",
            Self::VbStorage => "vb_storage",
            Self::VbIpC => "vb_ipc",
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum ColdMarker {
    Diagnostic,
    Diagnostics,
    Fixture,
    Fixtures,
    Harness,
    Kani,
    Loom,
    Proof,
    Property,
    Proptest,
    Proptests,
    Support,
    TestUtil,
    Tests,
    Verification,
}

impl ColdMarker {
    fn all() -> [Self; 15] {
        [
            Self::Diagnostic,
            Self::Diagnostics,
            Self::Fixture,
            Self::Fixtures,
            Self::Harness,
            Self::Kani,
            Self::Loom,
            Self::Proof,
            Self::Property,
            Self::Proptest,
            Self::Proptests,
            Self::Support,
            Self::TestUtil,
            Self::Tests,
            Self::Verification,
        ]
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Diagnostic => "diagnostic",
            Self::Diagnostics => "diagnostics",
            Self::Fixture => "fixture",
            Self::Fixtures => "fixtures",
            Self::Harness => "harness",
            Self::Kani => "kani",
            Self::Loom => "loom",
            Self::Proof => "proof",
            Self::Property => "property",
            Self::Proptest => "proptest",
            Self::Proptests => "proptests",
            Self::Support => "support",
            Self::TestUtil => "test_util",
            Self::Tests => "tests",
            Self::Verification => "verification",
        }
    }
}

#[derive(Debug)]
enum GateError {
    PatternFileMissing(String),
    GlobUnreadable { path: String, os_error: String },
    AllowlistParseFailure { line: u32, reason: String },
    ScriptInvocationFailure(String),
    NewResidueDetected,
}

impl GateError {
    fn exit_code(&self) -> ExitCode {
        match self {
            Self::NewResidueDetected => ExitCode::from(1),
            Self::PatternFileMissing(_)
            | Self::GlobUnreadable { .. }
            | Self::AllowlistParseFailure { .. }
            | Self::ScriptInvocationFailure(_) => ExitCode::from(2),
        }
    }

    fn emit(&self) {
        match self {
            Self::PatternFileMissing(name) => eprintln!("GateError:PatternFileMissing: {name}"),
            Self::GlobUnreadable { path, os_error } => {
                eprintln!("GateError:GlobUnreadable: {path}: {os_error}")
            }
            Self::AllowlistParseFailure { line, reason } => {
                eprintln!("GateError:AllowlistParseFailure: line {line}: {reason}")
            }
            Self::ScriptInvocationFailure(reason) => {
                eprintln!("GateError:ScriptInvocationFailure: {reason}")
            }
            Self::NewResidueDetected => {}
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ResidueMatch {
    file: String,
    line_no: u32,
    forbidden: ForbiddenImportName,
    snippet: String,
}

impl ResidueMatch {
    fn key(&self) -> AllowlistKey {
        AllowlistKey {
            file: self.file.clone(),
            line_no: self.line_no,
            forbidden: self.forbidden,
        }
    }

    fn active_line(&self) -> String {
        format!(
            "{}:{}: RUNTIME-FMT: {}: {}",
            self.file,
            self.line_no,
            self.forbidden.as_str(),
            self.snippet
        )
    }

    fn allowlisted_line(&self, entry: &AllowlistEntry) -> String {
        format!(
            "{}:{}: allowlisted: {}: {}",
            self.file,
            self.line_no,
            entry.reason(),
            self.snippet
        )
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct AllowlistKey {
    file: String,
    line_no: u32,
    forbidden: ForbiddenImportName,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct AllowlistEntry {
    owner: String,
    reviewed_by: String,
    test: String,
    reason: String,
}

impl AllowlistEntry {
    fn new(owner: &str, reviewed_by: &str, test: &str, reason: &str) -> Result<Self, String> {
        if owner.trim().is_empty()
            || reviewed_by.trim().is_empty()
            || test.trim().is_empty()
            || reason.trim().is_empty()
        {
            return Err("owner, reviewed_by, test, and reason must be non-empty".to_owned());
        }
        Ok(Self {
            owner: owner.to_owned(),
            reviewed_by: reviewed_by.to_owned(),
            test: test.to_owned(),
            reason: reason.to_owned(),
        })
    }

    fn is_complete(&self) -> bool {
        !self.owner.is_empty()
            && !self.reviewed_by.is_empty()
            && !self.test.is_empty()
            && !self.reason.is_empty()
    }

    fn reason(&self) -> &str {
        &self.reason
    }
}

struct AllowlistRef {
    entries: BTreeMap<AllowlistKey, AllowlistEntry>,
}

impl AllowlistRef {
    fn load(path: &Path) -> Result<Self, GateError> {
        if !path.exists() {
            return Ok(Self {
                entries: BTreeMap::new(),
            });
        }
        let text = fs::read_to_string(path).map_err(|error| GateError::AllowlistParseFailure {
            line: 0,
            reason: format!("{}: unreadable: {error}", path.display()),
        })?;
        let mut entries = BTreeMap::new();
        for (index, raw_line) in text.lines().enumerate() {
            let line_no = checked_line_no(index)
                .map_err(|reason| GateError::AllowlistParseFailure { line: 0, reason })?;
            let trimmed = raw_line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (key, entry) = parse_allowlist_line(trimmed, line_no)?;
            insert_allowlist_entry(&mut entries, key, entry, line_no)?;
        }
        Ok(Self { entries })
    }

    fn lookup(&self, key: &AllowlistKey) -> Option<&AllowlistEntry> {
        self.entries.get(key)
    }
}

fn checked_line_no(index: usize) -> Result<u32, String> {
    let one_based = index
        .checked_add(1)
        .ok_or_else(|| "line number overflow".to_owned())?;
    u32::try_from(one_based).map_err(|error| format!("line number overflow: {error}"))
}

fn next_part<'a>(parts: &mut std::str::Split<'a, char>, line: u32) -> Result<&'a str, GateError> {
    parts
        .next()
        .ok_or_else(|| GateError::AllowlistParseFailure {
            line,
            reason: "expected 7 pipe-separated fields".to_owned(),
        })
}

struct AllowlistLineParts<'a> {
    file: &'a str,
    line_value: &'a str,
    forbidden_value: &'a str,
    owner: &'a str,
    reviewed_by: &'a str,
    test: &'a str,
    reason: &'a str,
}

fn parse_allowlist_line(
    line_text: &str,
    line_no: u32,
) -> Result<(AllowlistKey, AllowlistEntry), GateError> {
    let parts = allowlist_line_parts(line_text, line_no)?;
    let key = parse_allowlist_key(&parts, line_no)?;
    let entry = parse_allowlist_entry(&parts, line_no)?;
    Ok((key, entry))
}

fn allowlist_line_parts<'a>(
    line_text: &'a str,
    line_no: u32,
) -> Result<AllowlistLineParts<'a>, GateError> {
    let mut parts = line_text.split('|');
    let parsed = AllowlistLineParts {
        file: next_part(&mut parts, line_no)?,
        line_value: next_part(&mut parts, line_no)?,
        forbidden_value: next_part(&mut parts, line_no)?,
        owner: next_part(&mut parts, line_no)?,
        reviewed_by: next_part(&mut parts, line_no)?,
        test: next_part(&mut parts, line_no)?,
        reason: next_part(&mut parts, line_no)?,
    };
    if parts.next().is_some() {
        return Err(GateError::AllowlistParseFailure {
            line: line_no,
            reason: "expected 7 pipe-separated fields".to_owned(),
        });
    }
    Ok(parsed)
}

fn parse_allowlist_key(
    parts: &AllowlistLineParts<'_>,
    line_no: u32,
) -> Result<AllowlistKey, GateError> {
    let parsed_line =
        parts
            .line_value
            .parse::<u32>()
            .map_err(|error| GateError::AllowlistParseFailure {
                line: line_no,
                reason: format!("invalid line number '{}': {error}", parts.line_value),
            })?;
    let forbidden =
        ForbiddenImportName::from_allowlist(parts.forbidden_value).map_err(|reason| {
            GateError::AllowlistParseFailure {
                line: line_no,
                reason,
            }
        })?;
    Ok(AllowlistKey {
        file: parts.file.to_owned(),
        line_no: parsed_line,
        forbidden,
    })
}

fn parse_allowlist_entry(
    parts: &AllowlistLineParts<'_>,
    line_no: u32,
) -> Result<AllowlistEntry, GateError> {
    AllowlistEntry::new(parts.owner, parts.reviewed_by, parts.test, parts.reason).map_err(
        |reason| GateError::AllowlistParseFailure {
            line: line_no,
            reason,
        },
    )
}

fn insert_allowlist_entry(
    entries: &mut BTreeMap<AllowlistKey, AllowlistEntry>,
    key: AllowlistKey,
    entry: AllowlistEntry,
    line_no: u32,
) -> Result<(), GateError> {
    match entries.entry(key) {
        Entry::Vacant(slot) => {
            let inserted = slot.insert(entry);
            if inserted.is_complete() {
                Ok(())
            } else {
                Err(GateError::AllowlistParseFailure {
                    line: line_no,
                    reason: "incomplete allowlist metadata".to_owned(),
                })
            }
        }
        Entry::Occupied(occupied) => Err(GateError::AllowlistParseFailure {
            line: line_no,
            reason: format!("duplicate key {}", allowlist_key_display(occupied.key())),
        }),
    }
}

fn allowlist_key_display(key: &AllowlistKey) -> String {
    format!("{}|{}|{}", key.file, key.line_no, key.forbidden.as_str())
}

#[derive(Clone, Debug)]
struct SourceFile {
    absolute: PathBuf,
    relative: String,
}

struct ResiduePolicy {
    forbidden: Vec<ForbiddenImport>,
    crates: Vec<HotCrateName>,
    cold_markers: Vec<ColdMarker>,
}

impl ResiduePolicy {
    fn from_master(master_path: &Path) -> Result<Self, GateError> {
        let master = fs::read_to_string(master_path)
            .map_err(|_error| GateError::PatternFileMissing("serde_json".to_owned()))?;
        let forbidden = ForbiddenImportName::all()
            .into_iter()
            .map(ForbiddenImport::from_name)
            .collect::<Vec<_>>();
        validate_master_forbidden_set(&master, &forbidden)?;
        let crates = HotCrateName::all().into_iter().collect::<Vec<_>>();
        validate_master_hot_crates(&master, &crates)?;
        Ok(Self {
            forbidden,
            crates,
            cold_markers: ColdMarker::all().into_iter().collect::<Vec<_>>(),
        })
    }
}

fn validate_master_forbidden_set(
    master: &str,
    forbidden: &[ForbiddenImport],
) -> Result<(), GateError> {
    for import in forbidden {
        if !master_supports_import(master, *import) {
            return Err(GateError::PatternFileMissing(
                import.name.as_str().to_owned(),
            ));
        }
    }
    Ok(())
}

fn master_supports_import(master: &str, import: ForbiddenImport) -> bool {
    import.has_valid_master_ref()
        && import.master_ref.section == 43
        && master_line_matches(
            master,
            import.master_ref.line,
            expected_master_trigger(import.name),
        )
}

fn master_line_matches(master: &str, line_no: u32, expected: &str) -> bool {
    let Some(zero_based) = line_no.checked_sub(1) else {
        return false;
    };
    let Ok(index) = usize::try_from(zero_based) else {
        return false;
    };
    master
        .lines()
        .nth(index)
        .is_some_and(|line| line.trim() == expected)
}

fn expected_master_trigger(name: ForbiddenImportName) -> &'static str {
    match name {
        ForbiddenImportName::SerdeJson => "JSON inserted into runtime core",
        ForbiddenImportName::SerdeYaml => "YAML interpreted at runtime",
        ForbiddenImportName::Hyper | ForbiddenImportName::Reqwest | ForbiddenImportName::Axum => {
            "HTTP inserted into runtime core"
        }
        ForbiddenImportName::HashMapStringGeneric => "HashMap<String, Value> runtime state",
        ForbiddenImportName::TokioSyncMpscUnbounded => "unbounded queue/loop/retry/fanout",
    }
}

fn validate_master_hot_crates(master: &str, crates: &[HotCrateName]) -> Result<(), GateError> {
    for crate_name in crates {
        if !master.contains(crate_name.as_str()) {
            return Err(GateError::PatternFileMissing(
                crate_name.as_str().to_owned(),
            ));
        }
    }
    Ok(())
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum ResidueQuarantineState {
    Init,
    Loaded,
    Walked,
    Matched,
    Differed,
    Done,
}

struct ScanReport {
    files_scanned: u32,
    hot_paths_count: u32,
    cold_paths_count: u32,
    total_matches: u32,
    active: Vec<ResidueMatch>,
    allowlisted: Vec<(ResidueMatch, AllowlistEntry)>,
}

impl ScanReport {
    fn new() -> Self {
        Self {
            files_scanned: 0,
            hot_paths_count: 0,
            cold_paths_count: 0,
            total_matches: 0,
            active: Vec::new(),
            allowlisted: Vec::new(),
        }
    }

    fn increment_files_scanned(&mut self) -> Result<(), GateError> {
        increment_counter(&mut self.files_scanned, "files_scanned")
    }

    fn increment_hot_paths(&mut self) -> Result<(), GateError> {
        increment_counter(&mut self.hot_paths_count, "hot_paths")
    }

    fn increment_cold_paths(&mut self) -> Result<(), GateError> {
        increment_counter(&mut self.cold_paths_count, "cold_paths")
    }

    fn increment_total_matches(&mut self) -> Result<(), GateError> {
        increment_counter(&mut self.total_matches, "total_matches")
    }

    fn summary_line(&self) -> String {
        format!(
            "summary: active={} allowlisted={} files_scanned={} hot_paths={} cold_paths={}",
            self.active.len(),
            self.allowlisted.len(),
            self.files_scanned,
            self.hot_paths_count,
            self.cold_paths_count
        )
    }
}

fn increment_counter(value: &mut u32, label: &'static str) -> Result<(), GateError> {
    let next = value
        .checked_add(1)
        .ok_or_else(|| GateError::ScriptInvocationFailure(format!("counter overflow: {label}")))?;
    *value = next;
    Ok(())
}

enum GateDecision {
    Pass(ScanReport),
    Fail(ScanReport),
}

impl GateDecision {
    fn exit_code(&self) -> ExitCode {
        match self {
            Self::Pass(_) => ExitCode::SUCCESS,
            Self::Fail(_) => GateError::NewResidueDetected.exit_code(),
        }
    }

    fn emit(&self) {
        match self {
            Self::Pass(report) => emit_pass(report),
            Self::Fail(report) => emit_fail(report),
        }
    }
}

fn emit_pass(report: &ScanReport) {
    for (residue, entry) in &report.allowlisted {
        eprintln!("{}", residue.allowlisted_line(entry));
    }
    println!("{}", report.summary_line());
}

fn emit_fail(report: &ScanReport) {
    for (residue, entry) in &report.allowlisted {
        eprintln!("{}", residue.allowlisted_line(entry));
    }
    for residue in &report.active {
        eprintln!("{}", residue.active_line());
    }
    eprintln!("{}", report.summary_line());
}

struct ResidueQuarantine {
    policy: ResiduePolicy,
    source_root: PathBuf,
    allowlist: AllowlistRef,
    report: ScanReport,
    hot_paths: Vec<SourceFile>,
    matches: Vec<ResidueMatch>,
    state: ResidueQuarantineState,
}

impl ResidueQuarantine {
    fn run(source_root: PathBuf) -> Result<GateDecision, GateError> {
        let master_path = source_root.join("velvet-ballistics-MASTER.md");
        let policy = ResiduePolicy::from_master(&master_path)?;
        let allowlist_path = source_root.join("scripts/forbid-runtime-fmt.allow");
        let allowlist = AllowlistRef::load(&allowlist_path)?;
        let mut quarantine = Self::init(policy, source_root, allowlist)?;
        quarantine.walk()?;
        quarantine.match_lines()?;
        quarantine.diff_against_allowlist()?;
        Ok(quarantine.decide())
    }

    fn init(
        policy: ResiduePolicy,
        source_root: PathBuf,
        allowlist: AllowlistRef,
    ) -> Result<Self, GateError> {
        if !source_root.is_dir() {
            return Err(GateError::GlobUnreadable {
                path: source_root.display().to_string(),
                os_error: "source root is not a directory".to_owned(),
            });
        }
        Ok(Self {
            policy,
            source_root,
            allowlist,
            report: ScanReport::new(),
            hot_paths: Vec::new(),
            matches: Vec::new(),
            state: ResidueQuarantineState::Init,
        })
        .map(|mut quarantine| {
            quarantine.state = ResidueQuarantineState::Loaded;
            quarantine
        })
    }

    fn walk(&mut self) -> Result<(), GateError> {
        require_state(self.state, ResidueQuarantineState::Loaded)?;
        let crates = self.policy.crates.clone();
        let mut files = Vec::new();
        for crate_name in crates {
            let relative_root = format!("crates/{}/src", crate_name.as_str());
            let crate_root = self.source_root.join(&relative_root);
            collect_rust_files(&crate_root, &relative_root, &mut files)?;
        }
        files.sort_by(|left, right| left.relative.cmp(&right.relative));
        for source in files {
            self.report.increment_files_scanned()?;
            if is_cold_path(&source.relative, &self.policy.cold_markers) {
                self.report.increment_cold_paths()?;
            } else {
                self.report.increment_hot_paths()?;
                self.hot_paths.push(source);
            }
        }
        self.state = ResidueQuarantineState::Walked;
        Ok(())
    }

    fn match_lines(&mut self) -> Result<(), GateError> {
        require_state(self.state, ResidueQuarantineState::Walked)?;
        let hot_paths = self.hot_paths.clone();
        for source in hot_paths {
            let text = fs::read_to_string(&source.absolute).map_err(|error| {
                GateError::GlobUnreadable {
                    path: source.relative.clone(),
                    os_error: error.to_string(),
                }
            })?;
            for (index, raw_line) in text.lines().enumerate() {
                let line_no = checked_line_no(index).map_err(GateError::ScriptInvocationFailure)?;
                let findings =
                    classify_line(&source.relative, line_no, raw_line, &self.policy.forbidden);
                for finding in findings {
                    self.report.increment_total_matches()?;
                    self.matches.push(finding);
                }
            }
        }
        self.state = ResidueQuarantineState::Matched;
        Ok(())
    }

    fn diff_against_allowlist(&mut self) -> Result<(), GateError> {
        require_state(self.state, ResidueQuarantineState::Matched)?;
        let mut active = Vec::new();
        let mut allowlisted = Vec::new();
        let matches = std::mem::take(&mut self.matches);
        for residue in matches {
            let key = residue.key();
            if let Some(entry) = self.allowlist.lookup(&key) {
                allowlisted.push((residue, entry.clone()));
            } else {
                active.push(residue);
            }
        }
        active.sort();
        allowlisted.sort();
        self.report.active = active;
        self.report.allowlisted = allowlisted;
        self.state = ResidueQuarantineState::Differed;
        Ok(())
    }

    fn decide(mut self) -> GateDecision {
        self.state = ResidueQuarantineState::Done;
        if self.report.active.is_empty() {
            GateDecision::Pass(self.report)
        } else {
            GateDecision::Fail(self.report)
        }
    }
}

fn require_state(
    actual: ResidueQuarantineState,
    expected: ResidueQuarantineState,
) -> Result<(), GateError> {
    if actual == expected {
        Ok(())
    } else {
        Err(GateError::ScriptInvocationFailure(format!(
            "state mismatch: expected {expected:?}, got {actual:?}"
        )))
    }
}

fn collect_rust_files(
    root: &Path,
    relative_root: &str,
    files: &mut Vec<SourceFile>,
) -> Result<(), GateError> {
    if !root.is_dir() {
        return Err(GateError::GlobUnreadable {
            path: relative_root.to_owned(),
            os_error: "not a directory".to_owned(),
        });
    }
    walk_directory(root, relative_root, files)
}

fn walk_directory(
    dir: &Path,
    relative: &str,
    files: &mut Vec<SourceFile>,
) -> Result<(), GateError> {
    let entries = read_directory(dir, relative)?;
    for entry_result in entries {
        if let Some(child) = directory_child(entry_result, relative)? {
            collect_directory_child(child, files)?;
        }
    }
    Ok(())
}

fn read_directory(dir: &Path, relative: &str) -> Result<fs::ReadDir, GateError> {
    fs::read_dir(dir).map_err(|error| GateError::GlobUnreadable {
        path: relative.to_owned(),
        os_error: error.to_string(),
    })
}

struct DirectoryChild {
    path: PathBuf,
    relative: String,
    file_type: fs::FileType,
}

fn directory_child(
    entry_result: Result<fs::DirEntry, std::io::Error>,
    relative: &str,
) -> Result<Option<DirectoryChild>, GateError> {
    let entry = entry_result.map_err(|error| GateError::GlobUnreadable {
        path: relative.to_owned(),
        os_error: error.to_string(),
    })?;
    let name = entry.file_name();
    let Some(name_text) = name.to_str() else {
        return Ok(None);
    };
    let child_relative = join_relative(relative, name_text);
    let file_type = entry
        .file_type()
        .map_err(|error| GateError::GlobUnreadable {
            path: child_relative.clone(),
            os_error: error.to_string(),
        })?;
    Ok(Some(DirectoryChild {
        path: entry.path(),
        relative: child_relative,
        file_type,
    }))
}

fn collect_directory_child(
    child: DirectoryChild,
    files: &mut Vec<SourceFile>,
) -> Result<(), GateError> {
    if child.file_type.is_dir() {
        walk_directory(&child.path, &child.relative, files)?;
    } else if child.file_type.is_file() && is_rust_source(&child.path) {
        files.push(SourceFile {
            absolute: child.path,
            relative: child.relative,
        });
    }
    Ok(())
}

fn join_relative(parent: &str, child: &str) -> String {
    if parent.is_empty() {
        child.to_owned()
    } else {
        format!("{parent}/{child}")
    }
}

fn is_rust_source(path: &Path) -> bool {
    path.extension().and_then(|extension| extension.to_str()) == Some("rs")
}

fn is_cold_path(relative: &str, markers: &[ColdMarker]) -> bool {
    relative.split('/').any(|component| {
        markers
            .iter()
            .any(|marker| component.contains(marker.as_str()))
    })
}

fn classify_line(
    file: &str,
    line_no: u32,
    raw_line: &str,
    imports: &[ForbiddenImport],
) -> Vec<ResidueMatch> {
    let code = without_comment(raw_line).trim();
    if code.is_empty() {
        return Vec::new();
    }
    let compact = remove_whitespace(code);
    let snippet = snippet(raw_line);
    imports
        .iter()
        .filter_map(|import| match_for_import(file, line_no, &snippet, code, &compact, *import))
        .collect()
}

fn match_for_import(
    file: &str,
    line_no: u32,
    snippet: &str,
    code: &str,
    compact: &str,
    import: ForbiddenImport,
) -> Option<ResidueMatch> {
    import.matches_line(code, compact).then(|| ResidueMatch {
        file: file.to_owned(),
        line_no,
        forbidden: import.name,
        snippet: snippet.to_owned(),
    })
}

fn without_comment(line: &str) -> &str {
    match line.split_once("//") {
        Some((before, _comment)) => before,
        None => line,
    }
}

fn remove_whitespace(line: &str) -> String {
    line.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn matches_tokio_mpsc_unbounded(compact: &str) -> bool {
    compact.contains("tokio::sync::mpsc::unbounded")
        || grouped_tokio_mpsc_import_contains_unbounded(compact)
}

fn grouped_tokio_mpsc_import_contains_unbounded(compact: &str) -> bool {
    compact
        .split("tokio::sync::mpsc::{")
        .skip(1)
        .filter_map(|tail| tail.split('}').next())
        .any(|group| group.split(',').any(|item| item.starts_with("unbounded")))
}

fn snippet(line: &str) -> String {
    line.trim().chars().take(120).collect()
}

fn resolve_source_root() -> Result<PathBuf, GateError> {
    let mut args = std::env::args_os().skip(1);
    match args.next() {
        Some(root) => {
            if args.next().is_some() {
                Err(GateError::ScriptInvocationFailure(
                    "expected at most one source-root argument".to_owned(),
                ))
            } else {
                Ok(PathBuf::from(root))
            }
        }
        None => std::env::current_dir()
            .map_err(|error| GateError::ScriptInvocationFailure(error.to_string())),
    }
}

fn run() -> Result<GateDecision, GateError> {
    let source_root = resolve_source_root()?;
    ResidueQuarantine::run(source_root)
}

fn main() -> ExitCode {
    match run() {
        Ok(decision) => {
            decision.emit();
            decision.exit_code()
        }
        Err(error) => {
            error.emit();
            error.exit_code()
        }
    }
}
