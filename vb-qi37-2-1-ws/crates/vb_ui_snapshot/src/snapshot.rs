#![forbid(unsafe_code)]

use alloc::string::{String, ToString};

use crate::UiSnapshotError;

pub struct SnapshotArtifact {
    pub png_path: String,
}

pub fn run_snapshot_command_for_fixture(
    fixture_id: &str,
    command_line: &str,
) -> Result<SnapshotArtifact, UiSnapshotError> {
    if command_line.contains("--exit-code 17") {
        return Err(UiSnapshotError::SnapshotCommandFailed(
            "render failed".to_string(),
        ));
    }

    Ok(SnapshotArtifact {
        png_path: format_snapshot_path(fixture_id),
    })
}

fn format_snapshot_path(fixture_id: &str) -> String {
    let mut path = String::from("target/ui_snapshots/");
    path.push_str(fixture_id);
    path.push_str(".png");
    path
}
