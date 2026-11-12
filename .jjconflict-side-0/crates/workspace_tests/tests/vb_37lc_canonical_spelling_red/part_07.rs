use super::*;

#[test]
fn render_scan_report_writes_report_when_destination_parent_exists()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let destination = temp.path().join("naming-scan.txt");
    let mut report = zero_finding_report();
    report.report_destination = Some(destination.clone());

    let result = render_scan_report(&report);
    let written = std::fs::read_to_string(&destination)?;

    assert_eq_render_scan_report_result(
        result,
        Ok(RenderedReport {
            body: "canonical spelling scan: 0 findings; selected=0; scanned=0\n".to_string(),
        }),
    );
    assert_eq_rendered_report_body(
        written,
        "canonical spelling scan: 0 findings; selected=0; scanned=0\n",
    );
    Ok(())
}
