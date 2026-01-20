//! Camera viewport management for the inspector.

use bevy::{
    camera::Viewport,
    prelude::*,
    window::{PrimaryWindow, Window},
};
use bevy_inspector_egui::bevy_egui::{EguiContextSettings, PrimaryEguiContext};

use crate::state::{GameViewportRect, InspectorEnabled, UiState};

/// Marker component for the main game camera.
///
/// Games should add this component to their primary camera for viewport management.
#[derive(Component)]
pub struct InspectorMainCamera;

/// System that adjusts the camera viewport to not overlap with egui panels.
pub fn set_camera_viewport(
    ui_state: Res<UiState>,
    enabled: Res<InspectorEnabled>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut cameras: Query<&mut Camera, (With<InspectorMainCamera>, Without<PrimaryEguiContext>)>,
    mut egui_settings: Single<&mut EguiContextSettings>,
) {
    egui_settings.capture_pointer_input = false;

    let scale_factor = window.scale_factor() * egui_settings.scale_factor;

    let (viewport_pos, viewport_size) = if enabled.0 {
        let viewport_pos = {
            let egui_pos = ui_state.viewport_rect.left_top().to_vec2() * scale_factor;
            Vec2::new(egui_pos.x, egui_pos.y)
        };
        let viewport_size = {
            let egui_size = ui_state.viewport_rect.size() * scale_factor;
            Vec2::new(egui_size.x, egui_size.y)
        };
        (viewport_pos, viewport_size)
    } else {
        (Vec2::ZERO, window.physical_size().as_vec2())
    };

    let physical_position = UVec2::new(viewport_pos.x as u32, viewport_pos.y as u32);
    let physical_size = UVec2::new(viewport_size.x as u32, viewport_size.y as u32);

    let rect = physical_position + physical_size;

    let window_size = window.physical_size();
    // wgpu will panic if trying to set a viewport rect which has coordinates extending
    // past the size of the render target, i.e. the physical window in our case.
    // Also prevent rendering when the window is minimized (size becomes very small).
    const MIN_WINDOW_SIZE: u32 = 16;
    if rect.x <= window_size.x
        && rect.y <= window_size.y
        && window_size.x >= MIN_WINDOW_SIZE
        && window_size.y >= MIN_WINDOW_SIZE
        && physical_size.x > 0
        && physical_size.y > 0
    {
        for mut cam in &mut cameras {
            cam.viewport = Some(Viewport {
                physical_position,
                physical_size,
                depth: 0.0..1.0,
            });
        }
    } else {
        // Clear viewport when window is minimized to prevent scissor rect validation errors
        for mut cam in &mut cameras {
            cam.viewport = None;
        }
    }
}

/// Run condition that returns true when the pointer is over egui panels (not the game viewport).
///
/// When the inspector panel is active, this checks if the cursor is inside the game viewport area.
/// If the cursor is inside the viewport, returns false (allow game input).
/// If the cursor is outside the viewport (over egui panels), returns true (block game input).
///
/// Use with `not(...)` to gate systems that should only run when clicking on the game viewport:
/// ```ignore
/// app.add_systems(Update, my_click_system.run_if(not(egui_pointer_over_area)));
/// ```
pub fn egui_pointer_over_area(
    viewport_rect: Res<GameViewportRect>,
    window: Single<&Window, With<PrimaryWindow>>,
    enabled: Res<InspectorEnabled>,
) -> bool {
    // If inspector panel is not enabled, don't block any clicks
    if !enabled.0 {
        return false;
    }

    // Check if cursor is inside the game viewport
    if let Some(cursor_pos) = window.cursor_position() {
        // If cursor is inside game viewport, don't block clicks
        if viewport_rect.contains(cursor_pos.x, cursor_pos.y) {
            return false;
        }
    }

    // Cursor is outside viewport (over egui panels) → block game input
    true
}
