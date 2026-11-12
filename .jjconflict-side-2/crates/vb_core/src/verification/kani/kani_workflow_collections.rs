fn kani_step_name(index: u8) -> Box<str> {
    match index {
        0 => Box::from("step_0"),
        1 => Box::from("step_1"),
        2 => Box::from("step_2"),
        _ => Box::from("step_3"),
    }
}

fn bounded_path() -> Box<[PathSegment]> {
    match bounded_len_3() {
        0 => Box::new([]),
        1 => Box::new([kani::any::<PathSegment>()]),
        2 => Box::new([kani::any::<PathSegment>(), kani::any::<PathSegment>()]),
        _ => Box::new([
            kani::any::<PathSegment>(),
            kani::any::<PathSegment>(),
            kani::any::<PathSegment>(),
        ]),
    }
}

fn bounded_accessors() -> Box<[AccessorProgram]> {
    match bounded_len_3() {
        0 => Box::new([]),
        1 => Box::new([kani::any::<AccessorProgram>()]),
        2 => Box::new([
            kani::any::<AccessorProgram>(),
            kani::any::<AccessorProgram>(),
        ]),
        _ => Box::new([
            kani::any::<AccessorProgram>(),
            kani::any::<AccessorProgram>(),
            kani::any::<AccessorProgram>(),
        ]),
    }
}

fn bounded_expr_branches() -> Box<[ExprBranch]> {
    match bounded_len_2() {
        0 => Box::new([]),
        1 => Box::new([kani::any::<ExprBranch>()]),
        _ => Box::new([kani::any(), kani::any()]),
    }
}

fn bounded_slot_branches() -> Box<[SlotBranch]> {
    match bounded_len_2() {
        0 => Box::new([]),
        1 => Box::new([kani::any::<SlotBranch>()]),
        _ => Box::new([kani::any(), kani::any()]),
    }
}

fn bounded_step_indices() -> Box<[StepIdx]> {
    match bounded_len_2() {
        0 => Box::new([]),
        1 => Box::new([StepIdx::new(kani::any())]),
        _ => Box::new([StepIdx::new(kani::any()), StepIdx::new(kani::any())]),
    }
}

fn bounded_len_3() -> u8 {
    let len: u8 = kani::any();
    kani::assume(len <= 3);
    len
}

fn bounded_len_2() -> u8 {
    let len: u8 = kani::any();
    kani::assume(len <= 2);
    len
}
