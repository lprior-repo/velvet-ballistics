use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const HOT_CRATES: &[&str] = &["vb_core", "vb_runtime", "vb_storage", "vb_ipc"];
const COLD_MARKERS: &[&str] = &[
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
    "test_util",
    "tests",
    "verification",
];

const HOT_FIXTURE: &str = concat!(
    "pub fn bad_print_map_unbounded() { println!(\"x\"); let _m: HashMap<String, u8> = HashMap::new(); let _c = std::sync::mpsc::channel(); }\n",
    "pub fn bad_sync_plain() { let _sync = std::sync::mpsc::sync_channel(8); }\n",
    "pub fn bad_sync_turbofish() { let _sync = std::sync::mpsc::sync_channel::<u8>(8); }\n",
    "pub fn bad_mpsc_plain() { let _sync = mpsc::sync_channel(8); }\n",
    "pub fn bad_mpsc_turbofish() { let _sync = mpsc::sync_channel::<u8>(8); }\n",
    "pub fn bad_mutex_vecdeque() { let _q: Mutex<VecDeque<u8>> = make_queue(); }\n",
    "pub fn bad_mutex_std_vecdeque() { let _q: Mutex<std::collections::VecDeque<u8>> = make_queue(); }\n",
    "pub fn bad_std_mutex_vecdeque() { let _q: std::sync::Mutex<VecDeque<u8>> = make_queue(); }\n",
    "pub fn bad_std_mutex_std_vecdeque() { let _q: std::sync::Mutex<std::collections::VecDeque<u8>> = make_queue(); }\n",
);

const BOUNDED_STATE_FIXTURE: &str = concat!(
    "pub struct BoundedState { pending: std::collections::VecDeque<u8>, capacity: usize }\n",
    "pub struct Queue { state: std::sync::Mutex<BoundedState> }\n",
    "pub fn ok() { let _queue = crossbeam_queue::ArrayQueue::<u8>::new(8); let _state = BoundedState { pending: std::collections::VecDeque::with_capacity(8), capacity: 8 }; }\n",
);

const TOKEN_BOUNDARY_FIXTURE: &str = concat!(
    "pub fn ok_sync_name() { let _sync = not_mpsc::sync_channel(8); }\n",
    "pub fn ok_mutex_name() { let _q: NotAMutex<VecDeque<u8>> = make_queue(); }\n",
    "pub fn ok_parking_lot() { let _q: parking_lot::Mutex<VecDeque<u8>> = make_queue(); }\n",
);

const TEST_MODULE_FIXTURE: &str = concat!(
    "#[cfg(test)] mod same_line_tests {\n",
    "pub fn ignored_same_line() { let _sync = mpsc::sync_channel(8); }\n",
    "}\n",
    "#[cfg(test)]\n",
    "mod next_line_tests\n",
    "{\n",
    "pub fn ignored_next_line() { let _q: Mutex<VecDeque<u8>> = make_queue(); }\n",
    "}\n",
    "#[cfg(test)] mod external_tests;\n",
    "pub fn external_does_not_over_skip() { let _sync = mpsc::sync_channel(8); }\n",
);

const VALID_ALLOWLIST: &str = concat!(
    "crates/vb_runtime/src/engine.rs|CHANNEL-BOUNDED-001|owner=self-test|reviewed_by=self-test|test=self-test|reason=synthetic bounded channel exception\n",
    "crates/vb_runtime/src/engine.rs|QUEUE-MUTEX-VECDEQUE-001|owner=self-test|reviewed_by=self-test|test=self-test|reason=synthetic bounded capacity proof\n",
);

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Finding {
    rel_path: String,
    line_no: usize,
    class_id: &'static str,
    text: String,
}

struct AllowEntry<'a> {
    path: &'a str,
    class: &'a str,
    owner: &'a str,
    reviewed_by: &'a str,
    test: &'a str,
    reason: &'a str,
}

