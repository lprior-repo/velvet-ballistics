use std::fs;
use std::path::{Path, PathBuf};

use proptest::prelude::*;
use vb_ui_snapshot::checks;

#[test]
fn overlap_violation_error_returns_contract_shape_when_controls_intersect() {
    let path = write_layout_fixture(
        "overlap",
        "layout_fixture=true\nkind=overlap\nscreen_id=execution_overview\nfirst_control_id=run_button\nsecond_control_id=stop_button\nfirst_rect=10,10,100,60\nsecond_rect=80,50,50,50\nlabel_rect=0,0,40,10\ncontainer_rect=0,0,100,100\nviewport_rect=0,0,1920,1080\ncontrast_milli=4500\nselected_visible=true\n",
    );
    let result = checks::check_overlap(&path)
        .map(|value| value.overlaps.len())
        .map_err(|error| format!("{error:?}"));

    assert_eq!(
        result,
        Err(String::from(
            "OverlapDetected { screen_id: \"execution_overview\", first_control_id: \"run_button\", second_control_id: \"stop_button\", overlap_area_px: 600 }"
        ))
    );
}

#[test]
fn clipping_violation_error_returns_contract_shape_when_label_exceeds_container() {
    let path = write_layout_fixture(
        "clipping",
        "layout_fixture=true\nkind=clipping\nscreen_id=execution_overview\nfirst_control_id=run_button\nsecond_control_id=stop_button\nfirst_rect=0,0,100,60\nsecond_rect=100,100,50,50\nlabel_rect=0,0,40,10\ncontainer_rect=0,0,10,10\nviewport_rect=0,0,1920,1080\ncontrast_milli=4500\nselected_visible=true\n",
    );
    let result = checks::check_clipping(&path)
        .map(|value| value.clipped_labels.len())
        .map_err(|error| format!("{error:?}"));

    assert_eq!(
        result,
        Err(String::from(
            "LabelClipped { screen_id: \"execution_overview\", control_id: \"run_button\", label_bounds: (0, 0, 40, 10), container_bounds: (0, 0, 10, 10) }"
        ))
    );
}

#[test]
fn bounds_violation_error_returns_contract_shape_when_control_exceeds_viewport() {
    let path = write_layout_fixture(
        "bounds",
        "layout_fixture=true\nkind=bounds\nscreen_id=execution_overview\nfirst_control_id=run_button\nsecond_control_id=stop_button\nfirst_rect=1900,10,40,20\nsecond_rect=100,100,50,50\nlabel_rect=0,0,40,10\ncontainer_rect=0,0,100,100\nviewport_rect=0,0,1920,1080\ncontrast_milli=4500\nselected_visible=true\n",
    );
    let result = checks::check_bounds(&path, 32, 246, 78)
        .map(|value| value.out_of_bounds_controls.len())
        .map_err(|error| format!("{error:?}"));

    assert_eq!(
        result,
        Err(String::from(
            "ControlOutOfBounds { screen_id: \"execution_overview\", control_id: \"run_button\", control_bounds: (1900, 10, 40, 20), viewport_bounds: (0, 0, 1920, 1080) }"
        ))
    );
}

#[test]
fn chip_readability_violation_error_returns_contract_shape_when_chip_has_zero_area() {
    let path = write_layout_fixture(
        "chip",
        "layout_fixture=true\nkind=chip_readability\nscreen_id=execution_overview\nfirst_control_id=run_status\nsecond_control_id=stop_button\nfirst_rect=0,0,0,0\nsecond_rect=100,100,50,50\nlabel_rect=0,0,40,10\ncontainer_rect=0,0,100,100\nviewport_rect=0,0,1920,1080\ncontrast_milli=1200\nselected_visible=true\n",
    );
    let result = checks::check_chip_readability(&path)
        .map(|value| value.unreadable_chips.len())
        .map_err(|error| format!("{error:?}"));

    assert_eq!(
        result,
        Err(String::from(
            "ChipUnreadable { screen_id: \"execution_overview\", control_id: \"run_status\", visible_area_px: 0, contrast_ratio: 1.2, threshold: 4.5 }"
        ))
    );
}

#[test]
fn selected_state_violation_error_returns_contract_shape_when_indicator_is_hidden() {
    let path = write_layout_fixture(
        "selected",
        "layout_fixture=true\nkind=selected_state\nscreen_id=workflow_graph_authoring\nfirst_control_id=node_7\nsecond_control_id=stop_button\nfirst_rect=0,0,0,0\nsecond_rect=100,100,50,50\nlabel_rect=0,0,40,10\ncontainer_rect=0,0,100,100\nviewport_rect=0,0,1920,1080\ncontrast_milli=4500\nselected_visible=false\n",
    );
    let result = checks::check_selected_state(&path)
        .map(|value| value.hidden_states.len())
        .map_err(|error| format!("{error:?}"));

    assert_eq!(
        result,
        Err(String::from(
            "SelectedStateHidden { screen_id: \"workflow_graph_authoring\", control_id: \"node_7\", selected_state_id: \"selected_indicator\", reason: \"zero-area\" }"
        ))
    );
}

fn write_layout_fixture(name: &str, content: &str) -> PathBuf {
    let path = Path::new("target/vb-nf2u-layout-tests").join(format!("{name}.txt"));
    let create_result =
        fs::create_dir_all(Path::new("target/vb-nf2u-layout-tests")).map_err(|error| error.kind());
    assert_eq!(create_result, Ok(()));
    let write_result = fs::write(&path, content).map_err(|error| error.kind());
    assert_eq!(write_result, Ok(()));
    path
}

proptest! {
    #[test]
    fn overlap_kernel_reports_exact_area_for_intersecting_rectangles(x in 0_u32..1000, y in 0_u32..1000, width in 1_u32..200, height in 1_u32..200) {
        let shifted_x = x.saturating_add(width / 2);
        let shifted_y = y.saturating_add(height / 2);

        let first = match vb_ui_snapshot::layout_kernel::Rect::new(x, y, width, height) {
            Ok(rect) => rect,
            Err(_) => {
                prop_assert!(false);
                return Ok(());
            }
        };
        let second = match vb_ui_snapshot::layout_kernel::Rect::new(shifted_x, shifted_y, width, height) {
            Ok(rect) => rect,
            Err(_) => {
                prop_assert!(false);
                return Ok(());
            }
        };

        let result = vb_ui_snapshot::layout_kernel::overlap_area_px(first, second);

        let overlap_width = width.saturating_sub(width / 2);
        let overlap_height = height.saturating_sub(height / 2);
        let expected = overlap_width.saturating_mul(overlap_height);

        prop_assert_eq!(result, Ok(expected));
    }
}
