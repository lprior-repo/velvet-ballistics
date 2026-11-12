use super::JUSTIFICATIONS_REL;
use super::check_test_integrity_fixture::{
    WEAKEN_DETAIL, expect_findings, justification_fixture, justification_record, report_failure,
};

pub(crate) fn run_justification_tests() -> bool {
    let (fixture, test_path, base_blob, new_blob, replacement_entry) =
        match justification_fixture("justify") {
            Ok(values) => values,
            Err(error) => return report_failure("justify", &format!("fixture setup: {error}")),
        };
    let base_commit = match fixture.head_hash() {
        Ok(hash) => hash,
        Err(error) => return report_failure("justify", &format!("head hash: {error}")),
    };

    let review_path = ".beads/vb-test-fixture/test-review.md";
    let review_blob = match fixture.current_blob(review_path) {
        Ok(Some(blob)) => blob,
        Ok(None) => return report_failure("justify", "review blob missing"),
        Err(error) => return report_failure("justify", &format!("review blob: {error}")),
    };

    // Sanity: the fixture yields exactly the WeakenedAssertion finding.
    let pre = match fixture.collect() {
        Ok(findings) => findings,
        Err(error) => return report_failure("justify", &format!("pre-collect: {error}")),
    };
    if !expect_findings(
        "justify-pre",
        &pre,
        &[("WeakenedAssertion", test_path.as_str())],
    ) {
        return false;
    }

    // 1. Valid record passes (finding justified).
    let valid_tsv = justification_record(
        &base_commit,
        &test_path,
        &base_blob,
        &new_blob,
        &replacement_entry,
        &review_blob,
    );
    let mut ok = true;
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &valid_tsv) {
        return report_failure("justify-valid", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            ok = false;
            eprintln!("SelfTest:justify-valid: check error: {error}");
            2
        }
    };
    println!(
        "SelfTest:justify-valid: {} expected=0 actual={exit}",
        if exit == 0 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 0;

    // 2. Stale current test blob -> finding stays -> FAIL (exit 1).
    let stale_new = "0".repeat(40);
    let stale_test_tsv = justification_record(
        &base_commit,
        &test_path,
        &base_blob,
        &stale_new,
        &replacement_entry,
        &review_blob,
    );
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &stale_test_tsv) {
        return report_failure("justify-stale-test", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            ok = false;
            eprintln!("SelfTest:justify-stale-test: check error: {error}");
            2
        }
    };
    println!(
        "SelfTest:justify-stale-test: {} expected=1 actual={exit}",
        if exit == 1 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 1;

    // 3. Stale base blob -> finding stays -> FAIL (exit 1).
    let stale_base = "0".repeat(40);
    let stale_base_tsv = justification_record(
        &base_commit,
        &test_path,
        &stale_base,
        &new_blob,
        &replacement_entry,
        &review_blob,
    );
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &stale_base_tsv) {
        return report_failure("justify-stale-base", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            ok = false;
            eprintln!("SelfTest:justify-stale-base: check error: {error}");
            2
        }
    };
    println!(
        "SelfTest:justify-stale-base: {} expected=1 actual={exit}",
        if exit == 1 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 1;

    // 4. Stale replacement blob -> finding stays -> FAIL (exit 1).
    let replacement_path = "crates/workspace_tests/tests/replacement_coverage.rs";
    let stale_replacement_entry = format!("{replacement_path}:{}", "0".repeat(40));
    let stale_replacement_tsv = justification_record(
        &base_commit,
        &test_path,
        &base_blob,
        &new_blob,
        &stale_replacement_entry,
        &review_blob,
    );
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &stale_replacement_tsv) {
        return report_failure("justify-stale-replacement", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            ok = false;
            eprintln!("SelfTest:justify-stale-replacement: check error: {error}");
            2
        }
    };
    println!(
        "SelfTest:justify-stale-replacement: {} expected=1 actual={exit}",
        if exit == 1 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 1;

    // 5. Stale review artifact blob -> finding stays -> FAIL (exit 1).
    let stale_review = "0".repeat(40);
    let stale_review_tsv = justification_record(
        &base_commit,
        &test_path,
        &base_blob,
        &new_blob,
        &replacement_entry,
        &stale_review,
    );
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &stale_review_tsv) {
        return report_failure("justify-stale-review", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            ok = false;
            eprintln!("SelfTest:justify-stale-review: check error: {error}");
            2
        }
    };
    println!(
        "SelfTest:justify-stale-review: {} expected=1 actual={exit}",
        if exit == 1 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 1;

    // 6. Wrong base commit -> inactive record is ignored, so the real current
    //    finding stays un-justified -> FAIL (exit 1), not a stale error.
    let wrong_base_tsv = justification_record(
        "0000000000000000000000000000000000000000",
        &test_path,
        &base_blob,
        &new_blob,
        &replacement_entry,
        &review_blob,
    );
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &wrong_base_tsv) {
        return report_failure("justify-wrong-base", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            ok = false;
            eprintln!("SelfTest:justify-wrong-base: check error: {error}");
            2
        }
    };
    println!(
        "SelfTest:justify-wrong-base: {} expected=1 actual={exit}",
        if exit == 1 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 1;

    // 7. Unknown path -> no matching finding -> error (exit 2).
    let unknown_path_tsv = format!(
        "{base_commit}\tcrates/workspace_tests/tests/other.rs\t{base_blob}\tcrates/workspace_tests/tests/other.rs\t{new_blob}\tWeakenedAssertion\t{WEAKEN_DETAIL}\t{replacement_entry}\t{review_path}\t{review_blob}\tvb-test-fixture\n"
    );
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &unknown_path_tsv) {
        return report_failure("justify-unknown-path", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("SelfTest:justify-unknown-path: rejected with: {error}");
            2
        }
    };
    println!(
        "SelfTest:justify-unknown-path: {} expected=2 actual={exit}",
        if exit == 2 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 2;

    // 8. Forbidden kind (NonUtf8TestFile) -> rejected at load (exit 2).
    let forbidden_tsv = format!(
        "{base_commit}\t{test_path}\t{base_blob}\t{test_path}\t{new_blob}\tNonUtf8TestFile\trelevant test file contains non-UTF8 bytes\t{replacement_entry}\t{review_path}\t{review_blob}\tvb-test-fixture\n"
    );
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &forbidden_tsv) {
        return report_failure("justify-forbidden-nonutf8", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("SelfTest:justify-forbidden-nonutf8: rejected with: {error}");
            2
        }
    };
    println!(
        "SelfTest:justify-forbidden-nonutf8: {} expected=2 actual={exit}",
        if exit == 2 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 2;

    // 9. Malformed record (wrong column count) -> error (exit 2).
    let malformed_tsv = "HEAD\tpath\tABSENT\n";
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, malformed_tsv) {
        return report_failure("justify-malformed", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("SelfTest:justify-malformed: rejected with: {error}");
            2
        }
    };
    println!(
        "SelfTest:justify-malformed: {} expected=2 actual={exit}",
        if exit == 2 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 2;

    // 10. Duplicate identity -> error (exit 2).
    let duplicate_tsv = format!("{valid_tsv}{valid_tsv}");
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &duplicate_tsv) {
        return report_failure("justify-duplicate", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("SelfTest:justify-duplicate: rejected with: {error}");
            2
        }
    };
    println!(
        "SelfTest:justify-duplicate: {} expected=2 actual={exit}",
        if exit == 2 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 2;

    // 11. Code-level skip (silent return) without replacement -> rejected (exit 2).
    let skip_tsv = format!(
        "{base_commit}\t{test_path}\t{base_blob}\t{test_path}\t{new_blob}\tIgnoredOrSkippedTest\treturn;\t-\t{review_path}\t{review_blob}\tvb-test-fixture\n"
    );
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &skip_tsv) {
        return report_failure("justify-forbidden-skip", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("SelfTest:justify-forbidden-skip: rejected with: {error}");
            2
        }
    };
    println!(
        "SelfTest:justify-forbidden-skip: {} expected=2 actual={exit}",
        if exit == 2 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 2;

    ok
}