#[derive(Default)]
struct ScanAccumulator {
    classified: Vec<String>,
    violations: Vec<Finding>,
    justified: Vec<Finding>,
}

#[derive(Copy, Clone)]
enum TestSkipState {
    Normal,
    CfgPending,
    AwaitingBody,
    InBody(usize),
}

struct SelfTestCounts {
    classes: BTreeSet<&'static str>,
    bounded_channel_count: usize,
    mutex_vecdeque_count: usize,
    test_module_channel_count: usize,
    test_module_mutex_count: usize,
}

fn without_comment(line: &str) -> &str {
    line.split_once("//")
        .map_or(line, |(prefix, _comment)| prefix)
}

fn compact(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn remove_spaces(line: &str) -> String {
    line.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn is_cold_path(path: &str) -> bool {
    path.split(['/', '.', '_', '-'])
        .any(|token| COLD_MARKERS.iter().any(|marker| token == *marker))
}

fn is_path_token_char(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn two_chars_before_byte(text: &str, byte_index: usize) -> (Option<char>, Option<char>) {
    let mut previous = None;
    let mut current = None;
    for (index, ch) in text.char_indices() {
        if index >= byte_index {
            break;
        }
        previous = current;
        current = Some(ch);
    }
    (previous, current)
}

fn starts_after_path_separator(previous: Option<char>, current: Option<char>) -> bool {
    current == Some(':') && previous == Some(':')
}

fn valid_path_token_start(text: &str, byte_index: usize) -> bool {
    let (previous, current) = two_chars_before_byte(text, byte_index);
    match current {
        Some(ch) if is_path_token_char(ch) => false,
        Some(':') => !starts_after_path_separator(previous, current),
        Some(_ch) => true,
        None => true,
    }
}

fn has_path_token_start(text: &str, needle: &str) -> bool {
    text.match_indices(needle)
        .any(|(index, _matched)| valid_path_token_start(text, index))
}

fn has_any_path_token_start(text: &str, needles: &[&str]) -> bool {
    needles
        .iter()
        .any(|needle| has_path_token_start(text, needle))
}

fn line_has_string_map(line: &str) -> bool {
    let normalized = remove_spaces(line);
    [
        "HashMap<String",
        "HashMap<&str",
        "BTreeMap<String",
        "BTreeMap<&str",
        "IndexMap<String",
        "IndexMap<&str",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn line_has_forbidden_sync_channel(line: &str) -> bool {
    let normalized = remove_spaces(line);
    has_any_path_token_start(
        &normalized,
        &[
            "std::sync::mpsc::sync_channel(",
            "std::sync::mpsc::sync_channel::<",
            "mpsc::sync_channel(",
            "mpsc::sync_channel::<",
        ],
    )
}

fn line_has_mutex_vecdeque(line: &str) -> bool {
    let normalized = remove_spaces(line);
    has_any_path_token_start(
        &normalized,
        &[
            "Mutex<VecDeque<",
            "Mutex<std::collections::VecDeque<",
            "std::sync::Mutex<VecDeque<",
            "std::sync::Mutex<std::collections::VecDeque<",
        ],
    )
}

fn line_has_print(line: &str) -> bool {
    line.contains("println!(") || line.contains("eprintln!(")
}

fn line_has_json(line: &str) -> bool {
    line.contains("serde_json") || line.contains("serde_json::Value")
}

fn line_has_yaml(line: &str) -> bool {
    line.contains("serde_saphyr") || line.contains("saphyr::") || line.contains(" saphyr")
}

fn line_has_unbounded_channel(line: &str) -> bool {
    line.contains("std::sync::mpsc::channel(")
        || line.contains("mpsc::channel(")
        || line.contains("unbounded_channel(")
        || line.contains("crossbeam_channel::unbounded(")
}

fn is_new_queue_class(class_id: &str) -> bool {
    class_id == "CHANNEL-BOUNDED-001" || class_id == "QUEUE-MUTEX-VECDEQUE-001"
}

fn should_skip_scan_line(line: &str) -> bool {
    line.is_empty() || line.starts_with('#') || line.starts_with("use ")
}

fn findings_for_checks(
    rel_path: &str,
    line_no: usize,
    text: &str,
    checks: &[(&'static str, bool)],
) -> Vec<Finding> {
    checks
        .iter()
        .filter_map(|(class_id, matched)| {
            finding_for_check(rel_path, line_no, text, class_id, *matched)
        })
        .collect()
}

fn finding_for_check(
    rel_path: &str,
    line_no: usize,
    text: &str,
    class_id: &'static str,
    matched: bool,
) -> Option<Finding> {
    matched.then(|| Finding {
        rel_path: rel_path.to_owned(),
        line_no,
        class_id,
        text: text.to_owned(),
    })
}

fn classify_line(rel_path: &str, line_no: usize, raw_line: &str) -> Vec<Finding> {
    let stripped = without_comment(raw_line).trim();
    if should_skip_scan_line(stripped) {
        return Vec::new();
    }
    classify_stripped_line(rel_path, line_no, stripped)
}

fn classify_stripped_line(rel_path: &str, line_no: usize, stripped: &str) -> Vec<Finding> {
    let text = compact(stripped);
    let mut findings = findings_for_checks(rel_path, line_no, &text, &format_checks(stripped));
    findings.extend(findings_for_checks(
        rel_path,
        line_no,
        &text,
        &structure_checks(stripped),
    ));
    findings
}

fn format_checks(stripped: &str) -> [(&'static str, bool); 4] {
    [
        ("FORMAT-PRINT-001", line_has_print(stripped)),
        ("FORMAT-DBG-001", stripped.contains("dbg!(")),
        ("FORMAT-JSON-001", line_has_json(stripped)),
        ("FORMAT-YAML-001", line_has_yaml(stripped)),
    ]
}

fn structure_checks(stripped: &str) -> [(&'static str, bool); 4] {
    [
        ("MAP-STRING-001", line_has_string_map(stripped)),
        (
            "CHANNEL-UNBOUNDED-001",
            line_has_unbounded_channel(stripped),
        ),
        (
            "CHANNEL-BOUNDED-001",
            line_has_forbidden_sync_channel(stripped),
        ),
        (
            "QUEUE-MUTEX-VECDEQUE-001",
            line_has_mutex_vecdeque(stripped),
        ),
    ]
}

fn rust_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    fs::read_dir(root)?.try_fold(Vec::new(), |mut acc, entry| {
        let path = entry?.path();
        if path.is_dir() {
            acc.extend(rust_files(&path)?);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            acc.push(path);
        }
        Ok(acc)
    })
}

fn hot_sources(root: &Path) -> io::Result<Vec<PathBuf>> {
    HOT_CRATES
        .iter()
        .try_fold(Vec::new(), |mut acc, crate_name| {
            let src = root.join("crates").join(crate_name).join("src");
            acc.extend(rust_files(&src)?);
            Ok(acc)
        })
}

fn malformed_allow_entry(line_no: usize) -> String {
    format!(
        "MalformedException: scripts/hot-cold-forbidden-apis.allow:{line_no} expected path|class|owner=...|reviewed_by=...|test=...|reason=..."
    )
}

fn next_allow_part<'a>(
    parts: &mut std::str::Split<'a, char>,
    line_no: usize,
) -> Result<&'a str, String> {
    parts.next().ok_or_else(|| malformed_allow_entry(line_no))
}

fn parse_allow_parts<'a>(trimmed: &'a str, line_no: usize) -> Result<AllowEntry<'a>, String> {
    let mut parts = trimmed.split('|');
    let entry = AllowEntry {
        path: next_allow_part(&mut parts, line_no)?,
        class: next_allow_part(&mut parts, line_no)?,
        owner: next_allow_part(&mut parts, line_no)?,
        reviewed_by: next_allow_part(&mut parts, line_no)?,
        test: next_allow_part(&mut parts, line_no)?,
        reason: next_allow_part(&mut parts, line_no)?,
    };
    if parts.next().is_some() {
        return Err(malformed_allow_entry(line_no));
    }
    Ok(entry)
}

fn validate_allow_path(path: &str, line_no: usize) -> Result<(), String> {
    if path.contains('*') || !path.starts_with("crates/") || !path.ends_with(".rs") {
        return Err(format!(
            "OverbroadException: scripts/hot-cold-forbidden-apis.allow:{line_no} path must be exact crates/*/src/*.rs"
        ));
    }
    Ok(())
}

fn validate_allow_class(class: &str, line_no: usize) -> Result<(), String> {
    if class == "ALL" || class.contains('*') {
        return Err(format!(
            "OverbroadException: scripts/hot-cold-forbidden-apis.allow:{line_no} class must be exact"
        ));
    }
    Ok(())
}

fn metadata_has_value(field: &str, prefix: &str) -> bool {
    field
        .strip_prefix(prefix)
        .map_or(false, |value| !value.trim().is_empty())
}

fn validate_allow_metadata(entry: &AllowEntry<'_>, line_no: usize) -> Result<(), String> {
    let valid = metadata_has_value(entry.owner, "owner=")
        && metadata_has_value(entry.reviewed_by, "reviewed_by=")
        && metadata_has_value(entry.test, "test=")
        && metadata_has_value(entry.reason, "reason=");
    if valid {
        return Ok(());
    }
    Err(format!(
        "MalformedException: scripts/hot-cold-forbidden-apis.allow:{line_no} missing owner/reviewed_by/test/reason"
    ))
}

fn parse_allow_line(index: usize, line: &str) -> Result<Option<(String, String)>, String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return Ok(None);
    }
    let line_no = index.saturating_add(1);
    let entry = parse_allow_parts(trimmed, line_no)?;
    validate_allow_path(entry.path, line_no)?;
    validate_allow_class(entry.class, line_no)?;
    validate_allow_metadata(&entry, line_no)?;
    Ok(Some((entry.path.to_owned(), entry.class.to_owned())))
}

fn insert_allow_entry(
    acc: &mut BTreeSet<(String, String)>,
    entry: (String, String),
    line_no: usize,
) -> Result<(), String> {
    if acc.insert(entry) {
        return Ok(());
    }
    Err(format!(
        "DuplicateException: scripts/hot-cold-forbidden-apis.allow:{line_no} duplicate path/class"
    ))
}

fn load_allow_file(root: &Path) -> Result<BTreeSet<(String, String)>, String> {
    let allow_path = root.join("scripts/hot-cold-forbidden-apis.allow");
    if !allow_path.exists() {
        return Ok(BTreeSet::new());
    }
    let text = fs::read_to_string(&allow_path)
        .map_err(|error| format!("scripts/hot-cold-forbidden-apis.allow: unreadable: {error}"))?;
    text.lines()
        .enumerate()
        .try_fold(BTreeSet::new(), |mut acc, (index, line)| {
            if let Some(entry) = parse_allow_line(index, line)? {
                insert_allow_entry(&mut acc, entry, index.saturating_add(1))?;
            }
            Ok(acc)
        })
}

fn brace_delta(line: &str) -> (usize, usize) {
    (line.matches('{').count(), line.matches('}').count())
}

fn state_after_brace_delta(opens: usize, closes: usize) -> TestSkipState {
    let depth = opens.saturating_sub(closes);
    if depth == 0 {
        TestSkipState::Normal
    } else {
        TestSkipState::InBody(depth)
    }
}

fn state_after_test_mod(code: &str) -> TestSkipState {
    if code.trim_end().ends_with(';') {
        return TestSkipState::Normal;
    }
    let (opens, closes) = brace_delta(code);
    if opens == 0 {
        TestSkipState::AwaitingBody
    } else {
        state_after_brace_delta(opens, closes)
    }
}

fn advance_test_body(depth: usize, code: &str) -> TestSkipState {
    let (opens, closes) = brace_delta(code);
    let next_depth = depth.saturating_add(opens).saturating_sub(closes);
    if next_depth == 0 {
        TestSkipState::Normal
    } else {
        TestSkipState::InBody(next_depth)
    }
}

fn cfg_pending_decision(code: &str) -> (TestSkipState, bool) {
    if code.contains("mod ") {
        return (state_after_test_mod(code), true);
    }
    if code.is_empty() || code.starts_with('#') {
        return (TestSkipState::CfgPending, true);
    }
    (TestSkipState::Normal, false)
}

fn awaiting_body_decision(code: &str) -> (TestSkipState, bool) {
    if code.is_empty() || code.starts_with('#') {
        return (TestSkipState::AwaitingBody, true);
    }
    if code.contains('{') {
        let (opens, closes) = brace_delta(code);
        return (state_after_brace_delta(opens, closes), true);
    }
    (TestSkipState::Normal, false)
}

fn test_skip_decision(state: TestSkipState, line: &str) -> (TestSkipState, bool) {
    let code = without_comment(line).trim();
    match state {
        TestSkipState::InBody(depth) => (advance_test_body(depth, code), true),
        TestSkipState::AwaitingBody => awaiting_body_decision(code),
        TestSkipState::CfgPending => cfg_pending_decision(code),
        TestSkipState::Normal if code.starts_with("#[cfg(test)]") && code.contains("mod ") => {
            (state_after_test_mod(code), true)
        }
        TestSkipState::Normal if code.starts_with("#[cfg(test)]") => {
            (TestSkipState::CfgPending, true)
        }
        TestSkipState::Normal => (TestSkipState::Normal, false),
    }
}

fn rel_path_for(root: &Path, source: &Path) -> String {
    source.strip_prefix(root).map_or_else(
        |_| source.display().to_string(),
        |path| path.display().to_string(),
    )
}

fn classified_line(cold: bool, rel_path: &str) -> String {
    format!(
        "ClassifiedPath|{}|{}",
        if cold { "cold" } else { "hot" },
        rel_path
    )
}

fn record_finding(
    allowed: &BTreeSet<(String, String)>,
    acc: &mut ScanAccumulator,
    finding: Finding,
) {
    if allowed.contains(&(finding.rel_path.clone(), finding.class_id.to_owned())) {
        acc.justified.push(finding);
    } else {
        acc.violations.push(finding);
    }
}

fn record_line_findings(
    rel_path: &str,
    line_no: usize,
    line: &str,
    allowed: &BTreeSet<(String, String)>,
    acc: &mut ScanAccumulator,
) {
    classify_line(rel_path, line_no, line)
        .into_iter()
        .for_each(|finding| record_finding(allowed, acc, finding));
}

fn scan_source_text(
    rel_path: &str,
    text: &str,
    allowed: &BTreeSet<(String, String)>,
    acc: &mut ScanAccumulator,
) {
    let mut state = TestSkipState::Normal;
    text.lines().enumerate().for_each(|(index, line)| {
        let (next_state, skip_line) = test_skip_decision(state, line);
        state = next_state;
        if !skip_line {
            record_line_findings(rel_path, index.saturating_add(1), line, allowed, acc);
        }
    });
}

fn scan_source(
    root: &Path,
    allowed: &BTreeSet<(String, String)>,
    source: &Path,
    acc: &mut ScanAccumulator,
) -> Result<(), String> {
    let rel_path = rel_path_for(root, source);
    let cold = is_cold_path(&rel_path);
    acc.classified.push(classified_line(cold, &rel_path));
    if cold {
        return Ok(());
    }
    let text = fs::read_to_string(source)
        .map_err(|error| format!("{}: unreadable: {error}", source.display()))?;
    scan_source_text(&rel_path, &text, allowed, acc);
    Ok(())
}

fn scan(root: &Path) -> Result<(Vec<String>, Vec<Finding>, Vec<Finding>), String> {
    let allowed = load_allow_file(root)?;
    let sources = hot_sources(root).map_err(|error| format!("hot source scan failed: {error}"))?;
    let mut acc = ScanAccumulator::default();
    for source in sources {
        scan_source(root, &allowed, &source, &mut acc)?;
    }
    Ok((acc.classified, acc.violations, acc.justified))
}

fn write_fixture(path: &Path, text: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)
}

fn write_fixture_checked(path: &Path, text: &str) -> Result<(), String> {
    write_fixture(path, text).map_err(|error| format!("{}: write failed: {error}", path.display()))
}

fn fresh_fixture_root() -> Result<PathBuf, String> {
    let root = std::env::temp_dir().join(format!("hot-cold-scan-{}", std::process::id()));
    match fs::remove_dir_all(&root) {
        Ok(()) => Ok(root),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(root),
        Err(error) => Err(format!("cleanup failed: {error}")),
    }
}

fn write_self_test_fixtures(root: &Path) -> Result<(), String> {
    write_fixture_checked(&root.join("crates/vb_runtime/src/engine.rs"), HOT_FIXTURE)?;
    write_fixture_checked(
        &root.join("crates/vb_runtime/src/bounded_state.rs"),
        BOUNDED_STATE_FIXTURE,
    )?;
    write_fixture_checked(
        &root.join("crates/vb_runtime/src/token_boundaries.rs"),
        TOKEN_BOUNDARY_FIXTURE,
    )?;
    write_fixture_checked(
        &root.join("crates/vb_runtime/src/test_module_shapes.rs"),
        TEST_MODULE_FIXTURE,
    )?;
    write_fixture_checked(
        &root.join("crates/vb_runtime/src/diagnostic.rs"),
        "pub fn ok() { println!(\"diagnostic only\"); }\n",
    )
}

fn class_set(findings: &[Finding]) -> BTreeSet<&'static str> {
    findings.iter().map(|finding| finding.class_id).collect()
}

