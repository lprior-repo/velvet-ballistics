use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Fixture(PathBuf);

impl Fixture {
    fn new(name: &str) -> Self {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.evidence/hotpath-cli")
            .join(format!("{name}-{}-{stamp}", std::process::id()));
        fs::create_dir_all(dir.parent().expect("parent")).expect("parent directory");
        fs::create_dir(&dir).expect("fresh fixture directory");
        let fixture = Self(dir);
        fixture.file("Cargo.toml", b"");
        fs::create_dir(fixture.0.join("crates")).expect("crates directory");
        fixture
    }

    fn file(&self, name: &str, contents: &[u8]) {
        let file = self.0.join(name);
        fs::create_dir_all(file.parent().expect("parent")).expect("file parent");
        fs::write(file, contents).expect("fixture file");
    }

    fn run(&self) -> Output {
        self.command().output().expect("scanner runs")
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_hotpath-scan"));
        command.current_dir(&self.0);
        command
    }
}

fn summary(classified: usize, violations: usize, justified: usize) -> String {
    format!(
        "ScanSummary|hot_roots=vb_core,vb_runtime,vb_storage,vb_ipc|classified={classified}|violations={violations}|justified={justified}\n"
    )
}

fn assert_output(output: Output, code: i32, stdout: &str, stderr: &str) {
    assert_eq!(output.status.code(), Some(code));
    assert_eq!(
        String::from_utf8(output.stdout).expect("UTF-8 stdout"),
        stdout
    );
    assert_eq!(
        String::from_utf8(output.stderr).expect("UTF-8 stderr"),
        stderr
    );
}

#[test]
fn additional_arguments_leave_empty_root_scan_unchanged() {
    let fixture = Fixture::new("empty");
    assert_output(fixture.run(), 0, &summary(0, 0, 0), "");
    assert_output(
        fixture.command().arg("--unused").output().expect("scanner"),
        0,
        &summary(0, 0, 0),
        "",
    );
}

#[test]
fn invalid_invocation_requires_a_manifest_file_and_crates_directory() {
    let fixture = Fixture::new("invalid-root");
    for (name, manifest_file, crates_dir) in [
        ("bad-manifest", false, true),
        ("bad-crates", true, false),
        ("both-invalid", false, false),
    ] {
        let dir = fixture.0.join(name);
        fs::create_dir(&dir).expect("invalid-root fixture");
        if manifest_file {
            fs::write(dir.join("Cargo.toml"), "").expect("manifest file");
        } else {
            fs::create_dir(dir.join("Cargo.toml")).expect("manifest directory");
        }
        if crates_dir {
            fs::create_dir(dir.join("crates")).expect("crates directory");
        } else {
            fs::write(dir.join("crates"), "not a directory").expect("crates file");
        }
        assert_output(
            fixture
                .command()
                .current_dir(dir)
                .output()
                .expect("scanner"),
            64,
            "",
            "InvalidInvocation: run from repository root\n",
        );
    }
}

#[test]
fn all_tokens_are_substrings_and_emitted_in_contract_order() {
    let fixture = Fixture::new("tokens");
    let rel = "crates/vb_core/src/types.rs";
    let text = "HashMapSuffix IndexMap IndexSet BTreeMap std::sync::mpsc mpsc::channel(";
    fixture.file(rel, text.as_bytes());
    let mut expected = format!("ClassifiedPath|hot|{rel}\n");
    for token in [
        "HashMap",
        "IndexMap",
        "IndexSet",
        "BTreeMap",
        "std::sync::mpsc",
        "mpsc::channel",
        "channel(",
    ] {
        expected.push_str(&format!("ViolationFound|{token}|{rel}|line=1|{text}\n"));
    }
    expected.push_str(&summary(1, 7, 0));
    assert_output(fixture.run(), 2, &expected, "");
}

#[test]
fn paths_use_component_order_within_each_root_and_fixed_root_order() {
    let fixture = Fixture::new("order");
    let paths = [
        "crates/vb_core/src/a/z.rs",
        "crates/vb_core/src/a-.rs",
        "crates/vb_core/src/z.rs",
        "crates/vb_runtime/src/m.rs",
        "crates/vb_storage/src/b.rs",
        "crates/vb_ipc/src/a.rs",
    ];
    for rel in paths.iter().rev() {
        fixture.file(rel, b"");
    }
    let expected = paths
        .iter()
        .map(|rel| format!("ClassifiedPath|hot|{rel}\n"))
        .collect::<String>()
        + &summary(6, 0, 0);
    assert_output(fixture.run(), 0, &expected, "");
}

