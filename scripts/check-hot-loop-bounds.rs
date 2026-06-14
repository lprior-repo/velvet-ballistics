use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const HOT_CRATES: &[&str] = &["vb_core", "vb_runtime", "vb_storage", "vb_ipc"];

const COLD_PATH_TOKENS: &[&str] = &[
    "/tests/",
    "/verification/",
    "/kani_",
    "/loom_",
    "_tests.rs",
    "/tests_and_verification",
    "/fixture",
    "/_tests",
];

#[derive(Copy, Clone)]
enum TestSkipState {
    Normal,
    Armed,
    InBody { depth: usize },
}

struct Finding {
    rel_path: String,
    line_no: usize,
    text: String,
}

impl Finding {
    fn debug_string(&self) -> String {
        format!("{}:{}: {}", self.rel_path, self.line_no, self.text)
    }
}

fn without_comment(line: &str) -> &str {
    line.split_once("//")
        .map_or(line, |(prefix, _comment)| prefix)
}

fn is_cold_path(rel_path: &str) -> bool {
    if rel_path.ends_with("/tests.rs") || rel_path.ends_with("_tests.rs") {
        return true;
    }
    COLD_PATH_TOKENS.iter().any(|tok| rel_path.contains(tok))
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

fn rel_path_for(root: &Path, source: &Path) -> String {
    source.strip_prefix(root).map_or_else(
        |_| source.display().to_string(),
        |path| path.display().to_string(),
    )
}

fn brace_delta(line: &str) -> (usize, usize) {
    (line.matches('{').count(), line.matches('}').count())
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn find_word_byte(text: &str, word: &[u8]) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i + word.len() <= bytes.len() {
        if &bytes[i..i + word.len()] == word {
            let before_ok = i == 0 || !is_word_byte(bytes[i - 1]);
            let after_pos = i + word.len();
            let after_ok = after_pos == bytes.len() || !is_word_byte(bytes[after_pos]);
            if before_ok && after_ok {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn contains_range_bounded_for(line: &str) -> bool {
    if let Some(for_pos) = find_word_byte(line, b"for") {
        let rest = &line[for_pos + 3..];
        let trimmed = rest.trim_start();
        if let Some(after_for) = trimmed.strip_prefix("in ") {
            let inner = after_for.trim_start();
            if inner.starts_with("0..") {
                return true;
            }
        }
    }
    false
}

fn has_unchecked_index(text: &str) -> bool {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if bytes[i] == b'[' {
            let mut j = i;
            while j > 0 {
                let prev = bytes[j - 1];
                if prev == b' ' || prev == b'\t' {
                    j -= 1;
                } else {
                    break;
                }
            }
            if j > 0 {
                let c = bytes[j - 1];
                if c != b'&' && is_word_byte(c) {
                    if let Some(close_offset) = text[i + 1..].find(']') {
                        let inner = &text[i + 1..i + 1 + close_offset];
                        if !inner.trim().is_empty() {
                            return true;
                        }
                    }
                }
            }
        }
        i += 1;
    }
    false
}

fn has_unbounded_channel(text: &str) -> bool {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let patterns: &[&[u8]] = &[
        b"std::sync::mpsc::sync_channel",
        b"std::sync::mpsc::channel",
        b"mpsc::sync_channel",
        b"mpsc::channel",
    ];
    let mut i = 0;
    while i < len {
        for pat in patterns {
            if i + pat.len() <= len && &bytes[i..i + pat.len()] == *pat {
                if i == 0 || !is_word_byte(bytes[i - 1]) {
                    let after = i + pat.len();
                    if after < len {
                        let next = bytes[after];
                        if next == b'(' || next == b'<' {
                            return true;
                        }
                    }
                }
            }
        }
        i += 1;
    }
    false
}

fn has_type_literal_index(text: &str) -> bool {
    let types: &[&str] = &[
        "[u8;", "[u8]",
        "[u16;", "[u16]",
        "[u32;", "[u32]",
        "[u64;", "[u64]",
        "[i8;", "[i8]",
        "[i16;", "[i16]",
        "[i32;", "[i32]",
        "[i64;", "[i64]",
        "[usize;", "[usize]",
        "[bool;", "[bool]",
        "[char;", "[char]",
        "[str;", "[str]",
        "[f32;", "[f32]",
        "[f64;", "[f64]",
        "[String;", "[String]",
    ];
    types.iter().any(|needle| text.contains(needle))
}

fn has_for_in_list(stripped: &str) -> bool {
    stripped.contains("for ") && stripped.contains(" in [")
}

fn has_slice_of_refs(text: &str) -> bool {
    text.contains("[&")
}

fn is_function_signature(line: &str, code: &str) -> bool {
    if find_word_byte(line, b"fn").is_none() {
        return false;
    }
    if !line.contains('(') {
        return false;
    }
    if !line.contains(')') && !line.contains("->") {
        return false;
    }
    if has_unchecked_index(line) {
        return false;
    }
    let trimmed = code.trim_end();
    if !trimmed.ends_with(')') && !trimmed.ends_with('{') {
        return false;
    }
    if trimmed.contains("->") {
        let after_arrow = trimmed.split("->").nth(1).map_or("", |s| s);
        if after_arrow.contains('=') {
            return false;
        }
    }
    true
}

fn is_field_with_slice(code: &str) -> bool {
    let trimmed = code.trim_start();
    if !trimmed.starts_with("pub ") {
        return false;
    }
    let after_pub = trimmed[4..].trim_start();
    let mut chars = after_pub.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    let mut end = first.len_utf8();
    for ch in chars {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            end += ch.len_utf8();
        } else {
            break;
        }
    }
    let after_ident = &after_pub[end..];
    let after_trim = after_ident.trim_start();
    if !after_trim.starts_with(':') && !after_trim.starts_with('=') {
        return false;
    }
    if !after_trim.contains('&') || !after_trim.contains('[') {
        return false;
    }
    true
}

fn skip_whitespace(bytes: &[u8], mut j: usize) -> usize {
    while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
        j += 1;
    }
    j
}

fn is_slice_signature(line: &str) -> bool {
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if bytes[i] == b':' {
            let j = skip_whitespace(bytes, i + 1);
            if j < len && bytes[j] == b'&' {
                let mut k = j + 1;
                if k < len && bytes[k] == b'\'' {
                    k += 1;
                    while k < len && is_word_byte(bytes[k]) {
                        k += 1;
                    }
                }
                k = skip_whitespace(bytes, k);
                if k + 3 < len && &bytes[k..k + 3] == b"mut" {
                    if k + 3 < len && (bytes[k + 3] == b' ' || bytes[k + 3] == b'\t') {
                        k += 4;
                        k = skip_whitespace(bytes, k);
                    }
                }
                if k < len && bytes[k] == b'[' {
                    if find_word_byte(&line[k..], b"]=").is_none() {
                        return true;
                    }
                }
            }
        }
        i += 1;
    }
    false
}

fn is_fn_pointer_slice_signature(line: &str) -> bool {
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if bytes[i] == b':' {
            let j = skip_whitespace(bytes, i + 1);
            if j < len && bytes[j] == b'&' {
                let mut k = j + 1;
                if k < len && bytes[k] == b'\'' {
                    k += 1;
                    while k < len && is_word_byte(bytes[k]) {
                        k += 1;
                    }
                }
                k = skip_whitespace(bytes, k);
                if k < len && bytes[k] == b'[' {
                    let inner = &line[k + 1..];
                    if inner.trim_start().starts_with('(') {
                        return true;
                    }
                }
            }
        }
        i += 1;
    }
    false
}

