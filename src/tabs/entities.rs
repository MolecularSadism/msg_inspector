//! Entity browser tab sorted by principal components.
//!
//! Provides a categorized view of entities grouped by registered "principal"
//! component types. Each principal component gets a collapsible tree.
//! Entities without any principal component appear under "Uncategorized".

use std::any::TypeId;

use bevy::ecs::component::ComponentId;
use bevy::ecs::world::EntityRef;
use bevy::prelude::*;
use bevy_egui::egui;
use bevy_inspector_egui::bevy_inspector::hierarchy::SelectedEntities;
use sublime_fuzzy::best_match;

use crate::state::InspectorSelection;

/// A registered principal component: its label, `ComponentId`, and `TypeId`.
pub struct PrincipalEntry {
    pub label: String,
    pub component_id: ComponentId,
    pub type_id: TypeId,
}

/// Resource that stores principal component registrations.
///
/// Register principal components during app setup via
/// [`InspectorExt::register_principal`]. In the **Entities** tab each
/// principal gets its own collapsible tree containing every entity that
/// has that component. Entities without *any* registered principal appear
/// under an **Uncategorized** folder.
#[derive(Resource, Default)]
pub struct PrincipalRegistry {
    /// Deferred registrations (type_id + label) collected before the world
    /// has component IDs assigned. Resolved on first use.
    deferred: Vec<(TypeId, String)>,
    /// Resolved entries with valid `ComponentId`s.
    entries: Vec<PrincipalEntry>,
}

impl PrincipalRegistry {
    /// Queue a component type for registration. The `ComponentId` is resolved
    /// lazily the first time the Entities tab renders.
    pub fn register<C: Component>(&mut self) {
        let type_id = TypeId::of::<C>();
        let label = pretty_type_name::<C>();
        self.deferred.push((type_id, label));
    }

    /// Resolve any deferred registrations using the live `World`.
    fn resolve(&mut self, world: &World) {
        for (type_id, label) in self.deferred.drain(..) {
            if let Some(component_id) = world.components().get_id(type_id) {
                // Avoid duplicates
                if !self.entries.iter().any(|e| e.type_id == type_id) {
                    self.entries.push(PrincipalEntry {
                        label,
                        component_id,
                        type_id,
                    });
                }
            }
        }
        // Keep entries sorted alphabetically by label
        self.entries.sort_by(|a, b| a.label.cmp(&b.label));
    }

    pub fn entries(&self) -> &[PrincipalEntry] {
        &self.entries
    }
}

/// Derive a short, human-readable name from the full type path.
fn pretty_type_name<T: 'static>() -> String {
    let full = std::any::type_name::<T>();
    full.rsplit("::").next().unwrap_or(full).to_string()
}

/// Persistent state for the Entities tab, stored inside `UiState`.
#[derive(Default)]
pub struct EntitiesTabState {
    /// Index of the currently active/selected principal tree (`None` = show all).
    pub active_tree: Option<usize>,
    /// Fuzzy search input.
    pub search: String,
}

/// Snapshot of a single principal (label + component id), cheap to clone.
#[derive(Clone)]
struct PrincipalSnapshot {
    label: String,
    component_id: ComponentId,
}

/// Render the entities tab.
pub fn render(
    ui: &mut egui::Ui,
    world: &mut World,
    selected_entities: &mut SelectedEntities,
    selection: &mut InspectorSelection,
    tab_state: &mut EntitiesTabState,
) {
    // Resolve deferred registrations and snapshot the principal data so we
    // can release the borrow on the registry before querying the world.
    let snapshots: Vec<PrincipalSnapshot> =
        world.resource_scope::<PrincipalRegistry, _>(|world, mut registry| {
            registry.resolve(world);
            registry
                .entries()
                .iter()
                .map(|e| PrincipalSnapshot {
                    label: e.label.clone(),
                    component_id: e.component_id,
                })
                .collect()
        });

    if snapshots.is_empty() {
        ui.label("No principal components registered.");
        ui.label("Use app.register_principal::<C>() to register components.");
        return;
    }

    // --- Tree selector (dropdown) ---
    ui.horizontal(|ui| {
        ui.label("Category:");
        let current_label = tab_state
            .active_tree
            .and_then(|idx| snapshots.get(idx))
            .map_or("All", |s| s.label.as_str());

        egui::ComboBox::from_id_salt("entities_tree_selector")
            .selected_text(current_label)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(tab_state.active_tree.is_none(), "All")
                    .clicked()
                {
                    tab_state.active_tree = None;
                }
                for (idx, snap) in snapshots.iter().enumerate() {
                    if ui
                        .selectable_label(tab_state.active_tree == Some(idx), &snap.label)
                        .clicked()
                    {
                        tab_state.active_tree = Some(idx);
                    }
                }
                // Uncategorized option
                let uncategorized_idx = snapshots.len();
                if ui
                    .selectable_label(
                        tab_state.active_tree == Some(uncategorized_idx),
                        "Uncategorized",
                    )
                    .clicked()
                {
                    tab_state.active_tree = Some(uncategorized_idx);
                }
            });

        // X button to clear selection
        if tab_state.active_tree.is_some() && ui.small_button("X").clicked() {
            tab_state.active_tree = None;
        }
    });

    // --- Fuzzy search ---
    ui.horizontal(|ui| {
        ui.label("Search:");
        ui.text_edit_singleline(&mut tab_state.search);
        if ui.small_button("X").clicked() {
            tab_state.search.clear();
        }
    });

    ui.separator();

    // --- Build entity lists per principal ---
    let search_query = tab_state.search.trim().to_owned();
    let uncategorized_idx = snapshots.len();

    egui::ScrollArea::vertical().show(ui, |ui| {
        if let Some(active) = tab_state.active_tree {
            if active == uncategorized_idx {
                render_uncategorized_section(
                    ui,
                    world,
                    &snapshots,
                    selected_entities,
                    selection,
                    &search_query,
                    true,
                );
            } else if let Some(snap) = snapshots.get(active) {
                render_principal_section(
                    ui,
                    world,
                    snap,
                    selected_entities,
                    selection,
                    &search_query,
                    true,
                );
            }
        } else {
            for snap in &snapshots {
                render_principal_section(
                    ui,
                    world,
                    snap,
                    selected_entities,
                    selection,
                    &search_query,
                    false,
                );
            }
            render_uncategorized_section(
                ui,
                world,
                &snapshots,
                selected_entities,
                selection,
                &search_query,
                false,
            );
        }
    });
}

