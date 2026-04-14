//! Resource browser tab.
//!
//! Lists all registered resources and allows selecting them for inspection.

use bevy::prelude::*;
use bevy::reflect::TypeRegistry;
use bevy_egui::egui;
use sublime_fuzzy::best_match;

use crate::state::InspectorSelection;

/// Render the resources tab.
pub fn render(
    ui: &mut egui::Ui,
    type_registry: &TypeRegistry,
    selection: &mut InspectorSelection,
    resources_search: &mut String,
) {
    super::search_bar(ui, "Search resources...", resources_search);
    ui.separator();

    let search_query = resources_search.trim();

    let mut resources: Vec<_> = type_registry
        .iter()
        .filter(|registration| registration.data::<ReflectResource>().is_some())
        .filter_map(|registration| {
            let name = registration.type_info().type_path_table().short_path();
            let type_id = registration.type_id();
            if search_query.is_empty() {
                Some((name, type_id, 0isize))
            } else {
                best_match(search_query, name).map(|m| (name, type_id, m.score()))
            }
        })
        .collect();

    if search_query.is_empty() {
        resources.sort_by(|(name_a, _, _), (name_b, _, _)| name_a.cmp(name_b));
    } else {
        resources.sort_by(|(_, _, a), (_, _, b)| b.cmp(a));
    }

    if resources.is_empty() {
        ui.add_space(12.0);
        ui.vertical_centered(|ui| {
            if search_query.is_empty() {
                ui.label(egui::RichText::new("No registered resources").weak());
            } else {
                ui.label(egui::RichText::new("No matching resources").weak());
            }
        });
        return;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        for (resource_name, type_id, _) in resources {
            let selected = match *selection {
                InspectorSelection::Resource(selected, _) => selected == type_id,
                _ => false,
            };

            if ui.selectable_label(selected, resource_name).clicked() {
                *selection = InspectorSelection::Resource(type_id, resource_name.to_string());
            }
        }
    });
}