fn count_class(findings: &[Finding], class_id: &str) -> usize {
    findings
        .iter()
        .filter(|finding| finding.class_id == class_id)
        .count()
}

fn count_path_class(findings: &[Finding], rel_path: &str, class_id: &str) -> usize {
    findings
        .iter()
        .filter(|finding| finding.rel_path == rel_path && finding.class_id == class_id)
        .count()
}

fn new_class_matches_for_path(findings: &[Finding], rel_path: &str) -> BTreeSet<&'static str> {
    findings
        .iter()
        .filter(|finding| finding.rel_path == rel_path && is_new_queue_class(finding.class_id))
        .map(|finding| finding.class_id)
        .collect()
}

fn ensure_no_new_class_findings(
    findings: &[Finding],
    rel_path: &str,
    label: &str,
) -> Result<(), String> {
    let matches = new_class_matches_for_path(findings, rel_path);
    if matches.is_empty() {
        return Ok(());
    }
    Err(format!("{label} false positives {matches:?}"))
}

fn initial_fixture_counts(root: &Path) -> Result<SelfTestCounts, String> {
    let (_classified, violations, _justified) = scan(root)?;
    ensure_fixture_false_positive_absence(&violations)?;
    let (test_module_channel_count, test_module_mutex_count) = test_module_counts(&violations);
    Ok(SelfTestCounts {
        classes: class_set(&violations),
        bounded_channel_count: count_class(&violations, "CHANNEL-BOUNDED-001"),
        mutex_vecdeque_count: count_class(&violations, "QUEUE-MUTEX-VECDEQUE-001"),
        test_module_channel_count,
        test_module_mutex_count,
    })
}

