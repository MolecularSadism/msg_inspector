//! Core panel management for the inspector UI.

use bevy::{camera::visibility::RenderLayers, prelude::*, window::PrimaryWindow};
use bevy_egui::EguiGlobalSettings;
use bevy_egui::{EguiContext, PrimaryEguiContext};

use crate::state::{GameViewportRect, InspectorEnabled, UiState};
use crate::{GameWindowEntity, InspectorMode, InspectorModeRes};

/// Marker component for the game window (distinct from the primary/inspector window).
#[derive(Component)]
pub struct GameWindow;

/// Startup system that spawns a separate borderless game window (TwoWindow mode).
pub fn spawn_game_window(mut commands: Commands) {
    let id = commands
        .spawn((
            Window {
                title: "Game".into(),
                decorations: false,
                visible: false, // hidden until sync_game_window_position places it
                position: WindowPosition::Centered(MonitorSelection::Primary),
                ..default()
            },
            GameWindow,
            Name::new("Game Window"),
        ))
        .id();
    commands.insert_resource(GameWindowEntity(id));
}

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

        // Export viewport rect for input handling and game window positioning
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
pub fn toggle_inspector(
    keys: Res<ButtonInput<KeyCode>>,
    mut enabled: ResMut<InspectorEnabled>,
    mode: Res<InspectorModeRes>,
    mut primary_window: Option<Single<&mut Window, With<PrimaryWindow>>>,
) {
    if keys.just_pressed(KeyCode::Delete) {
        enabled.0 = !enabled.0;

        // In TwoWindow mode, hide/show the inspector window itself
        if mode.0 == InspectorMode::TwoWindow
            && let Some(ref mut window) = primary_window
        {
            window.visible = enabled.0;
        }
    }
}

/// Startup system to configure egui and spawn required entities.
pub fn setup(mut commands: Commands, mut egui_global_settings: ResMut<EguiGlobalSettings>) {
    egui_global_settings.auto_create_primary_context = false;

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
