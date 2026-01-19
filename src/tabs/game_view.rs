//! Game viewport tab.
//!
//! Displays the game view and captures the viewport rectangle for camera clipping.

use bevy_egui::egui;

/// Render the game view tab.
///
/// This tab captures the clip rectangle which is used to set the camera viewport.
pub fn render(ui: &mut egui::Ui, viewport_rect: &mut egui::Rect) {
    *viewport_rect = ui.clip_rect();
}
