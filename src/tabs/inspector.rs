//! Entity/resource/asset inspector tab.
//!
//! Displays detailed information about the currently selected item.

use bevy::prelude::*;
use bevy::reflect::TypeRegistry;
use bevy_egui::egui;
use bevy_inspector_egui::bevy_inspector::{
    self, hierarchy::SelectedEntities, ui_for_entities_shared_components,
    ui_for_entity_with_children,
};

use crate::state::InspectorSelection;

/// Render the inspector tab.
pub fn render(
    ui: &mut egui::Ui,
    world: &mut World,
    type_registry: &TypeRegistry,
    selected_entities: &SelectedEntities,
    selection: &InspectorSelection,
) {
    match selection {
        InspectorSelection::Entities => match selected_entities.as_slice() {
            &[entity] => ui_for_entity_with_children(world, entity, ui),
            entities => ui_for_entities_shared_components(world, entities, ui),
        },
        InspectorSelection::Resource(type_id, name) => {
            ui.label(name);
            bevy_inspector::by_type_id::ui_for_resource(world, *type_id, ui, name, type_registry);
        }
        InspectorSelection::Asset(type_id, name, handle) => {
            ui.label(name);
            bevy_inspector::by_type_id::ui_for_asset(world, *type_id, *handle, ui, type_registry);
        }
    }
}