fn ensure_fixture_false_positive_absence(violations: &[Finding]) -> Result<(), String> {
    ensure_no_new_class_findings(
        violations,
        "crates/vb_runtime/src/bounded_state.rs",
        "bounded-state",
    )?;
    ensure_no_new_class_findings(
        violations,
        "crates/vb_runtime/src/token_boundaries.rs",
        "token-boundary",
    )
}

fn test_module_counts(violations: &[Finding]) -> (usize, usize) {
    let rel_path = "crates/vb_runtime/src/test_module_shapes.rs";
    (
        count_path_class(violations, rel_path, "CHANNEL-BOUNDED-001"),
        count_path_class(violations, rel_path, "QUEUE-MUTEX-VECDEQUE-001"),
    )
}

fn missing_required_classes(classes: &BTreeSet<&'static str>) -> Vec<&'static str> {
    [
        "FORMAT-PRINT-001",
        "MAP-STRING-001",
        "CHANNEL-UNBOUNDED-001",
        "CHANNEL-BOUNDED-001",
        "QUEUE-MUTEX-VECDEQUE-001",
    ]
    .iter()
    .filter(|class_id| !classes.contains(**class_id))
    .copied()
    .collect()
}

fn verify_required_classes(classes: &BTreeSet<&'static str>) -> Result<(), String> {
    let missing = missing_required_classes(classes);
    if missing.is_empty() {
        return Ok(());
    }
    Err(format!("missing classes {missing:?}"))
}

fn verify_new_class_counts(counts: &SelfTestCounts) -> Result<(), String> {
    if counts.bounded_channel_count >= 5 && counts.mutex_vecdeque_count >= 4 {
        return Ok(());
    }
    Err(format!(
        "incomplete new-class coverage bounded={} mutex_vecdeque={}",
        counts.bounded_channel_count, counts.mutex_vecdeque_count
    ))
}

fn verify_test_module_skip(counts: &SelfTestCounts) -> Result<(), String> {
    if counts.test_module_channel_count == 1 && counts.test_module_mutex_count == 0 {
        return Ok(());
    }
    Err(format!(
        "cfg(test) module skip mismatch bounded={} mutex_vecdeque={}",
        counts.test_module_channel_count, counts.test_module_mutex_count
    ))
}

fn expect_bad_allow(root: &Path, label: &str, line: &str) -> Result<(), String> {
    let allow_path = root.join("scripts/hot-cold-forbidden-apis.allow");
    write_fixture_checked(&allow_path, line)?;
    match load_allow_file(root) {
        Ok(_allowed) => Err(format!("allowlist accepted {label}")),
        Err(_error) => Ok(()),
    }
}

fn verify_malformed_allowlists(root: &Path) -> Result<(), String> {
    expect_bad_allow(root, "blank owner", "crates/vb_runtime/src/engine.rs|CHANNEL-BOUNDED-001|owner= |reviewed_by=self|test=self|reason=self\n")?;
    expect_bad_allow(root, "wildcard path", "crates/*/src/engine.rs|CHANNEL-BOUNDED-001|owner=self|reviewed_by=self|test=self|reason=self\n")?;
    expect_bad_allow(
        root,
        "ALL class",
        "crates/vb_runtime/src/engine.rs|ALL|owner=self|reviewed_by=self|test=self|reason=self\n",
    )?;
    expect_bad_allow(
        root,
        "non-crates path",
        "src/engine.rs|CHANNEL-BOUNDED-001|owner=self|reviewed_by=self|test=self|reason=self\n",
    )?;
    expect_bad_allow(root, "non-rs path", "crates/vb_runtime/src/engine.txt|CHANNEL-BOUNDED-001|owner=self|reviewed_by=self|test=self|reason=self\n")
}

fn write_valid_allowlist(root: &Path) -> Result<(), String> {
    let allow_path = root.join("scripts/hot-cold-forbidden-apis.allow");
    write_fixture_checked(&allow_path, VALID_ALLOWLIST)
}

fn justified_new_classes(justified: Vec<Finding>) -> BTreeSet<&'static str> {
    justified
        .into_iter()
        .filter(|finding| is_new_queue_class(finding.class_id))
        .map(|finding| finding.class_id)
        .collect()
}

