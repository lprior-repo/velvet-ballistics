use super::*;

#[test]
fn validated_foreach_backedge_inside_lane() -> Result<(), Box<dyn std::error::Error>> {
    let parts = legacy_fixtures::foreach_within_group();
    let regions = crate::workflow::concurrent_analyze::analyze(&parts)?;
    assert_eq!(regions.len(), 1, "expected exactly one concurrent region");
    let r = regions.first().ok_or("missing region")?;
    assert_eq!(r.owner, StepIdx::new(0));
    assert_eq!(r.join, StepIdx::new(6));
    assert_eq!(r.parent, None);
    assert_eq!(r.lanes.len(), 1);
    let lane = r.lanes.first().ok_or("missing lane")?;
    assert_eq!(lane.entry, StepIdx::new(1));
    assert_eq!(lane.ends.as_ref(), &[StepIdx::new(5)]);
    assert_eq!(lane.children.len(), 0);
    Ok(())
}

#[test]
fn validated_foreach_backedge_around_group() -> Result<(), Box<dyn std::error::Error>> {
    let parts = legacy_fixtures::group_within_foreach();
    let regions = crate::workflow::concurrent_analyze::analyze(&parts)?;
    assert_eq!(regions.len(), 1, "expected exactly one concurrent region");
    let r = regions.first().ok_or("missing region")?;
    assert_eq!(r.owner, StepIdx::new(1));
    assert_eq!(r.join, StepIdx::new(3));
    assert_eq!(r.parent, None);
    assert_eq!(r.lanes.len(), 1);
    let lane = r.lanes.first().ok_or("missing lane")?;
    assert_eq!(lane.entry, StepIdx::new(2));
    assert_eq!(lane.ends.as_ref(), &[StepIdx::new(2)]);
    assert_eq!(lane.children.len(), 0);
    Ok(())
}
