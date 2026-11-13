use super::*;

fn check_width_empty(meta: &WorkflowSourceMetadata) {
    kani::assert(meta.symbols_count() == 0, "Empty width must have 0 symbols");
    kani::assert(
        meta.symbol_bytes() == 0,
        "Empty width must have 0 symbol bytes",
    );
    let syms = meta.symbols();
    kani::assert(syms.is_empty(), "Empty width symbols slice must be empty");
}

fn check_width_one(meta: &WorkflowSourceMetadata) {
    kani::assert(meta.symbols_count() == 1, "One width must have 1 symbol");
    kani::assert(
        meta.symbol_bytes() == 6,
        "One width must have 6 symbol bytes",
    );
    let syms = meta.symbols();
    kani::assert(
        syms.iter().map(|x| x.as_ref()).eq(["step_0"]),
        "One width must have single symbol 'step_0'",
    );
}

fn check_width_two(meta: &WorkflowSourceMetadata) {
    kani::assert(meta.symbols_count() == 2, "Two width must have 2 symbols");
    kani::assert(
        meta.symbol_bytes() == 12,
        "Two width must have 12 symbol bytes",
    );
    let syms = meta.symbols();
    kani::assert(
        syms.iter().map(|x| x.as_ref()).eq(["step_0", "step_1"]),
        "Two width must have symbols 'step_0' and 'step_1'",
    );
}

fn check_requirement_length(meta: &WorkflowSourceMetadata, seed: &MetadataSeed) {
    let expected_len = if matches!(seed.width, SymbolWidth::Empty) || !seed.wants_requirement {
        0
    } else {
        1
    };
    let actual_len = meta.secret_requirements().len();
    kani::assert(
        actual_len == expected_len,
        "Requirement length must match seed",
    );
}

fn check_requirement_ids(meta: &WorkflowSourceMetadata, seed: &MetadataSeed) {
    let reqs = meta.secret_requirements();
    if let Some(req) = reqs.first() {
        let expected_alias = if matches!(seed.width, SymbolWidth::Two) && seed.alias_second {
            1
        } else {
            0
        };
        kani::assert(
            req.alias().get() == expected_alias,
            "Alias ID must match seed",
        );
        let expected_req = if matches!(seed.width, SymbolWidth::Two) && seed.requirement_second {
            1
        } else {
            0
        };
        kani::assert(
            req.requirement().get() == expected_req,
            "Requirement ID must match seed",
        );
        kani::assert(req.slot().is_none(), "Slot must be None for from_seed");
    }
}

fn cover_none_shape(outcome: &Option<Box<WorkflowSourceMetadata>>) {
    kani::cover!(outcome.is_none(), "from_seed returns None");
}

fn cover_some_empty_shape(outcome: &Option<Box<WorkflowSourceMetadata>>) {
    if let Some(meta) = outcome.as_deref() {
        kani::cover!(
            meta.symbols_count() == 0,
            "from_seed returns Some with 0 symbols"
        );
    }
}

fn cover_one_no_req_shape(outcome: &Option<Box<WorkflowSourceMetadata>>) {
    if let Some(meta) = outcome.as_deref() {
        kani::cover!(
            meta.symbols_count() == 1 && meta.secret_requirements().is_empty(),
            "One width no requirement"
        );
    }
}

fn cover_one_req_shape(outcome: &Option<Box<WorkflowSourceMetadata>>) {
    if let Some(meta) = outcome.as_deref() {
        let reqs = meta.secret_requirements();
        if let Some(req) = reqs.first() {
            kani::cover!(
                meta.symbols_count() == 1 && req.alias().get() == 0 && req.requirement().get() == 0,
                "One width req 0->0"
            );
        }
    }
}

fn cover_two_no_req_shape(outcome: &Option<Box<WorkflowSourceMetadata>>) {
    if let Some(meta) = outcome.as_deref() {
        kani::cover!(
            meta.symbols_count() == 2 && meta.secret_requirements().is_empty(),
            "Two width no requirement"
        );
    }
}

fn cover_two_req_00_shape(outcome: &Option<Box<WorkflowSourceMetadata>>) {
    if let Some(meta) = outcome.as_deref() {
        let reqs = meta.secret_requirements();
        if let Some(req) = reqs.first() {
            kani::cover!(
                meta.symbols_count() == 2 && req.alias().get() == 0 && req.requirement().get() == 0,
                "Two width req 0->0"
            );
        }
    }
}

fn cover_two_req_01_shape(outcome: &Option<Box<WorkflowSourceMetadata>>) {
    if let Some(meta) = outcome.as_deref() {
        let reqs = meta.secret_requirements();
        if let Some(req) = reqs.first() {
            kani::cover!(
                meta.symbols_count() == 2 && req.alias().get() == 0 && req.requirement().get() == 1,
                "Two width req 0->1"
            );
        }
    }
}

fn cover_two_req_10_shape(outcome: &Option<Box<WorkflowSourceMetadata>>) {
    if let Some(meta) = outcome.as_deref() {
        let reqs = meta.secret_requirements();
        if let Some(req) = reqs.first() {
            kani::cover!(
                meta.symbols_count() == 2 && req.alias().get() == 1 && req.requirement().get() == 0,
                "Two width req 1->0"
            );
        }
    }
}

fn cover_two_req_11_shape(outcome: &Option<Box<WorkflowSourceMetadata>>) {
    if let Some(meta) = outcome.as_deref() {
        let reqs = meta.secret_requirements();
        if let Some(req) = reqs.first() {
            kani::cover!(
                meta.symbols_count() == 2 && req.alias().get() == 1 && req.requirement().get() == 1,
                "Two width req 1->1"
            );
        }
    }
}

fn cover_two_req_shapes(outcome: &Option<Box<WorkflowSourceMetadata>>) {
    cover_two_req_00_shape(outcome);
    cover_two_req_01_shape(outcome);
    cover_two_req_10_shape(outcome);
    cover_two_req_11_shape(outcome);
}

fn cover_all_shapes(outcome: &Option<Box<WorkflowSourceMetadata>>) {
    cover_none_shape(outcome);
    cover_some_empty_shape(outcome);
    cover_one_no_req_shape(outcome);
    cover_one_req_shape(outcome);
    cover_two_no_req_shape(outcome);
    cover_two_req_shapes(outcome);
}

#[kani::proof]
#[kani::unwind(16)]
fn metadata_bounded_constructor_is_valid() {
    let seed = kani::any::<MetadataSeed>();
    let outcome = from_seed(seed);
    kani::assert(
        outcome.is_some() == seed.present,
        "presence must match seed.present",
    );
    if let Some(meta) = outcome.as_deref() {
        if matches!(seed.width, SymbolWidth::Empty) {
            check_width_empty(meta);
        } else if matches!(seed.width, SymbolWidth::One) {
            check_width_one(meta);
        } else if matches!(seed.width, SymbolWidth::Two) {
            check_width_two(meta);
        }
        check_requirement_length(meta, &seed);
        check_requirement_ids(meta, &seed);
    }
    cover_all_shapes(&outcome);
}