fn verify_no_engine_new_class_violations(violations: &[Finding]) -> Result<(), String> {
    let matches = new_class_matches_for_path(violations, "crates/vb_runtime/src/engine.rs");
    if matches.is_empty() {
        return Ok(());
    }
    Err(format!(
        "allowlisted new classes still violated {matches:?}"
    ))
}

fn verify_allowlist_justification(root: &Path) -> Result<(), String> {
    let (_classified, violations, justified) = scan(root)?;
    verify_no_engine_new_class_violations(&violations)?;
    let justified_classes = justified_new_classes(justified);
    let missing = ["CHANNEL-BOUNDED-001", "QUEUE-MUTEX-VECDEQUE-001"]
        .iter()
        .filter(|class_id| !justified_classes.contains(**class_id))
        .copied()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(());
    }
    Err(format!("missing justified classes {missing:?}"))
}

fn run_self_test() -> Result<(), String> {
    let root = fresh_fixture_root()?;
    write_self_test_fixtures(&root)?;
    let counts = initial_fixture_counts(&root)?;
    verify_required_classes(&counts.classes)?;
    verify_new_class_counts(&counts)?;
    verify_test_module_skip(&counts)?;
    verify_malformed_allowlists(&root)?;
    write_valid_allowlist(&root)?;
    verify_allowlist_justification(&root)
}

