//! Game viewport tab.
//!
//! Shows the game's render image letterboxed into the tab.

use bevy::{
    prelude::*,
    window::{PrimaryWindow, Window},
};
use bevy_egui::egui;

use crate::viewport::GameViewImage;

/// Render the game view tab.
///
/// Draws [`GameViewImage`] at the window's aspect ratio, centred in the tab
/// with black bars, and records where it was drawn in `viewport_rect`.
pub fn render(ui: &mut egui::Ui, world: &mut World, viewport_rect: &mut egui::Rect) {
    let area = ui.clip_rect();
    let Some(texture_id) = world
        .get_resource::<GameViewImage>()
        .map(|view| view.texture_id)
    else {
        *viewport_rect = area;
        return;
    };
    let window_size = world
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .single(world)
        .map(|window| egui::vec2(window.width(), window.height()))
        .unwrap_or(area.size());
    *viewport_rect = letterbox(area, window_size);

    let painter = ui.painter_at(area);
    painter.rect_filled(area, 0.0, egui::Color32::BLACK);
    painter.image(
        texture_id,
        *viewport_rect,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
}

/// The largest rect with `content`'s aspect ratio centred inside `area`.
fn letterbox(area: egui::Rect, content: egui::Vec2) -> egui::Rect {
    if content.x <= 0.0 || content.y <= 0.0 {
        return area;
    }
    let scale = (area.width() / content.x).min(area.height() / content.y);
    egui::Rect::from_center_size(area.center(), content * scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letterbox_keeps_aspect_and_centres() {
        let area = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(400.0, 100.0));
        let shown = letterbox(area, egui::vec2(160.0, 90.0));
        assert!((shown.width() / shown.height() - 160.0 / 90.0).abs() < 1e-4);
        assert!((shown.height() - area.height()).abs() < 1e-4);
        assert!((shown.center() - area.center()).length() < 1e-4);
    }

    #[test]
    fn letterbox_fits_tall_area_by_width() {
        let area = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(100.0, 400.0));
        let shown = letterbox(area, egui::vec2(160.0, 90.0));
        assert!((shown.width() - area.width()).abs() < 1e-4);
        assert!(area.contains_rect(shown));
    }

    #[test]
    fn letterbox_of_empty_content_is_the_area() {
        let area = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(100.0, 50.0));
        assert_eq!(letterbox(area, egui::Vec2::ZERO), area);
    }
}
