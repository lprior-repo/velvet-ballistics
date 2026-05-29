#![allow(unused_imports)]
use vstd::prelude::*;
// PO-vb-7m21-011 implementation target: vb_storage::codec::decode_record_header / JournalError::UnexpectedEof.
verus! { pub open spec fn unexpected_eof(actual:int, header:int)->bool { actual < header } pub proof fn po_vb_7m21_011(actual:int, header:int) requires actual<header ensures unexpected_eof(actual,header) {} }