fn self_test() -> i32 {
    match run_self_test() {
        Ok(()) => {
            println!("FixturePass: hot/cold forbidden API scanner");
            0
        }
        Err(error) => {
            eprintln!("FixtureFailure: {error}");
            1
        }
    }
}

fn is_self_test_requested() -> bool {
    std::env::args().any(|arg| arg == "--self-test")
}

fn repo_root() -> Result<PathBuf, String> {
    std::env::current_dir().map_err(|error| format!("cannot read current directory: {error}"))
}

fn print_classified(classified: &[String]) {
    classified.iter().for_each(|line| println!("{line}"));
}

fn print_justified(justified: &[Finding]) {
    justified.iter().for_each(|finding| {
        println!(
            "JustifiedException|{}|{}|line={}",
            finding.class_id, finding.rel_path, finding.line_no
        );
    });
}

fn print_violations(violations: &[Finding]) {
    violations.iter().for_each(|finding| {
        println!(
            "ViolationFound|{}|{}|line={}|{}",
            finding.class_id, finding.rel_path, finding.line_no, finding.text
        );
    });
}

fn print_summary(classified: &[String], violations: &[Finding], justified: &[Finding]) {
    println!(
        "ScanSummary|hot_crates={}|classified={}|violations={}|justified={}",
        HOT_CRATES.join(","),
        classified.len(),
        violations.len(),
        justified.len()
    );
}

fn print_scan_report(classified: &[String], violations: &[Finding], justified: &[Finding]) {
    print_classified(classified);
    print_justified(justified);
    print_violations(violations);
    print_summary(classified, violations, justified);
}

fn run_scan(root: &Path) -> i32 {
    match scan(root) {
        Ok((classified, violations, justified)) => {
            print_scan_report(&classified, &violations, &justified);
            if violations.is_empty() {
                0
            } else {
                2
            }
        }
        Err(error) => {
            eprintln!("{error}");
            3
        }
    }
}

fn run() -> i32 {
    if is_self_test_requested() {
        return self_test();
    }
    match repo_root() {
        Ok(root) => run_scan(&root),
        Err(error) => {
            eprintln!("InvalidInvocation: {error}");
            64
        }
    }
}

fn main() {
    std::process::exit(run());
}
