//! Entity hierarchy browser tab.
//!
//! Provides a searchable tree view of all entities in the world.

use bevy::ecs::world::EntityRef;
use bevy::prelude::*;
use bevy_egui::egui;
use bevy_inspector_egui::bevy_inspector::hierarchy::{SelectedEntities, hierarchy_ui};
use sublime_fuzzy::best_match;

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

    let search_query = hierarchy_search.trim();

    if search_query.is_empty() {
        // No search - use default hierarchy UI
        let selected = hierarchy_ui(world, ui, selected_entities);
        if selected {
            *selection = InspectorSelection::Entities;
        }
    } else {
        // Filtered entity list based on fuzzy search
        render_filtered_hierarchy(ui, world, selected_entities, selection, search_query);
    }
}

/// Render a filtered list of entities matching the search query using fuzzy matching.
fn render_filtered_hierarchy(
    ui: &mut egui::Ui,
    world: &mut World,
    selected_entities: &mut SelectedEntities,
    selection: &mut InspectorSelection,
    search_query: &str,
) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        let mut matching_entities: Vec<(Entity, String, isize)> = Vec::new();

        // Fuzzy search by Name component
        let mut q_named = world.query::<(Entity, &Name)>();
        for (entity, name) in q_named.iter(world) {
            if let Some(m) = best_match(search_query, name.as_str()) {
                matching_entities.push((entity, name.to_string(), m.score()));
            }
        }

        // Also fuzzy search by Entity ID (e.g., "123v4")
        let mut q_all = world.query::<EntityRef>();
        for entity_ref in q_all.iter(world) {
            let entity_id = entity_ref.id();
            let id_str = format!("{}v{}", entity_id.index(), entity_id.generation());
            if let Some(m) = best_match(search_query, &id_str) {
                let name = entity_ref
                    .get::<Name>()
                    .map_or_else(|| id_str.clone(), std::string::ToString::to_string);
                if !matching_entities.iter().any(|(e, _, _)| *e == entity_id) {
                    matching_entities.push((entity_id, name, m.score()));
                }
            }
        }

        // Sort by score descending (best matches first)
        matching_entities.sort_by(|(_, _, a), (_, _, b)| b.cmp(a));

        // Display results
        ui.label(format!("{} results", matching_entities.len()));
        ui.add_space(4.0);

        for (entity, display_name, _) in matching_entities {
            let is_selected = selected_entities.contains(entity);
            let label = format!("{display_name} ({entity:?})");

            if ui.selectable_label(is_selected, label).clicked() {
                let modifiers = ui.input(|i| i.modifiers);
                let add_to_selection = modifiers.ctrl || modifiers.shift;
                selected_entities.select_maybe_add(entity, add_to_selection);
                *selection = InspectorSelection::Entities;
            }
        }
    });
}