/// Render a collapsible section for one principal component.
fn render_principal_section(
    ui: &mut egui::Ui,
    world: &mut World,
    snap: &PrincipalSnapshot,
    selected_entities: &mut SelectedEntities,
    selection: &mut InspectorSelection,
    search_query: &str,
    always_open: bool,
) {
    let entities = collect_entities_with_component(world, snap.component_id, search_query);
    let header = format!("{} ({})", snap.label, entities.len());

    let mut collapsing = egui::CollapsingHeader::new(header).id_salt(&snap.label);
    if always_open {
        collapsing = collapsing.default_open(true);
    }
    collapsing.show(ui, |ui| {
        if entities.is_empty() {
            ui.weak("No matching entities");
        } else {
            render_entity_list(ui, &entities, selected_entities, selection);
        }
    });
}

/// Render the Uncategorized section for entities without any principal.
fn render_uncategorized_section(
    ui: &mut egui::Ui,
    world: &mut World,
    snapshots: &[PrincipalSnapshot],
    selected_entities: &mut SelectedEntities,
    selection: &mut InspectorSelection,
    search_query: &str,
    always_open: bool,
) {
    let component_ids: Vec<ComponentId> = snapshots.iter().map(|s| s.component_id).collect();
    let entities = collect_uncategorized_entities(world, &component_ids, search_query);
    let header = format!("Uncategorized ({})", entities.len());

    let mut collapsing =
        egui::CollapsingHeader::new(header).id_salt("entities_uncategorized");
    if always_open {
        collapsing = collapsing.default_open(true);
    }
    collapsing.show(ui, |ui| {
        if entities.is_empty() {
            ui.weak("No matching entities");
        } else {
            render_entity_list(ui, &entities, selected_entities, selection);
        }
    });
}

/// Collect all entities that have a given component, optionally filtered by fuzzy search.
fn collect_entities_with_component(
    world: &mut World,
    component_id: ComponentId,
    search_query: &str,
) -> Vec<(Entity, String, isize)> {
    let mut results: Vec<(Entity, String, isize)> = Vec::new();

    let mut q = world.query::<EntityRef>();
    for entity_ref in q.iter(world) {
        if !entity_ref.contains_id(component_id) {
            continue;
        }

        let display_name = entity_display_name(&entity_ref);

        if search_query.is_empty() {
            results.push((entity_ref.id(), display_name, 0));
        } else if let Some(m) = best_match(search_query, &display_name) {
            results.push((entity_ref.id(), display_name, m.score()));
        }
    }

    if search_query.is_empty() {
        results.sort_by(|(_, a, _), (_, b, _)| a.cmp(b));
    } else {
        results.sort_by(|(_, _, sa), (_, _, sb)| sb.cmp(sa));
    }

    results
}

/// Collect entities that do *not* have any of the given principal components.
fn collect_uncategorized_entities(
    world: &mut World,
    principal_ids: &[ComponentId],
    search_query: &str,
) -> Vec<(Entity, String, isize)> {
    let mut results: Vec<(Entity, String, isize)> = Vec::new();

    let mut q = world.query::<EntityRef>();
    for entity_ref in q.iter(world) {
        let has_any = principal_ids
            .iter()
            .any(|id| entity_ref.contains_id(*id));
        if has_any {
            continue;
        }

        let display_name = entity_display_name(&entity_ref);

        if search_query.is_empty() {
            results.push((entity_ref.id(), display_name, 0));
        } else if let Some(m) = best_match(search_query, &display_name) {
            results.push((entity_ref.id(), display_name, m.score()));
        }
    }

    if search_query.is_empty() {
        results.sort_by(|(_, a, _), (_, b, _)| a.cmp(b));
    } else {
        results.sort_by(|(_, _, sa), (_, _, sb)| sb.cmp(sa));
    }

    results
}

/// Get a display name for an entity: its `Name` component or its ID.
fn entity_display_name(entity_ref: &EntityRef) -> String {
    entity_ref.get::<Name>().map_or_else(
        || {
            let id = entity_ref.id();
            format!("Entity {}v{}", id.index(), id.generation())
        },
        |name| name.to_string(),
    )
}

/// Render a flat list of entities as selectable labels.
fn render_entity_list(
    ui: &mut egui::Ui,
    entities: &[(Entity, String, isize)],
    selected_entities: &mut SelectedEntities,
    selection: &mut InspectorSelection,
) {
    for (entity, display_name, _) in entities {
        let is_selected = selected_entities.contains(*entity);
        let label = format!("{display_name} ({entity:?})");

        if ui.selectable_label(is_selected, label).clicked() {
            let modifiers = ui.input(|i| i.modifiers);
            let add_to_selection = modifiers.ctrl || modifiers.shift;
            selected_entities.select_maybe_add(*entity, add_to_selection);
            *selection = InspectorSelection::Entities;
        }
    }
}
