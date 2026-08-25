//! Reusable egui widgets for dev panels.
//!
//! These are plain `egui` drawing functions with no Bevy coupling: hosts call
//! them from any tab or panel closure with their own data and apply the
//! reported interactions themselves.

pub mod bitmask;
pub mod cards;

pub use bitmask::{bitmask_field, bitmask_field_with};
pub use cards::{Card, CardAction, draw_cards, draw_cards_with_salt};
