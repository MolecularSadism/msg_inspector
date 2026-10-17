//! Core panel management for the inspector UI.

use bevy::{camera::visibility::RenderLayers, prelude::*};
use bevy_egui::EguiGlobalSettings;
use bevy_egui::{EguiContext, PrimaryEguiContext};

use crate::state::{InspectorEnabled, InspectorKeys, UiState};

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
    });
    crate::viewport::export_game_viewport_rect(world);
}

/// System to toggle the inspector panel visibility on [`InspectorKeys::toggle`].
pub fn toggle_inspector(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<InspectorKeys>,
    mut enabled: ResMut<InspectorEnabled>,
) {
    if keys.any_just_pressed(bindings.toggle.iter().copied()) {
        enabled.0 = !enabled.0;
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

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    fn toggled_by(bindings: InspectorKeys, key: KeyCode) -> bool {
        let mut world = World::new();
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(key);
        world.insert_resource(keys);
        world.insert_resource(bindings);
        world.insert_resource(InspectorEnabled(false));
        world
            .run_system_once(toggle_inspector)
            .expect("toggle_inspector runs");
        world.resource::<InspectorEnabled>().0
    }

    #[test]
    fn toggle_follows_the_configured_keys() {
        let bindings = InspectorKeys {
            toggle: vec![KeyCode::F1],
            ..default()
        };
        assert!(toggled_by(bindings.clone(), KeyCode::F1));
        let replaced = InspectorKeys::default().toggle[0];
        assert!(!toggled_by(bindings, replaced));
    }

    #[test]
    fn empty_toggle_list_disables_the_toggle() {
        let bindings = InspectorKeys {
            toggle: Vec::new(),
            ..default()
        };
        let default_key = InspectorKeys::default().toggle[0];
        assert!(!toggled_by(bindings, default_key));
    }
}
