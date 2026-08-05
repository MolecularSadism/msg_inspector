//! Entity/resource/asset inspector tab.
//!
//! Displays detailed information about the currently selected item.
//! Supports custom inspector sections registered via [`InspectorSectionRegistry`].

use std::any::TypeId;

use bevy::prelude::*;
use bevy::reflect::TypeRegistry;
use bevy_egui::egui;
use bevy_inspector_egui::bevy_inspector::{
    self, hierarchy::SelectedEntities, ui_for_entities_shared_components,
    ui_for_entity_with_children,
};
use sublime_fuzzy::best_match;

use crate::state::InspectorSelection;

/// Render callback for a custom inspector section.
///
/// Receives the egui [`Ui`](egui::Ui), mutable [`World`], and selected [`Entity`].
pub type SectionRenderFn = Box<dyn Fn(&mut egui::Ui, &mut World, Entity) + Send + Sync>;

/// A registered custom section for the Inspector tab.
///
/// Each section is coupled to a marker component — it only renders
/// when the selected entity has that component.
pub struct InspectorSection {
    /// `TypeId` of the marker component that gates visibility.
    pub marker_type_id: TypeId,
    /// Title displayed as a collapsible header.
    pub title: String,
    /// Render function: receives the egui Ui, mutable World, and selected Entity.
    pub render_fn: SectionRenderFn,
}

/// Resource holding all registered custom inspector sections.
#[derive(Resource, Default)]
pub struct InspectorSectionRegistry {
    pub(crate) sections: Vec<InspectorSection>,
}

/// Check whether an entity has a component with the given `TypeId`.
fn entity_has_component(world: &World, entity: Entity, type_id: TypeId) -> bool {
    let Some(component_id) = world.components().get_id(type_id) else {
        return false;
    };
    world
        .get_entity(entity)
        .ok()
        .is_some_and(|e| e.archetype().contains(component_id))
}

/// Built-in inspector section for the Transform component.
///
/// Shows translation (x, y, z) as editable drag fields and rotation as
/// a read-only quaternion display.
pub fn transform_section_ui(ui: &mut egui::Ui, world: &mut World, entity: Entity) {
    let Some(transform) = world.get::<Transform>(entity).copied() else {
        return;
    };
    let mut translation = transform.translation;
    let rotation = transform.rotation;

    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("x");
        changed |= ui
            .add(egui::DragValue::new(&mut translation.x).speed(0.1))
            .changed();
        ui.label("y");
        changed |= ui
            .add(egui::DragValue::new(&mut translation.y).speed(0.1))
            .changed();
        ui.label("z");
        changed |= ui
            .add(egui::DragValue::new(&mut translation.z).speed(0.1))
            .changed();
    });

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Quaternion").weak());
        ui.label(format!(
            "({:.3}, {:.3}, {:.3}, {:.3})",
            rotation.x, rotation.y, rotation.z, rotation.w
        ));
    });

    if changed && let Some(mut t) = world.get_mut::<Transform>(entity) {
        t.translation = translation;
    }
}

/// Render the inspector tab.
pub fn render(
    ui: &mut egui::Ui,
    world: &mut World,
    type_registry: &TypeRegistry,
    selected_entities: &mut SelectedEntities,
    selection: &mut InspectorSelection,
    inspector_search: &mut String,
) {
    match selection {
        InspectorSelection::Entities => {
            if selected_entities.as_slice().is_empty() {
                ui.add_space(20.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("◆").size(24.0).weak());
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("No entity selected").weak());
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new("Select an entity from the Hierarchy or Entities tab.")
                            .weak()
                            .small(),
                    );
                });
                return;
            }

            // Copy the selection out so it can be mutated while rendering (relationship
            // buttons change the inspection target).
            let entities = selected_entities.as_slice().to_vec();
            match entities.as_slice() {
                &[entity] => {
                    // ── Entity name ──────────────────────────────────
                    let entity_name = world
                        .get_entity(entity)
                        .ok()
                        .and_then(|e| e.get::<Name>().map(|n| n.to_string()))
                        .unwrap_or_else(|| format!("Entity {}", entity));

                    ui.heading(egui::RichText::new(&entity_name).strong());
                    ui.label(egui::RichText::new(format!("{entity:?}")).weak().small());
                    ui.add_space(4.0);

                    // ── Relationships ────────────────────────────────
                    super::relationships::render(ui, world, entity, selected_entities, selection);

                    // ── Custom sections ──────────────────────────────
                    render_custom_sections(ui, world, entity);

                    // ── Separator ────────────────────────────────────
                    ui.add_space(2.0);
                    ui.separator();
                    ui.add_space(2.0);

                    // ── Default inspector view ──────────────────────
                    super::search_bar(ui, "Filter components...", inspector_search);
                    let search_query = inspector_search.trim().to_string();
                    if !search_query.is_empty() {
                        render_matching_components_list(ui, world, &entities, &search_query);
                        ui.separator();
                    }
                    ui_for_entity_with_children(world, entity, ui);
                }
                entities => {
                    super::search_bar(ui, "Filter components...", inspector_search);
                    ui.separator();
                    let search_query = inspector_search.trim().to_string();
                    if !search_query.is_empty() {
                        render_matching_components_list(ui, world, entities, &search_query);
                        ui.separator();
                    }
                    ui_for_entities_shared_components(world, entities, ui);
                }
            }
        }
        InspectorSelection::Resource(type_id, name) => {
            ui.label(name.as_str());
            bevy_inspector::by_type_id::ui_for_resource(world, *type_id, ui, name, type_registry);
        }
        InspectorSelection::Asset(type_id, name, handle) => {
            ui.label(name.as_str());
            bevy_inspector::by_type_id::ui_for_asset(world, *type_id, *handle, ui, type_registry);
        }
    }
}

/// Render all matching custom sections for a single selected entity.
fn render_custom_sections(ui: &mut egui::Ui, world: &mut World, entity: Entity) {
    // Temporarily remove the registry to avoid aliasing &mut World
    let Some(registry) = world.remove_resource::<InspectorSectionRegistry>() else {
        return;
    };

    for section in &registry.sections {
        if entity_has_component(world, entity, section.marker_type_id) {
            egui::CollapsingHeader::new(egui::RichText::new(&section.title).strong())
                .default_open(true)
                .show(ui, |ui| {
                    (section.render_fn)(ui, world, entity);
                });
        }
    }

    world.insert_resource(registry);
}

/// Show a list of component names that match the fuzzy search query.
fn render_matching_components_list(
    ui: &mut egui::Ui,
    world: &World,
    entities: &[Entity],
    search_query: &str,
) {
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
            ui.label(
                egui::RichText::new("No matching components")
                    .weak()
                    .italics(),
            );
        } else {
            ui.label(format!("{} matching components:", matching.len()));
            for (name, _) in &matching {
                ui.small(egui::RichText::new(*name).strong());
            }
        }
    }
}
