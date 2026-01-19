//! Performance diagnostics tab.
//!
//! Displays FPS, frame time, and entity counts.

use bevy::prelude::*;
use bevy_egui::egui;

/// Render the diagnostics tab.
pub fn render(ui: &mut egui::Ui, world: &World) {
    ui.heading("Performance");
    ui.separator();

    let Some(time) = world.get_resource::<Time>() else {
        ui.label("Time resource not available");
        return;
    };
    let fps = 1.0 / time.delta_secs();
    let frame_time = time.delta_secs() * 1000.0;

    // Performance metrics in a grid
    ui.columns(2, |columns| {
        columns[0].label("FPS:");
        columns[1].colored_label(
            if fps >= 60.0 {
                egui::Color32::GREEN
            } else if fps >= 30.0 {
                egui::Color32::YELLOW
            } else {
                egui::Color32::RED
            },
            format!("{fps:.1} Hz"),
        );

        columns[0].label("Frame Time:");
        columns[1].colored_label(
            if frame_time <= 16.7 {
                egui::Color32::GREEN
            } else if frame_time <= 33.3 {
                egui::Color32::YELLOW
            } else {
                egui::Color32::RED
            },
            format!("{frame_time:.1} ms"),
        );
    });

    ui.add_space(10.0);
    ui.heading("Entities");
    ui.separator();

    // Entity counts
    let total_entities = world.entities().len();

    ui.columns(2, |columns| {
        columns[0].label("Total Entities:");
        columns[1].label(format!("{total_entities}"));
    });
}
