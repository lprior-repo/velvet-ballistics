use vb_core::workflow::CompiledNodeKind;

fn structural_fixtures() -> &'static [(&'static str, &'static [u8])] {
    &[
        ("Nop", &[0]),
        ("SetConst", &[1, 0]),
        ("Copy", &[2, 0]),
        ("EvalExpr", &[3, 0]),
        ("BuildObject", &[4, 0]),
        ("BuildList", &[5, 0]),
    ]
}

fn control_fixtures() -> &'static [(&'static str, &'static [u8])] {
    &[
        ("Do", &[6, 0, 0]),
        ("Choose", &[7, 0, 0]),
        ("ChooseSlot", &[8, 0, 0]),
        ("Jump", &[32, 0]),
        ("Finish", &[33, 0]),
        ("ErrorHandler", &[31, 0, 0, 0]),
    ]
}

fn iteration_fixtures() -> &'static [(&'static str, &'static [u8])] {
    &[
        ("ForEachStart", &[9, 0, 0, 1, 0, 0]),
        ("ForEachNext", &[10, 0, 0, 0]),
        ("ForEachJoin", &[11, 0]),
        ("TogetherStart", &[12, 0, 0]),
        ("TogetherBranch", &[13, 0, 0, 0, 0]),
        ("TogetherJoin", &[14, 1, 0]),
    ]
}

fn stream_fixtures() -> &'static [(&'static str, &'static [u8])] {
    &[
        ("CollectStart", &[15, 0, 1, 1, 0, 0]),
        ("CollectPage", &[16, 0, 0, 0]),
        ("CollectNext", &[17, 0, 0, 0]),
        ("CollectFinish", &[18, 0]),
        ("ReduceStart", &[19, 1, 0, 0, 0, 0, 0]),
        ("ReduceNext", &[20, 0, 0, 0, 0]),
        ("ReduceFinish", &[21, 0]),
    ]
}

fn retry_fixtures() -> &'static [(&'static str, &'static [u8])] {
    &[
        ("RepeatStart", &[22, 1, 0, 0]),
        ("RepeatAttempt", &[23, 0, 0, 0]),
        ("RepeatCheck", &[24, 0, 0]),
        ("RepeatFinish", &[25, 0]),
        ("RetryCheck", &[30, 0, 0, 0]),
        ("WaitUntil", &[26, 0]),
        ("WaitEvent", &[27, 0, 0]),
    ]
}

fn interaction_fixtures() -> &'static [(&'static str, &'static [u8])] {
    &[("Ask", &[28, 0, 0]), ("AskResume", &[29, 0])]
}

#[test]
fn legacy_kind_literals_reencode_at_pinned_producer() -> Result<(), Box<dyn std::error::Error>> {
    let all = [
        structural_fixtures(),
        control_fixtures(),
        iteration_fixtures(),
        stream_fixtures(),
        retry_fixtures(),
        interaction_fixtures(),
    ];
    for fixture in all.into_iter().flatten() {
        let (name, literal) = fixture;
        let kind = postcard::from_bytes::<CompiledNodeKind>(literal)?;
        let tag = format!("{kind:?}");
        assert!(tag.starts_with(name));
        let wire = postcard::to_allocvec(&kind)?;
        assert_eq!(wire.as_slice(), *literal);
        println!("LEGACY_KIND {name} {literal:?}");
    }
    Ok(())
}
