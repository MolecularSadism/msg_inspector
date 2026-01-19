//! Entity picking systems for selecting entities in the viewport.

use bevy::{gizmos::gizmos::Gizmos, prelude::*, window::Window};
use bevy_inspector_egui::bevy_egui::{EguiContext, PrimaryEguiContext};

use crate::state::{InspectorEnabled, InspectorSelection, UiState};

/// Marker component for the crosshair visual that shows the picked entity's position.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct PickedEntityMarker;

/// The size of the crosshair marker lines.
const CROSSHAIR_SIZE: f32 = 20.0;

/// Default crosshair color (green).
const CROSSHAIR_COLOR: Color = Color::srgb(0.2, 0.8, 0.2);

/// Automatically adds `Pickable` component to newly spawned sprites.
pub fn auto_add_pickable_to_sprites(
    mut commands: Commands,
    query: Query<Entity, (Added<Sprite>, Without<Pickable>)>,
) {
    for entity in &query {
        commands.entity(entity).insert(Pickable::default());
    }
}

/// Handles mouse clicks on entities to select them for inspection.
///
/// Note: This system requires `MouseCoords` resource to be available from the game.
/// Games should provide a system that populates world coordinates from mouse position.
/// This is a simplified version that uses direct window cursor position.
pub fn handle_picking_clicks(
    mut ui_state: ResMut<UiState>,
    enabled: Res<InspectorEnabled>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut q_egui_ctx: Query<&mut EguiContext, With<PrimaryEguiContext>>,
    q_sprites: Query<(Entity, &GlobalTransform, &Sprite), With<Pickable>>,
    images: Res<Assets<Image>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
) {
    if !enabled.0 {
        return;
    }

    // Only handle left clicks
    if !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }

    // Check if egui wants the pointer (clicking on UI panels)
    if let Ok(mut egui_ctx) = q_egui_ctx.single_mut()
        && egui_ctx.get_mut().wants_pointer_input()
    {
        return;
    }

    // Get cursor position in window
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    // Find a camera to convert to world coordinates
    // Try to find a camera that's not the egui camera
    let Some((camera, camera_transform)) = camera_query
        .iter()
        .find(|(cam, _)| cam.order >= 0)
    else {
        return;
    };

    // Convert cursor position to world coordinates
    let Ok(mouse_world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };

    // Find the sprite with highest z-order that contains the mouse position
    let mut best_hit: Option<(Entity, f32)> = None;

    for (entity, global_transform, sprite) in &q_sprites {
        let sprite_pos = global_transform.translation().truncate();
        let sprite_z = global_transform.translation().z;

        let sprite_size = sprite.custom_size.unwrap_or_else(|| {
            images
                .get(&sprite.image)
                .map(|img| img.size().as_vec2())
                .unwrap_or(Vec2::splat(32.0))
        });

        let half_size = sprite_size / 2.0;
        let min = sprite_pos - half_size;
        let max = sprite_pos + half_size;

        if mouse_world_pos.x >= min.x
            && mouse_world_pos.x <= max.x
            && mouse_world_pos.y >= min.y
            && mouse_world_pos.y <= max.y
        {
            // Select entity with highest z (closest to camera)
            if best_hit.is_none_or(|(_, best_z)| sprite_z > best_z) {
                best_hit = Some((entity, sprite_z));
            }
        }
    }

    if let Some((entity, _)) = best_hit {
        let add = keyboard.pressed(KeyCode::ControlLeft)
            || keyboard.pressed(KeyCode::ControlRight)
            || keyboard.pressed(KeyCode::ShiftLeft)
            || keyboard.pressed(KeyCode::ShiftRight);

        ui_state.selected_entities.select_maybe_add(entity, add);
        ui_state.selection = InspectorSelection::Entities;
    }
}

/// Updates the visual crosshair marker to show the position of selected entities.
pub fn update_picked_entity_marker(
    mut commands: Commands,
    ui_state: Res<UiState>,
    enabled: Res<InspectorEnabled>,
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
    let color = CROSSHAIR_COLOR;

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
