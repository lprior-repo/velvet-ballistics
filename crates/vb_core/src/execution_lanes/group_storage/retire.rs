use super::*;

fn can_retire(control: &GroupControl) -> bool {
    matches!(control.phase(), GroupPhase::Terminal(_)) && control.live_count() == 0
}

fn extract(mut control: GroupControl) -> Reserved {
    let mut reserved = Reserved {
        slots: core::mem::take(&mut control.pool.slots),
        results: core::mem::take(&mut control.results),
        lanes: core::mem::take(&mut control.lanes),
    };
    reserved.slots.clear();
    reserved.results.clear();
    reserved.lanes.clear();
    reserved
}

impl GroupStorage {
    fn validate_retirement(&self) -> Result<(), GroupError> {
        match &self.state {
            State::Active(control) if can_retire(control) => Ok(()),
            _ => Err(GroupError::InvalidTransition),
        }
    }

    pub fn retire(&mut self) -> Result<(), GroupError> {
        self.validate_retirement()?;
        let old = core::mem::replace(&mut self.state, State::Vacant(Reserved::empty()));
        match old {
            State::Active(control) => {
                let reserved = extract(control);
                self.state = State::Vacant(reserved);
                Ok(())
            }
            State::Vacant(r) => {
                self.state = State::Vacant(r);
                Err(GroupError::InvalidTransition)
            }
        }
    }
}
