//! Core panel management for the inspector UI.

use bevy::{pbr::AmbientLight, pbr::DirectionalLight, prelude::*, render::view::RenderLayers};
use bevy_egui::EguiGlobalSettings;
use bevy_inspector_egui::bevy_egui::{EguiContext, PrimaryEguiContext};

use crate::state::{GameViewportRect, InspectorEnabled, UiState};

/// System that renders the inspector UI.
pub fn show_ui_system(world: &mut World) {
    let Some(enabled) = world.get_resource::<InspectorEnabled>() else {
        return;
    };
    if !enabled.0 {
        return;
    }

    let Ok(egui_context) = world
        .query_filtered::<&mut EguiContext, With<PrimaryEguiContext>>()
        .single(world)
    else {
        return;
    };
    let mut egui_context = egui_context.clone();

    world.resource_scope::<UiState, _>(|world, mut ui_state| {
        ui_state.ui(world, egui_context.get_mut());

        // Export viewport rect for input handling
        if let Some(mut viewport_res) = world.get_resource_mut::<GameViewportRect>() {
            let rect = ui_state.viewport_rect;
            viewport_res.min_x = rect.min.x;
            viewport_res.min_y = rect.min.y;
            viewport_res.max_x = rect.max.x;
            viewport_res.max_y = rect.max.y;
        }
    });
}

/// System to toggle the inspector panel visibility.
pub fn toggle_inspector(keys: Res<ButtonInput<KeyCode>>, mut enabled: ResMut<InspectorEnabled>) {
    if keys.just_pressed(KeyCode::Delete) {
        enabled.0 = !enabled.0;
    }
}

/// Startup system to configure egui and spawn required entities.
pub fn setup(
    mut commands: Commands,
    mut egui_global_settings: ResMut<EguiGlobalSettings>,
) {
    egui_global_settings.auto_create_primary_context = false;

    // Ambient light for 3D scenes (optional, games can override)
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.02,
        ..default()
    });

    // Directional light
    commands.spawn((
        DirectionalLight {
            illuminance: 2000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::PI / 2.0)),
    ));

    // Egui camera (separate from game camera)
    commands.spawn((
        Camera2d,
        Name::new("Egui Camera"),
        PrimaryEguiContext,
        RenderLayers::none(),
        Msaa::Off,
        Camera {
            order: -1,
            ..default()
        },
    ));
}
