//! Entity browser tab sorted by principal components.
//!
//! Provides a categorized view of entities grouped by registered "principal"
//! component types. Each principal component gets a collapsible tree.
//! Principals can be organized into named groups via
//! [`register_principal_group`](crate::tabs::InspectorExt::register_principal_group),
//! which pins them under a shared parent category in the inspector.
//! Entities without any principal component appear under "Uncategorized".

use std::any::TypeId;

use bevy::ecs::component::ComponentId;
use bevy::ecs::world::EntityRef;
use bevy::prelude::*;
use bevy_egui::egui;
use bevy_inspector_egui::bevy_inspector::hierarchy::SelectedEntities;
use sublime_fuzzy::best_match;

use crate::state::InspectorSelection;

/// A registered principal component: its label, `ComponentId`, `TypeId`, and
/// optional group membership.
pub struct PrincipalEntry {
    pub label: String,
    pub component_id: ComponentId,
    pub type_id: TypeId,
    /// If `Some`, this principal belongs to a named group that is displayed as a
    /// parent category in the Entities tab.
    pub group: Option<String>,
}

/// Queued registration collected before `ComponentId`s are available.
struct DeferredPrincipal {
    type_id: TypeId,
    label: String,
    group: Option<String>,
}

/// Resource that stores principal component registrations.
///
/// Register principal components during app setup via
/// [`InspectorExt::register_principal`]. In the **Entities** tab each
/// principal gets its own collapsible tree containing every entity that
/// has that component. Use [`InspectorExt::register_principal_group`] to
/// organize principals under a shared parent category. Entities without
/// *any* registered principal appear under an **Uncategorized** folder.
#[derive(Resource, Default)]
pub struct PrincipalRegistry {
    /// Deferred registrations collected before the world has component IDs
    /// assigned. Resolved on first use.
    deferred: Vec<DeferredPrincipal>,
    /// Resolved entries with valid `ComponentId`s.
    entries: Vec<PrincipalEntry>,
}

impl PrincipalRegistry {
    /// Queue a component type for registration. The `ComponentId` is resolved
    /// lazily the first time the Entities tab renders.
    pub fn register<C: Component>(&mut self) {
        let type_id = TypeId::of::<C>();
        let label = pretty_type_name::<C>();
        self.deferred.push(DeferredPrincipal {
            type_id,
            label,
            group: None,
        });
    }

    /// Queue a component type with a custom display name.
    pub fn register_named<C: Component>(&mut self, name: &str) {
        let type_id = TypeId::of::<C>();
        self.deferred.push(DeferredPrincipal {
            type_id,
            label: name.to_owned(),
            group: None,
        });
    }

    /// Override the display name of the most recently queued principal.
    ///
    /// This is the backing method for [`InspectorExt::with_name`].
    pub fn set_last_name(&mut self, name: String) {
        if let Some(last) = self.deferred.last_mut() {
            last.label = name;
        }
    }

    /// Register a group of principal components under a shared parent category.
    pub fn register_group<P: PrincipalTuple>(&mut self, name: &str) {
        P::register_all(self, name);
    }

