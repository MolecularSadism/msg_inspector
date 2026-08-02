//! Entity picking systems for selecting entities in the viewport.
//!
//! Alt+Left click inside the Game tab selects the topmost sprite under the
//! cursor for the Inspector tab (Shift/Ctrl extends the selection). The pick
//! is routed through the [`InspectorMainCamera`], whose viewport
//! [`set_camera_viewport`](crate::set_camera_viewport) shrinks to the Game
//! tab's rect, so picks land on the world position the game view shows rather
//! than where the cursor sits on the full window.

use bevy::{
    ecs::system::SystemParam,
    gizmos::gizmos::Gizmos,
    prelude::*,
    sprite::Anchor,
    window::{PrimaryWindow, Window},
};
use bevy_egui::PrimaryEguiContext;

use crate::{
    state::{GameViewportRect, InspectorEnabled, InspectorSelection, UiState},
    viewport::InspectorMainCamera,
};

/// Marker component for the crosshair visual that shows the picked entity's position.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct PickedEntityMarker;

/// The size of the crosshair marker lines.
const CROSSHAIR_SIZE: f32 = 20.0;

/// Default crosshair color (green).
const DEFAULT_CROSSHAIR_COLOR: Color = Color::srgb(0.2, 0.8, 0.2);

/// Configuration for the entity selection crosshair visual.
///
/// # Example
///
/// ```
/// use bevy::prelude::*;
/// use msg_inspector::prelude::*;
/// let mut app = App::new();
/// // Change crosshair color to red
/// app.insert_resource(CrosshairConfig {
///     color: Color::srgb(1.0, 0.2, 0.2),
/// });
/// ```
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource)]
pub struct CrosshairConfig {
    /// Color of the crosshair gizmo.
    pub color: Color,
}

impl Default for CrosshairConfig {
    fn default() -> Self {
        Self {
            color: DEFAULT_CROSSHAIR_COLOR,
        }
    }
}

/// Marker component that excludes an entity from inspector entity picking.
///
/// Insert it on sprites that cover the game view without being meaningful pick
/// targets — a camera-following, screen-sized canvas sprite (as used by
/// pixel-perfect render pipelines) would otherwise swallow every pick.
///
/// # Example
///
/// ```
/// use bevy::prelude::*;
/// use msg_inspector::prelude::*;
///
/// fn spawn_canvas(mut commands: Commands) {
///     commands.spawn((Sprite::default(), PickingIgnore));
/// }
///
/// let mut app = App::new();
/// app.add_systems(Startup, spawn_canvas);
/// ```
#[derive(Component, Reflect, Clone, Copy, Debug, Default)]
#[reflect(Component)]
pub struct PickingIgnore;

/// The camera picks are projected through: the inspector's main camera,
/// never the egui context camera.
type PickingCamera<'w, 's> = Single<
    'w,
    's,
    (&'static Camera, &'static GlobalTransform),
    (With<InspectorMainCamera>, Without<PrimaryEguiContext>),
>;

/// Sprites eligible for picking — everything not marked [`PickingIgnore`].
type PickableSprites<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static GlobalTransform,
        &'static ViewVisibility,
        &'static Sprite,
        &'static Anchor,
    ),
    Without<PickingIgnore>,
>;

/// Input state and world data a pick is resolved against.
#[derive(SystemParam)]
pub struct PickingScene<'w, 's> {
    keys: Res<'w, ButtonInput<KeyCode>>,
    mouse_buttons: Res<'w, ButtonInput<MouseButton>>,
    window: Single<'w, 's, &'static Window, With<PrimaryWindow>>,
    camera: PickingCamera<'w, 's>,
    sprites: PickableSprites<'w, 's>,
    images: Res<'w, Assets<Image>>,
    layouts: Res<'w, Assets<TextureAtlasLayout>>,
}

