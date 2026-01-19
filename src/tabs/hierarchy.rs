//! Entity hierarchy browser tab.
//!
//! Provides a searchable tree view of all entities in the world.

use bevy::prelude::*;
use bevy_egui::egui;
use bevy_inspector_egui::bevy_inspector::hierarchy::{SelectedEntities, hierarchy_ui};

use crate::state::InspectorSelection;

/// Render the hierarchy tab.
pub fn render(
    ui: &mut egui::Ui,
    world: &mut World,
    selected_entities: &mut SelectedEntities,
    selection: &mut InspectorSelection,
    hierarchy_search: &mut String,
) {
    // Search input
    ui.horizontal(|ui| {
        ui.label("Search:");
        ui.text_edit_singleline(hierarchy_search);
        if ui.small_button("X").clicked() {
            hierarchy_search.clear();
        }
    });
    ui.separator();

    let search_query = hierarchy_search.trim().to_lowercase();

    if search_query.is_empty() {
        // No search - use default hierarchy UI
        let selected = hierarchy_ui(world, ui, selected_entities);
        if selected {
            *selection = InspectorSelection::Entities;
        }
    } else {
        // Filtered entity list based on search
        render_filtered_hierarchy(ui, world, selected_entities, selection, &search_query);
    }
}

/// Render a filtered list of entities matching the search query.
fn render_filtered_hierarchy(
    ui: &mut egui::Ui,
    world: &mut World,
    selected_entities: &mut SelectedEntities,
    selection: &mut InspectorSelection,
    search_query: &str,
) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        let mut matching_entities: Vec<(Entity, String)> = Vec::new();

        // Search by Name component
        let mut q_named = world.query::<(Entity, &Name)>();
        for (entity, name) in q_named.iter(world) {
            if name.as_str().to_lowercase().contains(search_query) {
                matching_entities.push((entity, name.to_string()));
            }
        }

        // Also search by Entity ID (e.g., "123v4")
        for entity in world.iter_entities() {
            let entity_id = entity.id();
            let id_str = format!("{}v{}", entity_id.index(), entity_id.generation());
            if id_str.contains(search_query) {
                let name = world
                    .get::<Name>(entity_id)
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| id_str.clone());
                if !matching_entities.iter().any(|(e, _)| *e == entity_id) {
                    matching_entities.push((entity_id, name));
                }
            }
        }

        // Sort by name for consistent ordering
        matching_entities.sort_by(|(_, a), (_, b)| a.cmp(b));

        // Display results
        ui.label(format!("{} results", matching_entities.len()));
        ui.add_space(4.0);

        for (entity, display_name) in matching_entities {
            let is_selected = selected_entities.contains(entity);
            let label = format!("{} ({:?})", display_name, entity);

            if ui.selectable_label(is_selected, label).clicked() {
                let modifiers = ui.input(|i| i.modifiers);
                let add_to_selection = modifiers.ctrl || modifiers.shift;
                selected_entities.select_maybe_add(entity, add_to_selection);
                *selection = InspectorSelection::Entities;
            }
        }
    });
}