fn is_destructuring_let(code: &str) -> bool {
    let trimmed = code.trim_start();
    let mut after_let = trimmed.strip_prefix("let ").map_or("", |s| s);
    if let Some(rest) = after_let.strip_prefix("mut ") {
        after_let = rest;
    }
    after_let.trim_start().starts_with('[')
}

fn has_full_slice(text: &str) -> bool {
    text.contains("[..]")
}

fn audit_file(rel_path: &str, text: &str) -> Vec<Finding> {
    let mut findings: Vec<Finding> = Vec::new();
    let mut state = TestSkipState::Normal;
    let mut range_loop_depth: usize = 0;
    let mut flux_trusted_depth: i32 = 0;

    for (index, raw_line) in text.lines().enumerate() {
        let line_no = index + 1;
        let trimmed_raw = raw_line.trim();
        if trimmed_raw.starts_with("//") || trimmed_raw.starts_with('*') {
            continue;
        }
        let line = without_comment(raw_line);
        let code = line.trim();

        match state {
            TestSkipState::Armed => {
                if !line.contains('{') {
                    continue;
                }
                let (opens, closes) = brace_delta(line);
                let depth = opens as i32 - closes as i32;
                if depth <= 0 {
                    state = TestSkipState::Normal;
                } else {
                    state = TestSkipState::InBody { depth: depth as usize };
                }
                continue;
            }
            TestSkipState::InBody { depth } => {
                let (opens, closes) = brace_delta(line);
                let new_depth = depth as i32 + opens as i32 - closes as i32;
                if new_depth <= 0 {
                    state = TestSkipState::Normal;
                } else {
                    state = TestSkipState::InBody { depth: new_depth as usize };
                }
                continue;
            }
            TestSkipState::Normal => {}
        }

        if code.starts_with("#[cfg(test)]") {
            if line.contains('{') {
                let (opens, closes) = brace_delta(line);
                let depth = opens as i32 - closes as i32;
                if depth <= 0 {
                    state = TestSkipState::Normal;
                } else {
                    state = TestSkipState::InBody { depth: depth as usize };
                }
            } else {
                state = TestSkipState::Armed;
            }
            continue;
        }

        if contains_range_bounded_for(line) {
            let (opens, closes) = brace_delta(line);
            let delta = opens as i32 - closes as i32;
            if delta > 0 {
                range_loop_depth = (range_loop_depth as i32 + delta) as usize;
            }
        } else if range_loop_depth > 0 {
            let (opens, closes) = brace_delta(line);
            let delta = opens as i32 - closes as i32;
            let new_depth = range_loop_depth as i32 + delta;
            if new_depth <= 0 {
                range_loop_depth = 0;
            } else {
                range_loop_depth = new_depth as usize;
            }
        }

        if flux_trusted_depth == 0 && code == "#[flux_rs::trusted]" {
            flux_trusted_depth = 1;
            continue;
        }
        if flux_trusted_depth > 0 {
            let (opens, closes) = brace_delta(line);
            flux_trusted_depth += opens as i32 - closes as i32;
            if flux_trusted_depth <= 0 {
                flux_trusted_depth = 0;
            }
            continue;
        }

        if is_function_signature(line, code) {
            continue;
        }

        if is_field_with_slice(code) {
            continue;
        }

        if is_slice_signature(line) {
            continue;
        }

        if is_fn_pointer_slice_signature(line) {
            continue;
        }

        if is_destructuring_let(code) {
            continue;
        }

        if has_full_slice(line) {
            continue;
        }

        if has_unchecked_index(line) {
            if code.starts_with('#') || code.contains("#[derive") {
                continue;
            }
            if has_type_literal_index(line) {
                continue;
            }
            if has_for_in_list(code) {
                continue;
            }
            if has_slice_of_refs(line) {
                continue;
            }
            if range_loop_depth > 0 {
                continue;
            }
            if flux_trusted_depth > 0 {
                continue;
            }
            let text: String = trimmed_raw.chars().take(100).collect();
            findings.push(Finding {
                rel_path: rel_path.to_owned(),
                line_no,
                text,
            });
        }

        if has_unbounded_channel(line) {
            let text: String = trimmed_raw.chars().take(100).collect();
            findings.push(Finding {
                rel_path: rel_path.to_owned(),
                line_no,
                text,
            });
        }
    }
    findings
}