/// Selects the sprite under the cursor on Alt+Left click in the game view.
///
/// The click is only handled while the inspector is enabled and the cursor is
/// inside [`GameViewportRect`] — the Game tab's rect, which holds regardless of
/// whether egui claims the pointer elsewhere in the dock. Holding Shift or
/// Ctrl extends the current selection instead of replacing it.
///
/// Candidates are visible sprites without [`PickingIgnore`]. Among hits, the
/// highest world z wins (closest to the 2D camera).
pub fn handle_picking_clicks(
    mut ui_state: ResMut<UiState>,
    enabled: Res<InspectorEnabled>,
    viewport_rect: Res<GameViewportRect>,
    scene: PickingScene,
) {
    let keys = &scene.keys;
    let alt_held = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
    if !enabled.0 || !alt_held || !scene.mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = scene.window.cursor_position() else {
        return;
    };
    if !viewport_rect.contains(cursor.x, cursor.y) {
        return;
    }

    // `viewport_to_world_2d` subtracts the camera's logical viewport offset
    // internally, so the raw window cursor position is already correct even
    // when the inspector has shrunk the camera to the Game tab.
    let (camera, camera_transform) = *scene.camera;
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor) else {
        return;
    };

    let mut best: Option<(Entity, f32)> = None;
    for (entity, global, visibility, sprite, anchor) in &scene.sprites {
        if !visibility.get() {
            continue;
        }
        if !sprite_contains_world_point(
            sprite,
            *anchor,
            global,
            world_pos,
            &scene.images,
            &scene.layouts,
        ) {
            continue;
        }
        let z = global.translation().z;
        if best.is_none_or(|(_, best_z)| z > best_z) {
            best = Some((entity, z));
        }
    }

    let Some((entity, _)) = best else {
        return;
    };

    let extend = keys.pressed(KeyCode::ShiftLeft)
        || keys.pressed(KeyCode::ShiftRight)
        || keys.pressed(KeyCode::ControlLeft)
        || keys.pressed(KeyCode::ControlRight);
    ui_state.selected_entities.select_maybe_add(entity, extend);
    ui_state.selection = InspectorSelection::Entities;
}

/// Whether `world_pos` lies inside the sprite's rendered rectangle.
///
/// The point is transformed into the sprite's local frame (covering
/// translation, rotation, and scale), then tested against the sprite's bounds
/// via [`Sprite::compute_pixel_space_point`], which resolves custom sizes,
/// sprite rects, and texture-atlas frames. A degenerate transform (zero scale)
/// produces a non-finite local point, which that test rejects.
fn sprite_contains_world_point(
    sprite: &Sprite,
    anchor: Anchor,
    global: &GlobalTransform,
    world_pos: Vec2,
    images: &Assets<Image>,
    layouts: &Assets<TextureAtlasLayout>,
) -> bool {
    let local = global
        .affine()
        .inverse()
        .transform_point3(world_pos.extend(global.translation().z));
    sprite
        .compute_pixel_space_point(local.truncate(), anchor, images, layouts)
        .is_ok()
}