#[test]
fn every_cold_token_is_reported_without_reading_cold_contents() {
    let fixture = Fixture::new("cold");
    let mut paths = [
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
    ]
    .map(|name| format!("crates/vb_core/src/{name}/data.rs"));
    for rel in &paths {
        fixture.file(rel, &[0xff]);
    }
    paths.sort();
    let expected = paths
        .iter()
        .map(|rel| format!("ClassifiedPath|cold|{rel}\n"))
        .collect::<String>()
        + &summary(15, 0, 0);
    assert_output(fixture.run(), 0, &expected, "");
}

#[test]
fn cold_tokens_require_exact_case_and_word_boundaries() {
    let fixture = Fixture::new("cold-boundaries");
    fixture.file("crates/vb_core/src/Tests.rs", b"HashMap");
    fixture.file("crates/vb_core/src/contest.rs", b"HashMap");
    fixture.file("crates/vb_core/src/test-plan.rs", b"HashMap");
    let expected = concat!(
        "ClassifiedPath|hot|crates/vb_core/src/Tests.rs\n",
        "ClassifiedPath|hot|crates/vb_core/src/contest.rs\n",
        "ClassifiedPath|cold|crates/vb_core/src/test-plan.rs\n",
        "ViolationFound|HashMap|crates/vb_core/src/Tests.rs|line=1|HashMap\n",
        "ViolationFound|HashMap|crates/vb_core/src/contest.rs|line=1|HashMap\n"
    )
    .to_owned()
        + &summary(3, 2, 0);
    assert_output(fixture.run(), 2, &expected, "");
}

#[test]
fn duplicate_allowances_and_repeated_text_count_once_per_matching_line() {
    let fixture = Fixture::new("allow");
    let rel = "crates/vb_core/src/types.rs";
    fixture.file(rel, b"HashMap HashMap\nHashMap");
    let entry = format!("{rel}|HashMap|owner=|reviewed_by=|test=|reason=\n");
    fixture.file("scripts/hotpath-scan.allow", entry.repeat(2).as_bytes());
    let expected = format!(
        "ClassifiedPath|hot|{rel}\nJustifiedException|HashMap|{rel}|line=1\nJustifiedException|HashMap|{rel}|line=2\n"
    ) + &summary(1, 0, 2);
    assert_output(fixture.run(), 0, &expected, "");
}

#[test]
fn allowances_match_the_full_path_and_only_the_named_token() {
    let fixture = Fixture::new("allow-path");
    let rel = "crates/vb_core/src/types.rs";
    fixture.file(rel, b" HashMap\t IndexMap  ");
    fixture.file(
        "scripts/hotpath-scan.allow",
        concat!(
            "crates/vb_runtime/src/types.rs|HashMap|owner=|reviewed_by=|test=|reason=\n",
            "crates/vb_core/src/types.rs|IndexMap|owner=|reviewed_by=|test=|reason=\n"
        )
        .as_bytes(),
    );
    let expected = format!(
        "ClassifiedPath|hot|{rel}\nJustifiedException|IndexMap|{rel}|line=1\nViolationFound|HashMap|{rel}|line=1|HashMap IndexMap\n"
    ) + &summary(1, 1, 1);
    assert_output(fixture.run(), 2, &expected, "");
}

#[test]
fn malformed_allowances_fail_before_classification_with_exact_diagnostics() {
    let cases = [
        (
            "wrong",
            "MalformedException",
            "expected path|token|owner=...|reviewed_by=...|test=...|reason=...",
        ),
        (
            "crates/*.rs|HashMap|owner=|reviewed_by=|test=|reason=",
            "OverbroadException",
            "path must be exact crates/*/src/*.rs",
        ),
        (
            "crates/a.rs|Unknown|owner=|reviewed_by=|test=|reason=",
            "UnknownTokenException",
            "token must be one of HashMap,IndexMap,IndexSet,BTreeMap,std::sync::mpsc,mpsc::channel,channel(",
        ),
        (
            "crates/a.rs|HashMap| owner=|reviewed_by=|test=|reason=",
            "MalformedException",
            "missing owner/reviewed_by/test/reason",
        ),
    ];
    for (entry, category, detail) in cases {
        let fixture = Fixture::new(category);
        fixture.file("crates/vb_core/src/types.rs", b"HashMap");
        fixture.file(
            "scripts/hotpath-scan.allow",
            format!("\n# comment\n{entry}\n").as_bytes(),
        );
        assert_output(
            fixture.run(),
            3,
            "",
            &format!("{category}: scripts/hotpath-scan.allow:3 {detail}\n"),
        );
    }
}

#[test]
fn exception_policy_preserves_prefix_only_metadata_and_outer_whitespace() {
    let fixture = Fixture::new("allow-prefix");
    fixture.file(
        "scripts/hotpath-scan.allow",
        b"  crates/a.rs|HashMap|owner=|reviewed_by=|test=|reason=  \n",
    );
    assert_output(fixture.run(), 0, &summary(0, 0, 0), "");
}

