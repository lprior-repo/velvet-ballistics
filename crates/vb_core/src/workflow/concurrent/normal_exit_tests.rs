use super::*;
use crate::workflow::ErrorRouting;

#[test]
fn successful_finish_cannot_hide_behind_error_handler() -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = super::legacy_fixtures::foreach_within_group();

    let body = parts.nodes.get_mut(2).ok_or("fixture")?;
    body.kind = K::Finish {
        result: SlotIdx::new(0),
    };
    body.output = None.into();
    body.routing.next = None.into();
    body.routing.error = ErrorRouting::Handler {
        handler: StepIdx::new(3),
        error_slot: None,
    };

    let result = crate::workflow::concurrent_analyze::analyze(&parts);
    assert_eq!(
        result,
        Err(WorkflowError::LoopRegionUnpaired {
            control: StepIdx::new(0)
        })
    );
    Ok(())
}
