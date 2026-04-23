//! Camera viewport management for the inspector.
//!
//! The inspector automatically discovers cameras rendering to the primary
//! window and clips their viewport to the GameView dock area while the
//! inspector panel is visible. Games do not need to tag their cameras or
//! otherwise cooperate with the inspector: the original viewport is captured
//! on a private component and restored when the inspector is toggled off.

use bevy::{
    camera::{RenderTarget, Viewport},
    prelude::*,
    window::{PrimaryWindow, Window, WindowRef},
};
use bevy_egui::{EguiContextSettings, PrimaryEguiContext};

use crate::state::{GameViewportRect, InspectorEnabled, UiState};

const MIN_WINDOW_SIZE: u32 = 16;

/// Tracks a camera whose viewport is currently being managed by the inspector.
///
/// Holds the viewport the camera had before the inspector took over so the
/// original setting can be restored when the panel is toggled off. Exposed
/// only because it appears in the signature of the public
/// [`set_camera_viewport`] system; it is not part of the stable API.
#[doc(hidden)]
#[derive(Component)]
pub struct ManagedByInspector {
    original_viewport: Option<Viewport>,
}

/// Cameras rendering to the primary window are the ones whose visible area
/// overlaps the inspector dock. Cameras targeting images (e.g. minimap or
/// effect textures) render off-screen and are left untouched. A camera with
/// no explicit `RenderTarget` component also defaults to the primary window.
fn targets_primary_window(render_target: Option<&RenderTarget>) -> bool {
    matches!(
        render_target,
        None | Some(RenderTarget::Window(WindowRef::Primary))
    )
}

type ManagedCameraQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut Camera,
        Option<&'static RenderTarget>,
        Option<&'static ManagedByInspector>,
    ),
    Without<PrimaryEguiContext>,
>;

/// System that adjusts the camera viewport to not overlap with egui panels.
///
/// Discovers game cameras at runtime: every camera targeting the primary
/// window (except the egui context's own camera) is clipped to the GameView
/// rect while the inspector is enabled, and restored when it is disabled.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn set_camera_viewport(
    ui_state: Res<UiState>,
    enabled: Res<InspectorEnabled>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut cameras: ManagedCameraQuery,
    mut egui_settings: Single<&mut EguiContextSettings, With<PrimaryEguiContext>>,
    mut commands: Commands,
) {
    egui_settings.capture_pointer_input = true;

    // When the inspector is off, hand every camera back its original viewport
    // and stop tracking it. From this point on we leave `camera.viewport`
    // entirely to the game.
    if !enabled.0 {
        for (entity, mut camera, _target, managed) in &mut cameras {
            let Some(managed) = managed else { continue };
            camera.viewport = managed.original_viewport.clone();
            commands.entity(entity).remove::<ManagedByInspector>();
        }
        return;
    }

    let scale_factor = window.scale_factor() * egui_settings.scale_factor;

    let egui_pos = ui_state.viewport_rect.left_top().to_vec2() * scale_factor;
    let egui_size = ui_state.viewport_rect.size() * scale_factor;
    let physical_position = UVec2::new(egui_pos.x as u32, egui_pos.y as u32);
    let physical_size = UVec2::new(egui_size.x as u32, egui_size.y as u32);
    let rect_end = physical_position + physical_size;

    let window_size = window.physical_size();
    // wgpu will panic if trying to set a viewport rect which has coordinates
    // extending past the size of the render target, i.e. the physical window
    // in our case. Also skip when the window is minimized (size very small).
    let clip = if rect_end.x <= window_size.x
        && rect_end.y <= window_size.y
        && window_size.x >= MIN_WINDOW_SIZE
        && window_size.y >= MIN_WINDOW_SIZE
        && physical_size.x > 0
        && physical_size.y > 0
    {
        Some(Viewport {
            physical_position,
            physical_size,
            depth: 0.0..1.0,
        })
    } else {
        None
    };

    for (entity, mut camera, target, managed) in &mut cameras {
        if !targets_primary_window(target) {
            continue;
        }

        if managed.is_none() {
            commands.entity(entity).insert(ManagedByInspector {
                original_viewport: camera.viewport.clone(),
            });
        }

        camera.viewport = clip.clone();
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
