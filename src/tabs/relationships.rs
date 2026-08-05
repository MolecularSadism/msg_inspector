//! Relationship navigation for the Inspector tab.
//!
//! Every relationship component on the inspected entity — [`ChildOf`], [`Children`],
//! and any user-defined `#[relationship]` / `#[relationship_target]` component — is
//! listed with one select button per referenced entity, so the inspection target can
//! be moved along the relationship graph without going back to the Hierarchy tab.
//!
//! Discovery is type-erased: it reads
//! [`ComponentInfo::relationship_accessor`](bevy::ecs::component::ComponentInfo::relationship_accessor),
//! which Bevy registers for every relationship component, so no per-type registration
//! is needed here.

use bevy::ecs::component::ComponentId;
use bevy::ecs::relationship::RelationshipAccessor;
use bevy::prelude::*;
use bevy_egui::egui;
use bevy_inspector_egui::bevy_inspector::hierarchy::SelectedEntities;

use crate::state::InspectorSelection;

/// Which side of a relationship a component sits on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelationshipKind {
    /// A `Relationship` component — points at the entity holding the target
    /// collection (e.g. [`ChildOf`]).
    Source,
    /// A `RelationshipTarget` component — holds the collection of entities
    /// pointing back at this one (e.g. [`Children`]).
    Target,
}

impl RelationshipKind {
    /// Arrow glyph indicating the direction of the relationship.
    const fn glyph(self) -> &'static str {
        match self {
            Self::Source => "→",
            Self::Target => "←",
        }
    }

    /// Tooltip text describing the direction of the relationship.
    const fn description(self) -> &'static str {
        match self {
            Self::Source => "Relationship: this entity points at the entity below",
            Self::Target => "Relationship target: the entities below point at this entity",
        }
    }
}

/// One relationship component found on the inspected entity.
pub struct RelationshipEntry {
    /// Component holding the relationship.
    pub component_id: ComponentId,
    /// Display name of the component (short type path when registered).
    pub name: String,
    /// Which side of the relationship this component represents.
    pub kind: RelationshipKind,
    /// Entities referenced by the component.
    pub entities: Vec<Entity>,
}

