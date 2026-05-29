#![allow(unused_imports)]
use vstd::prelude::*;
// PO-vb-7m21-001 implementation target: vb_storage::codec::encode_record_header / JournalError::PayloadTooLarge.
verus! { pub open spec fn payload_too_large(actual:int, declared:int, max:int)->bool { actual >= 60 && declared > max } pub proof fn po_vb_7m21_001(actual:int, declared:int, max:int) requires actual>=60, declared>max ensures payload_too_large(actual,declared,max) {} }
