use super::support::{FINISH, Fixture, NEXT, ROOT, TestResult, VALUES};
use vb_core::{SlotValue, StepIdx, Taint};
use vb_runtime::engine::EvidenceEvent;

pub(super) fn assert_pages_and_order(
    fixture: &Fixture,
    events: &[EvidenceEvent],
    after: bool,
) -> TestResult {
    sequence(fixture, events, after)?;
    let collector = fixture.output(StepIdx::new(0))?;
    let pages: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            EvidenceEvent::SlotWritten {
                slot,
                value: SlotValue::List(id),
                taint,
                extra,
            } if *slot == collector => Some((*id, *taint, *extra)),
            _ => None,
        })
        .collect();
    assert_eq!(pages.len(), 4);
    for (page, (values, cursor)) in pages.iter().take(3).zip(VALUES.chunks(2).zip([2, 4, 5])) {
        page_evidence(fixture, *page, values, cursor)?;
    }
    assert_eq!(pages.last(), Some(&(ROOT, Taint::Secret, None)));
    if after {
        finish_output(fixture, events)?;
    }
    Ok(())
}

fn sequence(fixture: &Fixture, events: &[EvidenceEvent], after: bool) -> TestResult {
    let expected: &[u16] = if after {
        &[0, 1, 2, 3, 1, 2, 3, 1, 2, 3, 4]
    } else {
        &[0, 1, 2, 3, 1, 2, 3, 1, 2, 3]
    };
    let triples = events.chunks_exact(3);
    assert!(
        triples.remainder().is_empty(),
        "evidence events must form complete triples, got remainder: {:?}",
        triples.remainder()
    );
    assert_eq!(
        triples.len(),
        expected.len(),
        "evidence triple count must match expected steps"
    );
    for (triple, raw) in triples.zip(expected) {
        let step = StepIdx::new(*raw);
        let output = if step == NEXT {
            None
        } else {
            Some(fixture.output(step)?)
        };
        assert_eq!(triple.first(), Some(&EvidenceEvent::StepStarted { step }));
        assert_eq!(
            triple.last(),
            Some(&EvidenceEvent::StepSucceeded { step, output })
        );
    }
    Ok(())
}

type PageEvidence = (
    vb_core::ListId,
    Taint,
    Option<vb_runtime::primitives::collect::CollectPaginationState>,
);

fn page_evidence(
    fixture: &Fixture,
    (id, taint, extra): PageEvidence,
    values: &[SlotValue],
    cursor: usize,
) -> TestResult {
    assert_eq!(fixture.state.store.list(id)?, values);
    assert_eq!(taint, Taint::Secret);
    let live = extra.ok_or("page missing live evidence")?;
    assert_eq!(live.source, ROOT);
    assert_eq!(live.current_page, id);
    assert_eq!(live.collector_slot, fixture.output(StepIdx::new(0))?);
    assert_eq!(live.run_id, fixture.expected.run);
    assert_eq!(live.cursor, cursor);
    assert_eq!(live.item_count, 5);
    assert_eq!(live.page_size, 2);
    assert_eq!(live.limit, 6);
    Ok(())
}

fn finish_output(fixture: &Fixture, events: &[EvidenceEvent]) -> TestResult {
    let output = fixture.output(FINISH)?;
    let writes: Vec<_> = events
        .iter()
        .filter(|event| {
            matches!(event,
                EvidenceEvent::SlotWritten { slot, value, taint, extra }
                if *slot == output && *value == SlotValue::List(ROOT)
                    && *taint == Taint::Secret && extra.is_none())
        })
        .collect();
    assert_eq!(writes.len(), 1);
    Ok(())
}

#[cfg(test)]
mod sequence_guard_probes {
    use super::super::support::{TestResult, drive, fixture};
    use super::sequence;

    /// Valid 30-event cut stream passes both guards without firing.
    #[test]
    fn valid_cut_stream_passes_both_guards() -> TestResult {
        let mut f = fixture()?;
        let (_, events) = drive(&mut f.state, 10)?;
        assert_eq!(events.len(), 30);
        sequence(&f, &events, false)?;
        Ok(())
    }

    /// 28 events leave a one-event remainder: the remainder guard must fire.
    #[test]
    #[should_panic(expected = "complete triples")]
    fn truncated_remainder_is_rejected() {
        let mut f = fixture().expect("fixture must build");
        let (_, mut events) = drive(&mut f.state, 10).expect("drive must succeed");
        events.truncate(28);
        let _ = sequence(&f, &events, false);
    }

    /// 33 events with a 10-step expectation are one triple too many.
    #[test]
    #[should_panic(expected = "triple count")]
    fn extra_triple_count_mismatch_is_rejected() {
        let mut f = fixture().expect("fixture must build");
        let (_, mut events) = drive(&mut f.state, 10).expect("drive must succeed");
        events.extend_from_within(27..30);
        let _ = sequence(&f, &events, false);
    }
}
