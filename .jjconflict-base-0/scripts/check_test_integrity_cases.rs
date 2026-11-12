use super::check_test_integrity_fixture::{BASE_TEXT, Fixture, expect_findings, report_failure};

pub(crate) fn run_delete_case(label: &str, path: &str) -> bool {
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write(path, BASE_TEXT)
        .and_then(|()| fixture.commit_base())
        .and_then(|()| fixture.remove(path));
    if let Err(error) = setup {
        return report_failure(label, &format!("setup: {error}"));
    }
    let findings = match fixture.collect() {
        Ok(findings) => findings,
        Err(error) => return report_failure(label, &format!("collect: {error}")),
    };
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => return report_failure(label, &format!("check: {error}")),
    };
    let expected = [
        ("DeletedTestFile", path),
        ("DeletedTestDeclaration", path),
        ("WeakenedAssertion", path),
    ];
    let findings_ok = expect_findings(label, &findings, &expected);
    println!("SelfTest:{label}: exit expected=1 actual={exit}");
    findings_ok && exit == 1
}

pub(crate) fn run_ignore_case(label: &str, path: &str) -> bool {
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write(path, BASE_TEXT)
        .and_then(|()| fixture.commit_base())
        .and_then(|()| {
            fixture.write(
                path,
                "#[test]\n#[ignore]\nfn behavior_test() {\n    assert_eq!(2 + 2, 4);\n}\n",
            )
        });
    if let Err(error) = setup {
        return report_failure(label, &format!("setup: {error}"));
    }
    let findings = match fixture.collect() {
        Ok(findings) => findings,
        Err(error) => return report_failure(label, &format!("collect: {error}")),
    };
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => return report_failure(label, &format!("check: {error}")),
    };
    let expected = [("IgnoredOrSkippedTest", path)];
    let findings_ok = expect_findings(label, &findings, &expected);
    println!("SelfTest:{label}: exit expected=1 actual={exit}");
    findings_ok && exit == 1
}

pub(crate) fn run_weaken_case(label: &str, path: &str) -> bool {
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write(path, BASE_TEXT)
        .and_then(|()| fixture.commit_base())
        .and_then(|()| {
            fixture.write(
                path,
                "#[test]\nfn behavior_test() {\n    assert!((2 + 2).checked_sub(4).is_some());\n}\n",
            )
        });
    if let Err(error) = setup {
        return report_failure(label, &format!("setup: {error}"));
    }
    let findings = match fixture.collect() {
        Ok(findings) => findings,
        Err(error) => return report_failure(label, &format!("collect: {error}")),
    };
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => return report_failure(label, &format!("check: {error}")),
    };
    let expected = [("WeakenedAssertion", path)];
    let findings_ok = expect_findings(label, &findings, &expected);
    println!("SelfTest:{label}: exit expected=1 actual={exit}");
    findings_ok && exit == 1
}

pub(crate) fn run_strengthen_case(label: &str, path: &str) -> bool {
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write(path, BASE_TEXT)
        .and_then(|()| fixture.commit_base())
        .and_then(|()| {
            fixture.write(
                path,
                "#[test]\nfn behavior_test() {\n    assert_eq!(2 + 2, 4);\n    assert_ne!(2 + 2, 5);\n}\n",
            )
        });
    if let Err(error) = setup {
        return report_failure(label, &format!("setup: {error}"));
    }
    let findings = match fixture.collect() {
        Ok(findings) => findings,
        Err(error) => return report_failure(label, &format!("collect: {error}")),
    };
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => return report_failure(label, &format!("check: {error}")),
    };
    let expected: [(&str, &str); 0] = [];
    let findings_ok = expect_findings(label, &findings, &expected);
    println!("SelfTest:{label}: exit expected=0 actual={exit}");
    findings_ok && exit == 0
}

pub(crate) fn run_silent_return_case(label: &str, path: &str) -> bool {
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write(path, BASE_TEXT)
        .and_then(|()| fixture.commit_base())
        .and_then(|()| {
            fixture.write(
                path,
                "#[test]\nfn behavior_test() {\n    let Ok(value) = Result::<u8, ()>::Ok(4) else {\n        return;\n    };\n    assert_eq!(value, 4);\n}\n",
            )
        });
    if let Err(error) = setup {
        return report_failure(label, &format!("setup: {error}"));
    }
    let findings = match fixture.collect() {
        Ok(findings) => findings,
        Err(error) => return report_failure(label, &format!("collect: {error}")),
    };
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => return report_failure(label, &format!("check: {error}")),
    };
    let expected = [("IgnoredOrSkippedTest", path)];
    let findings_ok = expect_findings(label, &findings, &expected);
    println!("SelfTest:{label}: exit expected=1 actual={exit}");
    findings_ok && exit == 1
}

pub(crate) fn run_rename_out_case(label: &str) -> bool {
    let from = "crates/workspace_tests/tests/behavior.rs";
    let to = "archive/behavior.rs";
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write(from, BASE_TEXT)
        .and_then(|()| fixture.commit_base())
        .and_then(|()| fixture.rename(from, to))
        .and_then(|()| fixture.stage_all());
    if let Err(error) = setup {
        return report_failure(label, &format!("setup: {error}"));
    }
    let findings = match fixture.collect() {
        Ok(findings) => findings,
        Err(error) => return report_failure(label, &format!("collect: {error}")),
    };
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => return report_failure(label, &format!("check: {error}")),
    };
    let expected = [
        ("DeletedTestFile", from),
        ("DeletedTestDeclaration", from),
        ("WeakenedAssertion", from),
    ];
    let findings_ok = expect_findings(label, &findings, &expected);
    println!("SelfTest:{label}: exit expected=1 actual={exit}");
    findings_ok && exit == 1
}

pub(crate) fn run_relocate_case(label: &str) -> bool {
    let from = "crates/workspace_tests/tests/behavior.rs";
    let to = "crates/workspace_tests/tests/behavior_relocated.rs";
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write(from, BASE_TEXT)
        .and_then(|()| fixture.commit_base())
        .and_then(|()| fixture.rename(from, to))
        .and_then(|()| fixture.stage_all());
    if let Err(error) = setup {
        return report_failure(label, &format!("setup: {error}"));
    }
    let findings = match fixture.collect() {
        Ok(findings) => findings,
        Err(error) => return report_failure(label, &format!("collect: {error}")),
    };
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => return report_failure(label, &format!("check: {error}")),
    };
    let expected: [(&str, &str); 0] = [];
    let findings_ok = expect_findings(label, &findings, &expected);
    println!("SelfTest:{label}: exit expected=0 actual={exit}");
    findings_ok && exit == 0
}
