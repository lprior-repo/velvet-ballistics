#[test]
fn debug_trim_wrap() {
    use vb_storage::error::JournalError;
    use vb_storage::EventSeq;
    use vb_storage::trimming::TrimError;
    use vb_core::ids::RunId;
    
    let inner = JournalError::SequenceGap {
        expected: EventSeq::new(1),
        actual: EventSeq::new(3),
    };
    println!("inner.diagnostic_code() = {:?}", inner.diagnostic_code());
    println!("inner.symbolic_code() = {:?}", inner.symbolic_code());
    
    let inner_code = inner.diagnostic_code();
    println!("inner_code.symbolic_code() = {:?}", inner_code.symbolic_code());
    
    let wrapped = JournalError::Trim(Box::new(TrimError::Journal(inner)));
    println!("wrapped.diagnostic_code() = {:?}", wrapped.diagnostic_code());
    println!("wrapped.symbolic_code() = {:?}", wrapped.symbolic_code());
    
    panic!("debug");
}
