//! Camera viewport management for the inspector.

use bevy::{
    camera::{RenderTarget, Viewport},
    prelude::*,
    window::{PrimaryWindow, Window, WindowRef},
};
use bevy_egui::{EguiContextSettings, PrimaryEguiContext};

use crate::panel::GameWindow;
use crate::state::{GameViewportRect, InspectorEnabled, UiState};
use crate::GameWindowEntity;

/// Marker component for the main game camera.
///
/// Games should add this component to their primary camera for viewport management.
#[derive(Component)]
pub struct InspectorMainCamera;

const MIN_WINDOW_SIZE: u32 = 16;

/// System that adjusts the camera viewport to not overlap with egui panels.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn set_camera_viewport(
    ui_state: Res<UiState>,
    enabled: Res<InspectorEnabled>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut cameras: Query<&mut Camera, (With<InspectorMainCamera>, Without<PrimaryEguiContext>)>,
    mut egui_settings: Single<&mut EguiContextSettings, With<PrimaryEguiContext>>,
) {
    egui_settings.capture_pointer_input = true;

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
/// ```
/// use bevy::prelude::*;
/// use msg_inspector::prelude::*;
///
/// fn my_click_system() {}
///
/// let mut app = App::new();
/// app.add_systems(Update, my_click_system.run_if(not(egui_pointer_over_area)));
/// ```
#[must_use]
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

/// System that routes `InspectorMainCamera` cameras to render into the game window.
///
/// In TwoWindow mode, game cameras render to the separate game window at full resolution
/// with no viewport clipping. `RenderTarget` is a separate component in Bevy 0.18.
#[allow(clippy::needless_pass_by_value, clippy::type_complexity)]
pub fn route_cameras_to_game_window(
    game_window: Res<GameWindowEntity>,
    mut cameras: Query<
        (&mut Camera, &mut RenderTarget),
        (With<InspectorMainCamera>, Without<PrimaryEguiContext>),
    >,
) {
    for (mut cam, mut render_target) in &mut cameras {
        let already_set = matches!(
            &*render_target,
            RenderTarget::Window(WindowRef::Entity(e)) if *e == game_window.0
        );
        if !already_set {
            *render_target = RenderTarget::Window(WindowRef::Entity(game_window.0));
        }
        if cam.viewport.is_some() {
            cam.viewport = None;
        }
    }
}

/// System that positions the game window exactly over the GameView tab area.
///
/// Reads the viewport rect captured by the GameView tab, converts to screen
/// coordinates relative to the inspector window, and moves/resizes the game
/// window to overlay that area.
#[allow(clippy::cast_possible_truncation)]
pub fn sync_game_window_position(
    ui_state: Res<UiState>,
    inspector_window: Single<&Window, With<PrimaryWindow>>,
    mut game_window: Single<&mut Window, With<GameWindow>>,
    enabled: Res<InspectorEnabled>,
) {
    if !enabled.0 {
        if game_window.visible {
            game_window.visible = false;
        }
        return;
    }

    let viewport = ui_state.viewport_rect;
    if viewport == bevy_egui::egui::Rect::NOTHING || viewport.width() <= 0.0 || viewport.height() <= 0.0 {
        return;
    }

    let scale = inspector_window.scale_factor();

    // Get inspector window's screen position (Bevy updates this when the window moves)
    let inspector_pos = match inspector_window.position {
        WindowPosition::At(pos) => pos,
        _ => return, // Position not yet known from OS
    };

    // viewport_rect is in egui logical coordinates relative to the inspector window.
    // Convert to screen coordinates for the game window position.
    let game_x = inspector_pos.x + (viewport.min.x * scale) as i32;
    let game_y = inspector_pos.y + (viewport.min.y * scale) as i32;
    let game_w = viewport.width() * scale;
    let game_h = viewport.height() * scale;

    if game_w < 1.0 || game_h < 1.0 {
        return;
    }

    game_window.position = WindowPosition::At(IVec2::new(game_x, game_y));
    game_window.resolution.set(game_w, game_h);
    if !game_window.visible {
        game_window.visible = true;
    }
}
