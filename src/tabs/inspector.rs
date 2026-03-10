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
use sublime_fuzzy::best_match;

use crate::state::InspectorSelection;

/// Render the inspector tab.
pub fn render(
    ui: &mut egui::Ui,
    world: &mut World,
    type_registry: &TypeRegistry,
    selected_entities: &SelectedEntities,
    selection: &InspectorSelection,
    inspector_search: &mut String,
) {
    // Search input
    ui.horizontal(|ui| {
        ui.label("Filter:");
        ui.text_edit_singleline(inspector_search);
        if ui.small_button("X").clicked() {
            inspector_search.clear();
        }
    });
    ui.separator();

    let search_query = inspector_search.trim().to_string();

    match selection {
        InspectorSelection::Entities => {
            if !search_query.is_empty() {
                render_matching_components_list(ui, world, selected_entities, &search_query);
                ui.separator();
            }
            match selected_entities.as_slice() {
                &[entity] => ui_for_entity_with_children(world, entity, ui),
                entities => ui_for_entities_shared_components(world, entities, ui),
            }
        }
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

/// Show a list of component names that match the fuzzy search query.
fn render_matching_components_list(
    ui: &mut egui::Ui,
    world: &World,
    selected_entities: &SelectedEntities,
    search_query: &str,
) {
    let entities = selected_entities.as_slice();
    if entities.is_empty() {
        return;
    }

    let Some(type_registry_res) = world.get_resource::<AppTypeRegistry>() else {
        return;
    };
    let type_registry = type_registry_res.0.read();

    for &entity in entities {
        let Some(entity_ref) = world.get_entity(entity).ok() else {
            continue;
        };

        let mut matching: Vec<(&str, isize)> = entity_ref
            .archetype()
            .components()
            .iter()
            .filter_map(|component_id| {
                let info = world.components().get_info(*component_id)?;
                let type_id = info.type_id()?;
                let registration = type_registry.get(type_id)?;
                let short_name = registration.type_info().type_path_table().short_path();
                best_match(search_query, short_name).map(|m| (short_name, m.score()))
            })
            .collect();

        matching.sort_by(|(_, a): &(&str, isize), (_, b): &(&str, isize)| b.cmp(a));

        if matching.is_empty() {
            ui.label("No matching components");
        } else {
            ui.label(format!("{} matching components:", matching.len()));
            for (name, _) in &matching {
                ui.small(egui::RichText::new(*name).strong());
            }
        }
    }
}
