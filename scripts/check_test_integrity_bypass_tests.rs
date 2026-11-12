use super::JUSTIFICATIONS_REL;
use super::check_test_integrity_fixture::{
    BASE_TEXT, Fixture, WEAKEN_DETAIL, justification_fixture, report_failure,
};

pub(crate) fn run_justification_bypass_tests() -> bool {
    let (fixture, test_path, base_blob, _new_blob, _replacement_entry) =
        match justification_fixture("bypass") {
            Ok(values) => values,
            Err(error) => return report_failure("bypass", &format!("fixture setup: {error}")),
        };
    let base_commit = match fixture.head_hash() {
        Ok(hash) => hash,
        Err(error) => return report_failure("bypass", &format!("head hash: {error}")),
    };
    let review_path = ".beads/vb-test-fixture/test-review.md";
    let review_blob = match fixture.current_blob(review_path) {
        Ok(Some(blob)) => blob,
        Ok(None) => return report_failure("bypass", "review blob missing"),
        Err(error) => return report_failure("bypass", &format!("review blob: {error}")),
    };

    let mut ok = true;

    // Bypass #1: old_path still exists (weakened) but new_path binds an
    // unrelated file. The finding must stay un-justified (exit 1).
    if let Err(error) = fixture.write("README.md", "# Unrelated\n") {
        return report_failure("bypass-unrelated", &format!("write README: {error}"));
    }
    let unrelated_blob = match fixture.current_blob("README.md") {
        Ok(Some(blob)) => blob,
        _ => return report_failure("bypass-unrelated", "unrelated blob missing"),
    };
    let unrelated_tsv = format!(
        "{base_commit}\t{test_path}\t{base_blob}\tREADME.md\t{unrelated_blob}\tWeakenedAssertion\t{WEAKEN_DETAIL}\t-\t{review_path}\t{review_blob}\tvb-test-fixture\n"
    );
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &unrelated_tsv) {
        return report_failure("bypass-unrelated", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            ok = false;
            eprintln!("SelfTest:bypass-unrelated: check error: {error}");
            2
        }
    };
    println!(
        "SelfTest:bypass-unrelated: {} expected=1 actual={exit}",
        if exit == 1 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 1;

    // Bypass #2: new_path=ABSENT while the original still exists. The finding
    // must stay un-justified (exit 1).
    let absent_tsv = format!(
        "{base_commit}\t{test_path}\t{base_blob}\tABSENT\tABSENT\tWeakenedAssertion\t{WEAKEN_DETAIL}\t-\t{review_path}\t{review_blob}\tvb-test-fixture\n"
    );
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &absent_tsv) {
        return report_failure("bypass-absent", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            ok = false;
            eprintln!("SelfTest:bypass-absent: check error: {error}");
            2
        }
    };
    println!(
        "SelfTest:bypass-absent: {} expected=1 actual={exit}",
        if exit == 1 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 1;

    // A subsequent actual test change is still caught (exit 1) with the bypass
    // record present: the record cannot track or waive the changed file.
    if let Err(error) = fixture.write(
        &test_path,
        "#[test]\nfn behavior_test() {\n    assert!((2 + 2).checked_sub(4).is_some());\n}\n// changed\n",
    ) {
        return report_failure("bypass-changed", &format!("write changed test: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            ok = false;
            eprintln!("SelfTest:bypass-changed: check error: {error}");
            2
        }
    };
    println!(
        "SelfTest:bypass-changed: {} expected=1 actual={exit}",
        if exit == 1 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 1;

    ok
}

pub(crate) fn run_justification_lifecycle_tests() -> bool {
    let mut ok = true;

    // A stale record for an older base commit must not fail a clean future
    // main: after advancing to a new base with zero findings, the inactive
    // record is ignored and the gate passes (exit 0).
    let fixture = match Fixture::new("lifecycle-clean") {
        Ok(fixture) => fixture,
        Err(error) => return report_failure("lifecycle-clean", &format!("fixture setup: {error}")),
    };
    let test_path = "crates/workspace_tests/tests/behavior.rs";
    let review_path = ".beads/vb-test-fixture/test-review.md";
    let setup = fixture
        .write(test_path, BASE_TEXT)
        .and_then(|()| fixture.commit_base());
    if let Err(error) = setup {
        return report_failure("lifecycle-clean", &format!("setup: {error}"));
    }
    let old_base = match fixture.head_hash() {
        Ok(hash) => hash,
        Err(error) => return report_failure("lifecycle-clean", &format!("head hash: {error}")),
    };
    let base_blob = match fixture.base_blob(test_path) {
        Ok(Some(blob)) => blob,
        _ => return report_failure("lifecycle-clean", "base blob missing"),
    };
    let weakened =
        "#[test]\nfn behavior_test() {\n    assert!((2 + 2).checked_sub(4).is_some());\n}\n";
    let setup = fixture
        .write(test_path, weakened)
        .and_then(|()| fixture.write(review_path, "# review\n"));
    if let Err(error) = setup {
        return report_failure("lifecycle-clean", &format!("weaken: {error}"));
    }
    let new_blob = match fixture.current_blob(test_path) {
        Ok(Some(blob)) => blob,
        _ => return report_failure("lifecycle-clean", "new blob missing"),
    };
    let review_blob = match fixture.current_blob(review_path) {
        Ok(Some(blob)) => blob,
        _ => return report_failure("lifecycle-clean", "review blob missing"),
    };
    let record = format!(
        "{old_base}\t{test_path}\t{base_blob}\t{test_path}\t{new_blob}\tWeakenedAssertion\t{WEAKEN_DETAIL}\t-\t{review_path}\t{review_blob}\tvb-test-fixture\n"
    );
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &record) {
        return report_failure("lifecycle-clean", &format!("write tsv: {error}"));
    }
    // Advance to a clean "next main" base: the weakened state is committed and
    // the working tree is clean, so there are no current findings.
    if let Err(error) = fixture.commit_base() {
        return report_failure("lifecycle-clean", &format!("commit clean: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            ok = false;
            eprintln!("SelfTest:lifecycle-clean: check error: {error}");
            2
        }
    };
    println!(
        "SelfTest:lifecycle-clean: {} expected=0 actual={exit}",
        if exit == 0 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 0;

    ok
}