#[test]
fn first_double_slash_is_stripped_even_inside_a_literal() {
    let fixture = Fixture::new("comments");
    let rel = "crates/vb_core/src/comments.rs";
    fixture.file(
        rel,
        b"// HashMap\nlet url = \"http://HashMap\";\n/* HashMap */\nHashMapSuffix\n",
    );
    let expected = format!(
        "ClassifiedPath|hot|{rel}\nViolationFound|HashMap|{rel}|line=3|/* HashMap */\nViolationFound|HashMap|{rel}|line=4|HashMapSuffix\n"
    ) + &summary(1, 2, 0);
    assert_output(fixture.run(), 2, &expected, "");
}

#[test]
fn mixed_line_separators_preserve_source_line_numbers() {
    let fixture = Fixture::new("newlines");
    let rel = "crates/vb_core/src/lines.rs";
    fixture.file(
        rel,
        "HashMap\r\nIndexMap\rIndexSet\nBTreeMap\u{2028}channel(".as_bytes(),
    );
    let mut expected = format!("ClassifiedPath|hot|{rel}\n");
    for (line, token) in [
        (1, "HashMap"),
        (2, "IndexMap"),
        (3, "IndexSet"),
        (4, "BTreeMap"),
        (5, "channel("),
    ] {
        expected.push_str(&format!(
            "ViolationFound|{token}|{rel}|line={line}|{token}\n"
        ));
    }
    expected.push_str(&summary(1, 5, 0));
    assert_output(fixture.run(), 2, &expected, "");
}

#[test]
fn directory_symlinks_are_not_followed_and_file_symlinks_are_scanned() {
    let fixture = Fixture::new("links");
    let rel = "crates/vb_core/src";
    fixture.file(&format!("{rel}/good.rs"), b"HashMap");
    std::os::unix::fs::symlink(".", fixture.0.join(rel).join("cycle")).expect("directory link");
    std::os::unix::fs::symlink("good.rs", fixture.0.join(rel).join("alias.rs")).expect("file link");
    let expected = format!(
        "ClassifiedPath|hot|{rel}/alias.rs\nClassifiedPath|hot|{rel}/good.rs\nViolationFound|HashMap|{rel}/alias.rs|line=1|HashMap\nViolationFound|HashMap|{rel}/good.rs|line=1|HashMap\n"
    ) + &summary(2, 2, 0);
    assert_output(fixture.run(), 2, &expected, "");
}

#[test]
fn dot_rs_is_included_in_the_source_glob() {
    let fixture = Fixture::new("dot-rs");
    let rel = "crates/vb_core/src/.rs";
    fixture.file(rel, b"HashMap");
    let expected =
        format!("ClassifiedPath|hot|{rel}\nViolationFound|HashMap|{rel}|line=1|HashMap\n")
            + &summary(1, 1, 0);
    assert_output(fixture.run(), 2, &expected, "");
}

#[test]
fn unreadable_source_content_fails_closed_without_a_summary() {
    let fixture = Fixture::new("invalid-utf8");
    fixture.file("crates/vb_core/src/bad.rs", &[0xff]);
    let output = fixture.run();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        output.stdout,
        b"ClassifiedPath|hot|crates/vb_core/src/bad.rs\n"
    );
    assert!(String::from_utf8_lossy(&output.stderr).starts_with("HotpathScanIoError:"));
}

#[test]
fn failed_stdout_is_an_operational_failure() {
    let fixture = Fixture::new("full-output");
    let full = fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .expect("full device");
    let output = fixture.command().stdout(full).output().expect("scanner");
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).starts_with("HotpathScanIoError:"));
}

#[test]
fn legacy_control_whitespace_is_trimmed_tokenized_and_collapsed() {
    let fixture = Fixture::new("control-whitespace");
    let hot = "crates/vb_core/src/hot.rs";
    let cold = "crates/vb_core/src/test\u{1f}helpers.rs";
    fixture.file(hot, "HashMap\u{1f}IndexMap".as_bytes());
    fixture.file(cold, b"HashMap");
    fixture.file(
        "scripts/hotpath-scan.allow",
        format!("\u{1f}{hot}|HashMap|owner=|reviewed_by=|test=|reason=\u{1f}\n").as_bytes(),
    );
    let expected = format!(
        "ClassifiedPath|hot|{hot}\nJustifiedException|HashMap|{hot}|line=1\nClassifiedPath|cold|{cold}\nViolationFound|IndexMap|{hot}|line=1|HashMap IndexMap\n"
    ) + &summary(2, 1, 1);
    assert_output(fixture.run(), 2, &expected, "");
}
