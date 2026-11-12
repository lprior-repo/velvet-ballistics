use super::check_test_integrity_fixture::{
    BASE_TEXT, Fixture, expect_findings, large_test_retained_modified, large_test_text,
    large_test_weakened, report_failure,
};

pub(crate) fn run_relocate_modified_retained_case(label: &str) -> bool {
    let from = "crates/workspace_tests/tests/behavior.rs";
    let to = "crates/workspace_tests/tests/behavior_relocated.rs";
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write(from, &large_test_text())
        .and_then(|()| fixture.commit_base())
        .and_then(|()| fixture.rename(from, to))
        .and_then(|()| fixture.write(to, &large_test_retained_modified()))
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

pub(crate) fn run_relocate_weakened_case(label: &str) -> bool {
    let from = "crates/workspace_tests/tests/behavior.rs";
    let to = "crates/workspace_tests/tests/behavior_relocated.rs";
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write(from, &large_test_text())
        .and_then(|()| fixture.commit_base())
        .and_then(|()| fixture.rename(from, to))
        .and_then(|()| fixture.write(to, &large_test_weakened()))
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
    let expected = [("WeakenedAssertion", from)];
    let findings_ok = expect_findings(label, &findings, &expected);
    println!("SelfTest:{label}: exit expected=1 actual={exit}");
    findings_ok && exit == 1
}

pub(crate) fn run_relocate_spaces_case(label: &str) -> bool {
    let from = "crates/workspace_tests/tests/behavior with spaces.rs";
    let to = "crates/workspace_tests/tests/behavior moved.rs";
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

pub(crate) fn run_relocate_invalid_utf8_case(label: &str) -> bool {
    let from = "crates/workspace_tests/tests/behavior.rs";
    let to = "crates/workspace_tests/tests/behavior_relocated.rs";
    let mut invalid = BASE_TEXT.as_bytes().to_vec();
    invalid.extend_from_slice(&[0xff, 0x00]);
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write_bytes(from, &invalid)
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
    let expected = [("NonUtf8TestFile", from)];
    let findings_ok = expect_findings(label, &findings, &expected);
    println!("SelfTest:{label}: exit expected=1 actual={exit}");
    findings_ok && exit == 1
}

pub(crate) fn run_non_utf8_relevant_case(label: &str) -> bool {
    let path = "crates/workspace_tests/tests/behavior.rs";
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write(path, BASE_TEXT)
        .and_then(|()| fixture.commit_base());
    if let Err(error) = setup {
        return report_failure(label, &format!("setup: {error}"));
    }
    // Relevant .rs file with a NUL byte and trailing invalid UTF-8 (0xff 0x00).
    let mutated = [
        0x23u8, 0x5b, 0x74, 0x65, 0x73, 0x74, 0x5d, 0x0a, 0x66, 0x6e, 0x20, 0x62, 0x65, 0x68, 0x61,
        0x76, 0x69, 0x6f, 0x72, 0x5f, 0x74, 0x65, 0x73, 0x74, 0x28, 0x29, 0x20, 0x7b, 0x0a, 0x20,
        0x20, 0x20, 0x20, 0x61, 0x73, 0x73, 0x65, 0x72, 0x74, 0x5f, 0x65, 0x71, 0x28, 0x64, 0x75,
        0x6d, 0x6d, 0x79, 0x29, 0x3b, 0x0a, 0x7d, 0x0a, 0xff, 0x00,
    ];
    if let Err(error) = fixture.write_bytes(path, &mutated) {
        return report_failure(label, &format!("mutate: {error}"));
    }
    let findings = match fixture.collect() {
        Ok(findings) => findings,
        Err(error) => return report_failure(label, &format!("collect: {error}")),
    };
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => return report_failure(label, &format!("check: {error}")),
    };
    let expected = [("NonUtf8TestFile", path)];
    let findings_ok = expect_findings(label, &findings, &expected);
    println!("SelfTest:{label}: exit expected=1 actual={exit}");
    findings_ok && exit == 1
}

pub(crate) fn run_unrelated_binary_pass_case(label: &str) -> bool {
    let path = "crates/workspace_tests/tests/behavior.rs";
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write(path, BASE_TEXT)
        .and_then(|()| fixture.commit_base());
    if let Err(error) = setup {
        return report_failure(label, &format!("setup: {error}"));
    }
    let binary = [0xffu8, 0xfe, 0x00, 0x01, 0x80, 0x81, 0x82, 0x83, 0x84, 0x85];
    if let Err(error) = fixture.write_bytes(".beads/evidence/proof.dat", &binary) {
        return report_failure(label, &format!("write evidence: {error}"));
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

pub(crate) fn run_binary_with_test_delete_case(label: &str) -> bool {
    let path = "crates/workspace_tests/tests/behavior.rs";
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
    let binary = [0xffu8, 0xfe, 0x00, 0x01, 0x80, 0x81, 0x82, 0x83, 0x84, 0x85];
    if let Err(error) = fixture.write_bytes(".beads/evidence/proof.dat", &binary) {
        return report_failure(label, &format!("write evidence: {error}"));
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
