use super::*;

#[path = "support/cut.rs"]
mod cut;
#[path = "support/fixtures.rs"]
mod fixtures;
use cut::*;
use fixtures::*;

#[path = "cases/actions.rs"]
mod actions;
#[path = "cases/negative.rs"]
mod negative;
#[path = "cases/positive.rs"]
mod positive;
#[path = "cases/waits.rs"]
mod waits;