fn scan(root: &Path) -> Result<(usize, Vec<Finding>), String> {
    let sources = hot_sources(root).map_err(|error| format!("hot source scan failed: {error}"))?;
    let mut scanned = 0;
    let mut all: Vec<Finding> = Vec::new();
    for source in sources {
        let rel_path = rel_path_for(root, &source);
        if is_cold_path(&rel_path) {
            continue;
        }
        scanned += 1;
        let text = fs::read_to_string(&source)
            .map_err(|error| format!("{}: unreadable: {error}", source.display()))?;
        all.extend(audit_file(&rel_path, &text));
    }
    Ok((scanned, all))
}

fn repo_root() -> Result<PathBuf, String> {
    std::env::current_dir().map_err(|error| format!("cannot read current directory: {error}"))
}

fn is_self_test_requested() -> bool {
    std::env::args().any(|arg| arg == "--self-test")
}

const SELF_TEST_INDEX_FILE: &str = concat!(
    "pub fn good_let_destructure(input: [u8; 2]) -> u8 { let [a, b] = input; a + b }\n",
    "pub fn good_full_slice(input: &[u8]) -> &[u8] { &input[..] }\n",
    "pub fn good_for_range(arr: [u8; 4]) -> u8 { let mut s = 0u8; for v in 0..4 { s += arr[v]; } s }\n",
    "pub fn good_checked_access(arr: &[u8], idx: usize) -> u8 { arr.get(idx).copied().unwrap_or(0) }\n",
    "#[flux_rs::trusted]\n",
    "pub fn flux_trusted_index(arr: [u8; 4], idx: usize) -> u8 { arr[idx] }\n",
    "pub fn good_not_mpsc() { let _x = not_mpsc::channel::<u8>(); }\n",
    "pub fn good_crossbeam() { let _q = crossbeam_channel::bounded::<u8>(8); }\n",
    "pub fn good_map_ref_key(m: std::collections::HashMap<u8, u8>, k: u8) -> u8 { m[&k] }\n",
);

