use crate::{CompileError, CompileErrors};

pub(super) fn emitted_at_once(
    index: usize,
    raw: Option<u32>,
    max_fanout: u16,
) -> Result<Option<u16>, CompileErrors> {
    let Some(requested) = raw.filter(|value| *value > 1) else {
        return Ok(None);
    };
    let cap = u32::from(max_fanout);
    u16::try_from(requested.min(cap)).map(Some).map_err(|_| {
        CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "for_each.at_once",
            expected: "a positive concurrency upper bound",
        }])
    })
}