    /// Resolve any deferred registrations using the live `World`.
    fn resolve(&mut self, world: &World) {
        for deferred in self.deferred.drain(..) {
            if let Some(component_id) = world.components().get_id(deferred.type_id) {
                // Later registrations override earlier ones (e.g. a group
                // registration overrides a standalone registration).
                if let Some(existing) = self
                    .entries
                    .iter_mut()
                    .find(|e| e.type_id == deferred.type_id)
                {
                    existing.label = deferred.label;
                    existing.group = deferred.group;
                } else {
                    self.entries.push(PrincipalEntry {
                        label: deferred.label,
                        component_id,
                        type_id: deferred.type_id,
                        group: deferred.group,
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

/// Trait for registering a tuple of [`Component`] types as a principal group.
///
/// Implemented automatically for tuples of components up to arity 8.
/// You do not need to implement this trait yourself — pass a tuple to
/// [`InspectorExt::register_principal_group`] and the blanket implementation
/// handles the rest.
pub trait PrincipalTuple {
    /// Push deferred registrations for every component in the tuple.
    fn register_all(registry: &mut PrincipalRegistry, group: &str);
}

macro_rules! impl_principal_tuple {
    ($($T:ident),+) => {
        impl<$($T: Component),+> PrincipalTuple for ($($T,)+) {
            fn register_all(registry: &mut PrincipalRegistry, group: &str) {
                $(
                    registry.deferred.push(DeferredPrincipal {
                        type_id: TypeId::of::<$T>(),
                        label: pretty_type_name::<$T>(),
                        group: Some(group.to_owned()),
                    });
                )+
            }
        }
    };
}

impl_principal_tuple!(A);
impl_principal_tuple!(A, B);
impl_principal_tuple!(A, B, C);
impl_principal_tuple!(A, B, C, D);
impl_principal_tuple!(A, B, C, D, E);
impl_principal_tuple!(A, B, C, D, E, F);
impl_principal_tuple!(A, B, C, D, E, F, G);
impl_principal_tuple!(A, B, C, D, E, F, G, H);

/// Which category is currently selected in the Entities tab dropdown.
#[derive(Default, Clone, PartialEq)]
pub enum ActiveCategory {
    /// Show all categories.
    #[default]
    All,
    /// A named group (index into the group snapshot list).
    Group(usize),
    /// A standalone (ungrouped) principal (index into the standalone list).
    Standalone(usize),
    /// Entities without any registered principal.
    Uncategorized,
}

/// Persistent state for the Entities tab, stored inside `UiState`.
#[derive(Default)]
pub struct EntitiesTabState {
    /// Currently selected category filter.
    pub active_category: ActiveCategory,
    /// Fuzzy search input.
    pub search: String,
}

/// Snapshot of a single principal (label + component id), cheap to clone.
#[derive(Clone)]
struct PrincipalSnapshot {
    label: String,
    component_id: ComponentId,
}

/// Snapshot of a named group containing several principals.
#[derive(Clone)]
struct GroupSnapshot {
    name: String,
    members: Vec<PrincipalSnapshot>,
}

/// Render the entities tab.
pub fn render(
    ui: &mut egui::Ui,
    world: &mut World,
    selected_entities: &mut SelectedEntities,
    selection: &mut InspectorSelection,
    tab_state: &mut EntitiesTabState,
) {
    // Resolve deferred registrations and build group / standalone snapshots
    // so we can release the borrow on the registry before querying the world.
    let (groups, standalones, all_component_ids): (
        Vec<GroupSnapshot>,
        Vec<PrincipalSnapshot>,
        Vec<ComponentId>,
    ) = world.resource_scope::<PrincipalRegistry, _>(|world, mut registry| {
        registry.resolve(world);

        let mut group_map: Vec<(String, Vec<PrincipalSnapshot>)> = Vec::new();
        let mut standalones = Vec::new();
        let mut all_ids = Vec::new();

        for entry in registry.entries() {
            let snap = PrincipalSnapshot {
                label: entry.label.clone(),
                component_id: entry.component_id,
            };
            all_ids.push(entry.component_id);

            if let Some(ref group_name) = entry.group {
                if let Some(g) = group_map.iter_mut().find(|(n, _)| n == group_name) {
                    g.1.push(snap);
                } else {
                    group_map.push((group_name.clone(), vec![snap]));
                }
            } else {
                standalones.push(snap);
            }
        }

        let groups: Vec<GroupSnapshot> = group_map
            .into_iter()
            .map(|(name, members)| GroupSnapshot { name, members })
            .collect();

        (groups, standalones, all_ids)
    });

    if groups.is_empty() && standalones.is_empty() {
        ui.label("No principal components registered.");
        ui.label("Use app.register_principal::<C>() to register components.");
        return;
    }

    // --- Category selector (dropdown) ---
    ui.horizontal(|ui| {
        ui.label("Category:");
        let current_label = match &tab_state.active_category {
            ActiveCategory::All => "All",
            ActiveCategory::Group(idx) => {
                groups.get(*idx).map_or("All", |g| g.name.as_str())
            }
            ActiveCategory::Standalone(idx) => {
                standalones.get(*idx).map_or("All", |s| s.label.as_str())
            }
            ActiveCategory::Uncategorized => "Uncategorized",
        };

        egui::ComboBox::from_id_salt("entities_tree_selector")
            .selected_text(current_label)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(tab_state.active_category == ActiveCategory::All, "All")
                    .clicked()
                {
                    tab_state.active_category = ActiveCategory::All;
                }
                for (idx, group) in groups.iter().enumerate() {
                    if ui
                        .selectable_label(
                            tab_state.active_category == ActiveCategory::Group(idx),
                            &group.name,
                        )
                        .clicked()
                    {
                        tab_state.active_category = ActiveCategory::Group(idx);
                    }
                }
                for (idx, snap) in standalones.iter().enumerate() {
                    if ui
                        .selectable_label(
                            tab_state.active_category == ActiveCategory::Standalone(idx),
                            &snap.label,
                        )
                        .clicked()
                    {
                        tab_state.active_category = ActiveCategory::Standalone(idx);
                    }
                }
                if ui
                    .selectable_label(
                        tab_state.active_category == ActiveCategory::Uncategorized,
                        "Uncategorized",
                    )
                    .clicked()
                {
                    tab_state.active_category = ActiveCategory::Uncategorized;
                }
            });

        if tab_state.active_category != ActiveCategory::All && ui.small_button("X").clicked() {
            tab_state.active_category = ActiveCategory::All;
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

    let search_query = tab_state.search.trim().to_owned();
    let active = tab_state.active_category.clone();

    egui::ScrollArea::vertical().show(ui, |ui| {
        match active {
            ActiveCategory::All => {
                // Groups pinned at top
                for group in &groups {
                    render_group_section(
                        ui,
                        world,
                        group,
                        selected_entities,
                        selection,
                        &search_query,
                        false,
                    );
                }
                for snap in &standalones {
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
                    &all_component_ids,
                    selected_entities,
                    selection,
                    &search_query,
                    false,
                );
            }
            ActiveCategory::Group(idx) => {
                if let Some(group) = groups.get(idx) {
                    render_group_section(
                        ui,
                        world,
                        group,
                        selected_entities,
                        selection,
                        &search_query,
                        true,
                    );
                }
            }
            ActiveCategory::Standalone(idx) => {
                if let Some(snap) = standalones.get(idx) {
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
            }
            ActiveCategory::Uncategorized => {
                render_uncategorized_section(
                    ui,
                    world,
                    &all_component_ids,
                    selected_entities,
                    selection,
                    &search_query,
                    true,
                );
            }
        }
    });
}

/// Render a collapsible group containing its member principal sections.
fn render_group_section(
    ui: &mut egui::Ui,
    world: &mut World,
    group: &GroupSnapshot,
    selected_entities: &mut SelectedEntities,
    selection: &mut InspectorSelection,
    search_query: &str,
    always_open: bool,
) {
    let mut collapsing =
        egui::CollapsingHeader::new(&group.name).id_salt(format!("group_{}", group.name));
    if always_open {
        collapsing = collapsing.default_open(true);
    }
    collapsing.show(ui, |ui| {
        for member in &group.members {
            render_principal_section(
                ui,
                world,
                member,
                selected_entities,
                selection,
                search_query,
                always_open,
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
    all_principal_ids: &[ComponentId],
    selected_entities: &mut SelectedEntities,
    selection: &mut InspectorSelection,
    search_query: &str,
    always_open: bool,
) {
    let entities = collect_uncategorized_entities(world, all_principal_ids, search_query);
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