/// Updates the visual crosshair marker to show the position of selected entities.
pub fn update_picked_entity_marker(
    mut commands: Commands,
    ui_state: Res<UiState>,
    enabled: Res<InspectorEnabled>,
    crosshair_config: Res<CrosshairConfig>,
    q_marker: Query<Entity, With<PickedEntityMarker>>,
    q_transforms: Query<&GlobalTransform>,
    mut gizmos: Gizmos,
) {
    // Despawn existing markers if dev panel is disabled
    if !enabled.0 {
        for entity in &q_marker {
            commands.entity(entity).despawn();
        }
        return;
    }

    // Draw crosshair gizmos for each selected entity
    let color = crosshair_config.color;

    for entity in ui_state.selected_entities.iter() {
        if let Ok(transform) = q_transforms.get(entity) {
            let pos = transform.translation().truncate();

            // Draw crosshair
            // Horizontal line
            gizmos.line_2d(
                Vec2::new(pos.x - CROSSHAIR_SIZE, pos.y),
                Vec2::new(pos.x + CROSSHAIR_SIZE, pos.y),
                color,
            );
            // Vertical line
            gizmos.line_2d(
                Vec2::new(pos.x, pos.y - CROSSHAIR_SIZE),
                Vec2::new(pos.x, pos.y + CROSSHAIR_SIZE),
                color,
            );
            // Circle outline
            gizmos.circle_2d(
                Isometry2d::from_translation(pos),
                CROSSHAIR_SIZE * 0.7,
                color,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assets() -> (Assets<Image>, Assets<TextureAtlasLayout>) {
        (Assets::default(), Assets::default())
    }

    fn sprite_sized(size: Vec2) -> Sprite {
        Sprite {
            custom_size: Some(size),
            ..Default::default()
        }
    }

    fn hit(sprite: &Sprite, anchor: Anchor, transform: Transform, world_pos: Vec2) -> bool {
        let (images, layouts) = assets();
        sprite_contains_world_point(
            sprite,
            anchor,
            &GlobalTransform::from(transform),
            world_pos,
            &images,
            &layouts,
        )
    }

    #[test]
    fn hit_inside_and_outside_axis_aligned_sprite() {
        let sprite = sprite_sized(Vec2::new(10.0, 4.0));
        let transform = Transform::from_translation(Vec3::new(100.0, 50.0, 3.0));

        assert!(hit(
            &sprite,
            Anchor::CENTER,
            transform,
            Vec2::new(100.0, 50.0)
        ));
        assert!(hit(
            &sprite,
            Anchor::CENTER,
            transform,
            Vec2::new(104.9, 51.9)
        ));
        assert!(!hit(
            &sprite,
            Anchor::CENTER,
            transform,
            Vec2::new(105.5, 50.0)
        ));
        assert!(!hit(
            &sprite,
            Anchor::CENTER,
            transform,
            Vec2::new(100.0, 52.5)
        ));
    }

    #[test]
    fn hit_respects_transform_scale() {
        let sprite = sprite_sized(Vec2::splat(10.0));
        let transform = Transform::from_scale(Vec3::new(2.0, 2.0, 1.0));

        // Scaled world half-extent is 10.0 on both axes.
        assert!(hit(
            &sprite,
            Anchor::CENTER,
            transform,
            Vec2::new(9.0, -9.0)
        ));
        assert!(!hit(
            &sprite,
            Anchor::CENTER,
            transform,
            Vec2::new(11.0, 0.0)
        ));
    }

    #[test]
    fn hit_respects_rotation() {
        let sprite = sprite_sized(Vec2::new(10.0, 2.0));
        let transform =
            Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));

        // A quarter turn puts the long axis vertical in world space.
        assert!(hit(&sprite, Anchor::CENTER, transform, Vec2::new(0.0, 4.0)));
        assert!(!hit(
            &sprite,
            Anchor::CENTER,
            transform,
            Vec2::new(4.0, 0.0)
        ));
    }

    #[test]
    fn hit_respects_anchor() {
        let sprite = sprite_sized(Vec2::splat(8.0));
        let transform = Transform::from_translation(Vec3::new(10.0, 10.0, 0.0));

        // Bottom-left anchor: the sprite spans upward and rightward from the
        // entity translation.
        assert!(hit(
            &sprite,
            Anchor::BOTTOM_LEFT,
            transform,
            Vec2::new(11.0, 11.0)
        ));
        assert!(!hit(
            &sprite,
            Anchor::BOTTOM_LEFT,
            transform,
            Vec2::new(9.0, 9.0)
        ));
    }

    #[test]
    fn hit_uses_atlas_frame_size() {
        let (images, mut layouts) = assets();
        let layout = TextureAtlasLayout::from_grid(UVec2::splat(16), 2, 2, None, None);
        let layout_handle = layouts.add(layout);
        let sprite = Sprite {
            texture_atlas: Some(TextureAtlas {
                layout: layout_handle,
                index: 0,
            }),
            ..Default::default()
        };
        let global = GlobalTransform::IDENTITY;

        // The 16x16 frame, not the 32x32 sheet, bounds the hit.
        assert!(sprite_contains_world_point(
            &sprite,
            Anchor::CENTER,
            &global,
            Vec2::new(7.0, 7.0),
            &images,
            &layouts,
        ));
        assert!(!sprite_contains_world_point(
            &sprite,
            Anchor::CENTER,
            &global,
            Vec2::new(9.0, 0.0),
            &images,
            &layouts,
        ));
    }

    #[test]
    fn degenerate_scale_never_hits() {
        let sprite = sprite_sized(Vec2::splat(10.0));
        let transform = Transform::from_scale(Vec3::ZERO);

        assert!(!hit(&sprite, Anchor::CENTER, transform, Vec2::ZERO));
    }
}
