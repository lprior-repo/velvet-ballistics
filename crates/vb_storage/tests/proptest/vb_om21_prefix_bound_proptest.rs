#![forbid(unsafe_code)]
// Obligation: PO-vb-om21-prefix-bound-proptest
use proptest::prelude::*;
use vb_core::RunId;
use vb_storage::{keys::run_event_key, types::EventSeq};
proptest! {
  #[test]
  fn vb_om21_prefix_bound_proptest(run in any::<u64>(), a in any::<u64>(), b in any::<u64>()) {
    let run = RunId::new(run);
    let ka = run_event_key(run, EventSeq::new(a));
    let kb = run_event_key(run, EventSeq::new(b));
    match (ka, kb) {
      (Ok(ka), Ok(kb)) => {
        prop_assert_eq!(ka[0], 0x11);
        prop_assert_eq!((a <= b), (ka <= kb));
      }
      (Err(err), _) | (_, Err(err)) => {
        prop_assert!(false, "key encoding returned error: {:?}", err);
      }
    }
  }
}