/// Collect every relationship component on `entity` together with the entities it references.
///
/// Returns an empty vector when the entity does not exist or holds no relationship
/// components. Entries are sorted by component name for a stable UI ordering.
///
/// # Example
///
/// ```
/// use bevy::prelude::*;
/// use msg_inspector::collect_relationships;
///
/// let mut world = World::new();
/// let parent = world.spawn_empty().id();
/// let child = world.spawn(ChildOf(parent)).id();
/// world.flush();
///
/// let entries = collect_relationships(&world, child);
/// assert_eq!(entries.len(), 1);
/// assert_eq!(entries[0].entities, vec![parent]);
/// ```
#[must_use]
pub fn collect_relationships(world: &World, entity: Entity) -> Vec<RelationshipEntry> {
    let Ok(entity_ref) = world.get_entity(entity) else {
        return Vec::new();
    };

    let type_registry = world.get_resource::<AppTypeRegistry>().map(|r| r.0.clone());
    let type_registry = type_registry.as_ref().map(|r| r.read());

    let mut entries: Vec<RelationshipEntry> = entity_ref
        .archetype()
        .components()
        .iter()
        .filter_map(|&component_id| {
            let info = world.components().get_info(component_id)?;
            let accessor = info.relationship_accessor()?;
            let ptr = entity_ref.get_by_id(component_id).ok()?;

            let (kind, entities) = match accessor {
                RelationshipAccessor::Relationship {
                    entity_field_offset,
                    ..
                } => {
                    // SAFETY: `ptr` points at the value of `component_id` on this entity, which is
                    // the component the accessor was registered for; the accessor guarantees the
                    // offset is in bounds and holds a valid `Entity`.
                    let target = unsafe { *ptr.byte_add(*entity_field_offset).deref::<Entity>() };
                    (RelationshipKind::Source, vec![target])
                }
                RelationshipAccessor::RelationshipTarget { iter, .. } => {
                    // SAFETY: `ptr` points at the value of `component_id` on this entity, which is
                    // the component the accessor was registered for.
                    let entities = unsafe { iter(ptr) }.collect();
                    (RelationshipKind::Target, entities)
                }
            };

            let name = info
                .type_id()
                .and_then(|type_id| type_registry.as_ref()?.get(type_id))
                .map_or_else(
                    || info.name().to_string(),
                    |registration| {
                        registration
                            .type_info()
                            .type_path_table()
                            .short_path()
                            .to_string()
                    },
                );

            Some(RelationshipEntry {
                component_id,
                name,
                kind,
                entities,
            })
        })
        .collect();

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

/// Number of referenced entities above which the list gets its own scroll area.
const SCROLL_THRESHOLD: usize = 12;

/// Maximum height of the scroll area used for long relationship lists.
const SCROLL_MAX_HEIGHT: f32 = 240.0;

/// Render the relationships section for a single inspected entity.
///
/// Draws nothing when the entity holds no relationship components. Clicking a
/// select button makes that entity the inspection target; holding Ctrl or Shift
/// adds it to the current selection instead of replacing it.
pub(crate) fn render(
    ui: &mut egui::Ui,
    world: &World,
    entity: Entity,
    selected_entities: &mut SelectedEntities,
    selection: &mut InspectorSelection,
) {
    let entries = collect_relationships(world, entity);
    if entries.is_empty() {
        return;
    }

    egui::CollapsingHeader::new(egui::RichText::new("Relationships").strong())
        .default_open(true)
        .show(ui, |ui| {
            for entry in &entries {
                render_entry(ui, world, entry, selected_entities, selection);
            }
        });
}

/// Render one relationship component and its referenced entities.
fn render_entry(
    ui: &mut egui::Ui,
    world: &World,
    entry: &RelationshipEntry,
    selected_entities: &mut SelectedEntities,
    selection: &mut InspectorSelection,
) {
    let header = format!(
        "{} {} ({})",
        entry.kind.glyph(),
        entry.name,
        entry.entities.len()
    );

    egui::CollapsingHeader::new(header)
        .id_salt(entry.component_id)
        .default_open(true)
        .show(ui, |ui| {
            ui.label(egui::RichText::new(entry.kind.description()).weak().small());

            if entry.entities.len() > SCROLL_THRESHOLD {
                egui::ScrollArea::vertical()
                    .id_salt(entry.component_id)
                    .max_height(SCROLL_MAX_HEIGHT)
                    .show(ui, |ui| {
                        render_targets(ui, world, entry, selected_entities, selection);
                    });
            } else {
                render_targets(ui, world, entry, selected_entities, selection);
            }
        });
}

/// Render one selectable row per referenced entity.
fn render_targets(
    ui: &mut egui::Ui,
    world: &World,
    entry: &RelationshipEntry,
    selected_entities: &mut SelectedEntities,
    selection: &mut InspectorSelection,
) {
    for &target in &entry.entities {
        let target_ref = world.get_entity(target).ok();
        let alive = target_ref.is_some();
        let label = target_ref
            .and_then(|e| e.get::<Name>())
            .map_or_else(|| format!("{target}"), |name| format!("{name} ({target})"));

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;

            let button = ui
                .add_enabled(alive, egui::Button::new("⏵").small())
                .on_hover_text("Inspect this entity")
                .on_disabled_hover_text("Entity no longer exists");

            if button.clicked() {
                let modifiers = ui.input(|i| i.modifiers);
                let add_to_selection = modifiers.ctrl || modifiers.shift;
                selected_entities.select_maybe_add(target, add_to_selection);
                *selection = InspectorSelection::Entities;
            }

            let text = egui::RichText::new(label);
            let text = if selected_entities.contains(target) {
                text.strong()
            } else if alive {
                text
            } else {
                text.weak().italics()
            };
            ui.label(text);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Component)]
    #[relationship(relationship_target = Followers)]
    struct Follows(Entity);

    #[derive(Component)]
    #[relationship_target(relationship = Follows)]
    struct Followers(Vec<Entity>);

    #[test]
    fn collects_child_of_and_children() {
        let mut world = World::new();
        let parent = world.spawn_empty().id();
        let child_a = world.spawn(ChildOf(parent)).id();
        let child_b = world.spawn(ChildOf(parent)).id();
        world.flush();

        let child_entries = collect_relationships(&world, child_a);
        assert_eq!(child_entries.len(), 1);
        assert_eq!(child_entries[0].kind, RelationshipKind::Source);
        assert_eq!(child_entries[0].entities, vec![parent]);

        let parent_entries = collect_relationships(&world, parent);
        assert_eq!(parent_entries.len(), 1);
        assert_eq!(parent_entries[0].kind, RelationshipKind::Target);
        assert_eq!(parent_entries[0].entities, vec![child_a, child_b]);
    }

    #[test]
    fn collects_custom_relationships() {
        let mut world = World::new();
        let leader = world.spawn_empty().id();
        let follower = world.spawn(Follows(leader)).id();
        world.flush();

        let follower_entries = collect_relationships(&world, follower);
        assert_eq!(follower_entries.len(), 1);
        assert_eq!(follower_entries[0].entities, vec![leader]);

        let leader_entries = collect_relationships(&world, leader);
        assert_eq!(leader_entries.len(), 1);
        assert_eq!(leader_entries[0].kind, RelationshipKind::Target);
        assert_eq!(leader_entries[0].entities, vec![follower]);
    }

    #[test]
    fn ignores_non_relationship_components_and_missing_entities() {
        let mut world = World::new();
        let entity = world.spawn(Name::new("lonely")).id();
        world.flush();

        assert!(collect_relationships(&world, entity).is_empty());

        world.despawn(entity);
        assert!(collect_relationships(&world, entity).is_empty());
    }

    #[test]
    fn multiple_relationships_are_listed_in_stable_order() {
        let mut world = World::new();
        let other = world.spawn_empty().id();
        let entity = world.spawn((ChildOf(other), Follows(other))).id();
        world.flush();

        let names: Vec<String> = collect_relationships(&world, entity)
            .iter()
            .map(|entry| entry.name.clone())
            .collect();
        assert_eq!(names.len(), 2);
        assert!(names.windows(2).all(|pair| pair[0] <= pair[1]));
        assert!(names.iter().any(|name| name.ends_with("ChildOf")));
        assert!(names.iter().any(|name| name.ends_with("Follows")));
    }
}
