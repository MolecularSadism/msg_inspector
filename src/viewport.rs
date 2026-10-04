//! The Game tab's view of the game.
//!
//! While the inspector is open, every camera that targets the primary window
//! renders into [`GameViewImage`] instead — an image with the window's own
//! physical size and scale factor. The game therefore sees a full-size render
//! target and never needs to know about the dock: layout, canvases,
//! post-processing and screen-space effects work as they do without the
//! inspector. The Game tab shows the image letterboxed into whatever room the
//! dock leaves, and picking pointers over it are mapped back into the image so
//! UI hover and clicks land where they are drawn.

use bevy::{
    camera::{CameraUpdateSystems, ImageRenderTarget, NormalizedRenderTarget, RenderTarget},
    picking::{PickingSystems, pointer::PointerLocation},
    prelude::*,
    render::render_resource::{Extent3d, TextureFormat},
    window::{PrimaryWindow, Window, WindowRef},
};
use bevy_egui::{EguiContextSettings, EguiTextureHandle, EguiUserTextures, PrimaryEguiContext};

use crate::state::{GameViewportRect, InspectorEnabled, UiState};

/// Marker component for the main game camera.
///
/// Entity picking projects the cursor into the world through this camera.
#[derive(Component)]
pub struct InspectorMainCamera;

/// The image the game's window cameras render into while the inspector is open.
#[derive(Resource, Debug, Clone)]
pub struct GameViewImage {
    /// The render target, sized to the primary window's physical resolution.
    pub handle: Handle<Image>,
    /// The egui texture the Game tab draws.
    pub texture_id: bevy_egui::egui::TextureId,
}

impl GameViewImage {
    /// The image as a render target with the given scale factor.
    fn target(&self, scale_factor: f32) -> ImageRenderTarget {
        ImageRenderTarget {
            handle: self.handle.clone(),
            scale_factor,
        }
    }
}

/// The window target a camera had before it was moved onto [`GameViewImage`].
#[derive(Component, Debug, Clone)]
#[component(storage = "SparseSet")]
struct RetargetedToGameView(RenderTarget);

/// Game cameras with their render target — every camera except the egui context's.
type GameCameras<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut RenderTarget,
        Option<&'static RetargetedToGameView>,
    ),
    (With<Camera>, Without<PrimaryEguiContext>),
>;

pub(crate) fn plugin(app: &mut App) {
    app.add_systems(Startup, create_game_view_image)
        .add_systems(
            PostUpdate,
            (resize_game_view_image, retarget_window_cameras)
                .chain()
                .before(CameraUpdateSystems),
        )
        .add_systems(
            PreUpdate,
            retarget_window_pointers
                .after(PickingSystems::ProcessInput)
                .before(PickingSystems::Backend),
        );
}

fn create_game_view_image(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut user_textures: ResMut<EguiUserTextures>,
) {
    let handle = images.add(Image::new_target_texture(
        1,
        1,
        TextureFormat::Rgba8UnormSrgb,
        None,
    ));
    let texture_id = user_textures.add_image(EguiTextureHandle::Strong(handle.clone()));
    commands.insert_resource(GameViewImage { handle, texture_id });
}

/// Keeps [`GameViewImage`] at the window's physical size while the inspector is open.
fn resize_game_view_image(
    enabled: Res<InspectorEnabled>,
    view: Res<GameViewImage>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
) {
    if !enabled.0 {
        return;
    }
    let size = window.physical_size().max(UVec2::ONE);
    let current = images
        .get(&view.handle)
        .map(|image| image.size())
        .unwrap_or_default();
    if current != size
        && let Some(image) = images.get_mut(&view.handle)
    {
        image.resize(Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        });
    }
}

/// Moves every camera that renders to the primary window onto [`GameViewImage`]
/// while the inspector is open, and back when it closes.
///
/// Cameras spawned later are picked up on their first frame. The egui context
/// camera stays on the window: it draws the dock and the image.
fn retarget_window_cameras(
    mut commands: Commands,
    enabled: Res<InspectorEnabled>,
    view: Res<GameViewImage>,
    window: Single<(Entity, &Window), With<PrimaryWindow>>,
    mut cameras: GameCameras,
) {
    let (primary, window) = *window;
    let image_target = view.target(window.scale_factor());
    for (entity, mut target, retargeted) in &mut cameras {
        match (enabled.0, retargeted) {
            (true, None) => {
                let on_primary = match &*target {
                    RenderTarget::Window(WindowRef::Primary) => true,
                    RenderTarget::Window(WindowRef::Entity(window)) => *window == primary,
                    _ => false,
                };
                if on_primary {
                    commands
                        .entity(entity)
                        .insert(RetargetedToGameView(target.clone()));
                    *target = RenderTarget::Image(image_target.clone());
                }
            }
            // Follows scale-factor changes without touching unchanged targets.
            (true, Some(_)) => {
                if !matches!(&*target, RenderTarget::Image(current) if *current == image_target) {
                    *target = RenderTarget::Image(image_target.clone());
                }
            }
            (false, Some(RetargetedToGameView(original))) => {
                *target = original.clone();
                commands.entity(entity).remove::<RetargetedToGameView>();
            }
            (false, None) => {}
        }
    }
}

/// Maps pointers on the primary window into [`GameViewImage`] while the inspector
/// is open.
///
/// Picking backends only hit cameras whose render target is the pointer's, and
/// the game cameras now render to the image. A pointer over the shown image is
/// moved to the matching image position; a pointer over the dock has no
/// location in the game.
fn retarget_window_pointers(
    enabled: Res<InspectorEnabled>,
    view: Res<GameViewImage>,
    rect: Res<GameViewportRect>,
    window: Single<(Entity, &Window), With<PrimaryWindow>>,
    mut pointers: Query<&mut PointerLocation>,
) {
    if !enabled.0 {
        return;
    }
    let (primary, window) = *window;
    for mut pointer in &mut pointers {
        let Some(location) = pointer.location.as_ref() else {
            continue;
        };
        let on_primary = matches!(
            &location.target,
            NormalizedRenderTarget::Window(window) if window.entity() == primary
        );
        if !on_primary {
            continue;
        }
        let mapped = rect.to_game(location.position);
        pointer.location = mapped.map(|position| bevy::picking::pointer::Location {
            target: NormalizedRenderTarget::Image(view.target(window.scale_factor())),
            position,
        });
    }
}

/// Publishes where the Game tab shows the image, in window logical pixels.
///
/// The image keeps the window's logical size, so the game-pixel scale is the
/// window width over the shown width.
pub(crate) fn export_game_viewport_rect(world: &mut World) {
    let egui_scale = world
        .query_filtered::<&EguiContextSettings, With<PrimaryEguiContext>>()
        .single(world)
        .map_or(1.0, |settings| settings.scale_factor);
    let window_width = world
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .single(world)
        .map_or(0.0, Window::width);
    let shown = world.resource::<UiState>().viewport_rect;
    let mut rect = world.resource_mut::<GameViewportRect>();
    rect.min_x = shown.min.x * egui_scale;
    rect.min_y = shown.min.y * egui_scale;
    rect.max_x = shown.max.x * egui_scale;
    rect.max_y = shown.max.y * egui_scale;
    let shown_width = shown.width() * egui_scale;
    rect.game_pixels_per_window_pixel = if shown_width > 0.0 {
        window_width / shown_width
    } else {
        1.0
    };
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