const SELF_TEST_CFG_TEST_FILE: &str = concat!(
    "#[cfg(test)] mod same_line_tests {\n",
    "    pub fn bad_inside_test(arr: Vec<u8>, idx: usize) -> u8 { arr[idx] }\n",
    "}\n",
    "#[cfg(test)]\n",
    "mod next_line_tests\n",
    "{\n",
    "    pub fn bad_inside_test_2(arr: Vec<u8>, idx: usize) -> u8 { arr[idx] }\n",
    "}\n",
);

const SELF_TEST_TYPE_LITERAL_FILE: &str = concat!(
    "pub fn good_type_literal(arr: [u8; 4], idx: usize) -> u8 { arr[idx] }\n",
    "pub fn good_for_in_list() { for v in [1u8, 2, 3, 4] { let _x = v; } }\n",
    "pub fn good_for_in_list_with_index() { for v in [1u8, 2, 3, 4].iter() { let _x = v; } }\n",
);

const SELF_TEST_SLICE_SIG_FILE: &str = concat!(
    "pub fn good_slice_sig(name: &[u8]) -> usize { name.len() }\n",
    "pub fn good_mut_slice_sig(name: &mut [u8]) -> usize { name.len() }\n",
    "pub fn good_lifetime_slice_sig<'a>(name: &'a [u8]) -> usize { name.len() }\n",
    "pub struct GoodField {\n",
    "    pub name: &[u8],\n",
    "    pub other: Vec<u8>,\n",
    "}\n",
    "pub fn good_fn_pointer_sig(fns: &[fn(u8) -> u8], idx: usize) -> u8 { fns[idx.min(3)](0) }\n",
);

const SELF_TEST_BAD_FINDINGS_FILE: &str = concat!(
    "pub fn bad_index(arr: Vec<u8>, idx: usize) -> u8 { arr[idx] }\n",
    "pub fn use_mpsc_chan() { let (_tx, _rx) = mpsc::channel(); }\n",
    "pub fn use_sync_chan() { let (_tx, _rx) = mpsc::sync_channel(8); }\n",
    "pub fn use_std_chan() { let (_tx, _rx) = std::sync::mpsc::channel(); }\n",
    "pub fn use_path_index() { let arr = vec![1u8, 2, 3]; let _ = arr[0]; }\n",
);

