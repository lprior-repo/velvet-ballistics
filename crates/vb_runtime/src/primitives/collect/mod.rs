//! Collect pagination primitive handlers.

mod handlers;
mod state;

#[cfg(test)]
mod tests;

pub use handlers::{collect_finish, collect_next, collect_page, collect_start};