fn fresh_fixture_root() -> Result<PathBuf, String> {
    let root = std::env::temp_dir().join(format!("hot-loop-bounds-{}", std::process::id()));
    match fs::remove_dir_all(&root) {
        Ok(()) => Ok(root),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(root),
        Err(error) => Err(format!("cleanup failed: {error}")),
    }
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

fn write_self_test_fixtures(root: &Path) -> Result<(), String> {
    write_fixture_checked(&root.join("crates/vb_core/src/index_check.rs"), SELF_TEST_INDEX_FILE)?;
    write_fixture_checked(&root.join("crates/vb_core/src/cfg_test_check.rs"), SELF_TEST_CFG_TEST_FILE)?;
    write_fixture_checked(&root.join("crates/vb_runtime/src/type_literal_check.rs"), SELF_TEST_TYPE_LITERAL_FILE)?;
    write_fixture_checked(&root.join("crates/vb_storage/src/slice_sig_check.rs"), SELF_TEST_SLICE_SIG_FILE)?;
    write_fixture_checked(&root.join("crates/vb_ipc/src/bad_findings_check.rs"), SELF_TEST_BAD_FINDINGS_FILE)
}

fn count_findings_with(findings: &[Finding], rel_path: &str, marker: &str) -> usize {
    findings
        .iter()
        .filter(|f| f.rel_path == rel_path && f.text.contains(marker))
        .count()
}

fn total_findings(findings: &[Finding]) -> usize {
    findings.len()
}

fn ensure_no_findings(findings: &[Finding], rel_path: &str, label: &str) -> Result<(), String> {
    let matches: Vec<String> = findings
        .iter()
        .filter(|f| f.rel_path == rel_path)
        .map(|f| f.debug_string())
        .collect();
    if matches.is_empty() {
        return Ok(());
    }
    Err(format!("{label} unexpected findings: {matches:?}"))
}

fn run_self_test() -> Result<(), String> {
    let root = fresh_fixture_root()?;
    write_self_test_fixtures(&root)?;
    let (_scanned, findings) = scan(&root)?;

    ensure_no_findings(
        &findings,
        "crates/vb_core/src/index_check.rs",
        "index-check-fixture",
    )?;
    ensure_no_findings(
        &findings,
        "crates/vb_core/src/cfg_test_check.rs",
        "cfg-test-fixture",
    )?;
    ensure_no_findings(
        &findings,
        "crates/vb_runtime/src/type_literal_check.rs",
        "type-literal-fixture",
    )?;
    ensure_no_findings(
        &findings,
        "crates/vb_storage/src/slice_sig_check.rs",
        "slice-sig-fixture",
    )?;

    let bad_findings_path = "crates/vb_ipc/src/bad_findings_check.rs";
    let bad_count = count_findings_with(&findings, bad_findings_path, "arr[idx]")
        + count_findings_with(&findings, bad_findings_path, "mpsc::channel")
        + count_findings_with(&findings, bad_findings_path, "mpsc::sync_channel");
    if bad_count < 3 {
        return Err(format!(
            "bad-findings-fixture expected >=3 findings, got {bad_count}; total={}",
            total_findings(&findings)
        ));
    }

    let mpsc_findings: Vec<_> = findings
        .iter()
        .filter(|f| f.text.contains("mpsc::channel") || f.text.contains("mpsc::sync_channel"))
        .collect();
    if mpsc_findings.len() < 3 {
        return Err(format!(
            "expected >=3 mpsc findings, got {}; total={}",
            mpsc_findings.len(),
            total_findings(&findings)
        ));
    }

    let not_mpsc_findings = findings
        .iter()
        .filter(|f| f.text.contains("not_mpsc::channel"))
        .count();
    if not_mpsc_findings != 0 {
        return Err(format!(
            "not_mpsc::channel should not be flagged, got {not_mpsc_findings} findings"
        ));
    }

    let crossbeam_findings = findings
        .iter()
        .filter(|f| f.text.contains("crossbeam_channel::bounded"))
        .count();
    if crossbeam_findings != 0 {
        return Err(format!(
            "crossbeam_channel::bounded should not be flagged, got {crossbeam_findings} findings"
        ));
    }

    Ok(())
}

fn self_test() -> i32 {
    match run_self_test() {
        Ok(()) => {
            println!("FixturePass: hot-loop-bounds scanner");
            0
        }
        Err(error) => {
            eprintln!("FixtureFailure: {error}");
            1
        }
    }
}

fn print_summary(scanned: usize, violations: &[Finding]) {
    println!(
        "hot-loop-bounds-audit: scanned {} hot-crate files in {}",
        scanned,
        HOT_CRATES.join(",")
    );
    if violations.is_empty() {
        println!("PASS: no hot-loop bound violations found in hot-crate sources");
    } else {
        println!("FAIL: {} hot-loop bound violations found", violations.len());
        for finding in violations.iter().take(30) {
            println!(
                "  {}:{}: {}: {}",
                finding.rel_path, finding.line_no, classify(&finding.text), finding.text
            );
        }
        if violations.len() > 30 {
            println!("  ... and {} more", violations.len() - 30);
        }
    }
}

fn classify(text: &str) -> &'static str {
    if text.contains("mpsc::channel") || text.contains("mpsc::sync_channel") {
        "unbounded channel"
    } else {
        "unchecked index"
    }
}

fn run_scan(root: &Path) -> i32 {
    match scan(root) {
        Ok((scanned, findings)) => {
            print_summary(scanned, &findings);
            if findings.is_empty() {
                0
            } else {
                1
            }
        }
        Err(error) => {
            eprintln!("{error}");
            2
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

fn main() -> ExitCode {
    let code = run();
    match code {
        0 => ExitCode::SUCCESS,
        1 => ExitCode::from(1),
        2 => ExitCode::from(2),
        64 => ExitCode::from(64),
        _ => ExitCode::from(1),
    }
}
