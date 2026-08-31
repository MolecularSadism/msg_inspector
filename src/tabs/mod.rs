//! Tab definitions and registration for the inspector.

mod assets;
mod diagnostics;
pub(crate) mod entities;
mod game_view;
mod gpu;
mod hierarchy;
mod inspector;
pub(crate) mod relationships;
mod resources;

pub use diagnostics::{DiagnosticsCounters, FrameTimeHistory, update_frame_time_history};
pub use entities::{ActiveCategory, EntitiesTabState, PrincipalRegistry, PrincipalTuple};
pub use inspector::{InspectorSection, InspectorSectionRegistry, transform_section_ui};
pub use relationships::{RelationshipEntry, RelationshipKind, collect_relationships};

use std::any::TypeId;

use bevy::prelude::*;
use bevy::reflect::{TypePath, Typed};
use bevy_egui::egui;
use bevy_inspector_egui::bevy_inspector::hierarchy::SelectedEntities;

use crate::BitmaskRegistry;
use crate::state::InspectorSelection;

/// Render a consistent search/filter bar with hint text and a clear button.
pub(crate) fn search_bar(ui: &mut egui::Ui, hint: &str, text: &mut String) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.label(egui::RichText::new("▶").small().weak());
        ui.add(
            egui::TextEdit::singleline(text)
                .desired_width(ui.available_width() - 24.0)
                .hint_text(egui::RichText::new(hint).weak()),
        );
        if !text.is_empty() && ui.small_button("X").on_hover_text("Clear").clicked() {
            text.clear();
        }
    });
}

/// Trait for registering custom dev panel tabs.
///
/// Implement this trait to create custom tabs with full control over
/// rendering and state management.
///
/// # Example
///
/// ```
/// use bevy::prelude::*;
/// use msg_inspector::prelude::*;
/// use msg_inspector::egui;
///
/// /// A custom tab that displays entity statistics.
/// struct EntityStatsTab {
///     title: String,
/// }
///
/// impl InspectorTab for EntityStatsTab {
///     fn id(&self) -> &'static str {
///         "entity_stats"
///     }
///
///     fn title(&self) -> &str {
///         &self.title
///     }
///
///     fn ui(&mut self, ui: &mut egui::Ui, world: &mut World) {
///         ui.heading("Entity Statistics");
///
///         // Query world for data
///         let mut query = world.query::<&Transform>();
///         ui.label(format!("Entities with Transform: {}", query.iter(world).count()));
///     }
///
///     fn dock_position(&self) -> DockPosition {
///         DockPosition::Right
///     }
///
///     fn is_visible(&self, world: &World) -> bool {
///         world.entities().len() > 0
///     }
/// }
///
/// // Register the custom tab
/// fn setup_inspector(app: &mut App) {
///     app.register_inspector_tab(EntityStatsTab {
///         title: "Entity Stats".to_string(),
///     });
/// }
/// fn main() {}
/// ```
pub trait InspectorTab: Send + Sync + 'static {
    /// Unique identifier for this tab.
    fn id(&self) -> &'static str;

    /// Display name shown in tab header.
    fn title(&self) -> &str;

    /// Render the tab UI.
    fn ui(&mut self, ui: &mut egui::Ui, world: &mut World);

    /// Preferred dock position (default: Bottom).
    fn dock_position(&self) -> DockPosition {
        DockPosition::Bottom
    }

    /// Whether this tab is visible (default: always visible).
    fn is_visible(&self, _world: &World) -> bool {
        true
    }
}

/// Preferred dock position for a tab.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum DockPosition {
    Left,
    Right,
    #[default]
    Bottom,
    Center,
}

/// Resource for registering custom tabs.
#[derive(Resource, Default)]
pub struct InspectorTabRegistry {
    pub(crate) tabs: Vec<Box<dyn InspectorTab>>,
}

impl InspectorTabRegistry {
    /// Register a custom tab.
    pub fn register<T: InspectorTab>(&mut self, tab: T) {
        self.tabs.push(Box::new(tab));
    }

    /// Get all registered tabs.
    #[must_use]
    pub fn tabs(&self) -> &[Box<dyn InspectorTab>] {
        &self.tabs
    }

    /// Get mutable access to all registered tabs.
    pub fn tabs_mut(&mut self) -> &mut [Box<dyn InspectorTab>] {
        &mut self.tabs
    }

    /// Number of registered custom tabs.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    /// Whether there are any registered custom tabs.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }
}

/// Extension trait for App to register inspector tabs.
pub trait InspectorExt {
    /// Register a principal component type for the Entities tab.
    ///
    /// Each registered principal component gets a collapsible tree in the
    /// Entities tab, listing every entity that has this component. Entities
    /// without any registered principal appear under "Uncategorized".
    ///
    /// # Example
    ///
    /// ```
    /// use bevy::prelude::*;
    /// use msg_inspector::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Enemy;
    ///
    /// #[derive(Component)]
    /// struct Npc;
    ///
    /// fn plugin(app: &mut App) {
    ///     app.add_plugins(InspectorPlugin::default());
    ///     app.register_principal::<Enemy>()
    ///        .register_principal::<Npc>();
    /// }
    /// ```
    fn register_principal<C: Component>(&mut self) -> &mut Self;

    /// Override the display name of the most recently registered principal.
    ///
    /// Chain this immediately after [`register_principal`](Self::register_principal)
    /// to give the principal a custom label in the Entities tab.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy::prelude::*;
    /// use msg_inspector::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Enemy;
    ///
    /// fn plugin(app: &mut App) {
    ///     app.add_plugins(InspectorPlugin::default());
    ///     app.register_principal::<Enemy>().with_name("Bad Guys");
    /// }
    /// ```
    fn with_name(&mut self, name: impl Into<String>) -> &mut Self;

    /// Assign the most recently registered principal to a named group.
    ///
    /// Chain this immediately after [`register_principal`](Self::register_principal)
    /// to place the principal inside a collapsible group category in the Entities tab.
    /// Can be combined with [`with_name`](Self::with_name).
    ///
    /// # Example
    ///
    /// ```
    /// use bevy::prelude::*;
    /// use msg_inspector::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Enemy;
    ///
    /// #[derive(Component)]
    /// struct Npc;
    ///
    /// fn plugin(app: &mut App) {
    ///     app.add_plugins(InspectorPlugin::default());
    ///     app.register_principal::<Enemy>().with_group("Characters")
    ///        .register_principal::<Npc>().with_name("NPC").with_group("Characters");
    /// }
    /// ```
    fn with_group(&mut self, group: &str) -> &mut Self;

    /// Register a group of principal components under a shared parent category.
    ///
    /// The group appears as a collapsible parent in the Entities tab, with each
    /// component type as a nested section inside it.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy::prelude::*;
    /// use msg_inspector::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Enemy;
    ///
    /// #[derive(Component)]
    /// struct Npc;
    ///
    /// fn plugin(app: &mut App) {
    ///     app.add_plugins(InspectorPlugin::default());
    ///     app.register_principal_group::<(Enemy, Npc)>("Characters");
    /// }
    /// ```
    fn register_principal_group<P: PrincipalTuple>(&mut self, name: &str) -> &mut Self;

    /// Register a custom tab with full `InspectorTab` implementation.
    fn register_inspector_tab<T: InspectorTab>(&mut self, tab: T) -> &mut Self;

    /// Register a read-only analytics tab (no world mutation).
    ///
    /// The tab will be placed in the Bottom dock by default.
    /// Use [`Self::register_inspector_analytics_at`] to specify a custom dock position.
    fn register_inspector_analytics<F>(
        &mut self,
        id: &'static str,
        title: &'static str,
        ui_fn: F,
    ) -> &mut Self
    where
        F: Fn(&mut egui::Ui, &World) + Send + Sync + 'static;

    /// Register a read-only analytics tab at a specific dock position.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy::prelude::*;
    /// use msg_inspector::prelude::*;
    ///
    /// fn plugin(app: &mut App) {
    ///     app.add_plugins(InspectorPlugin::default());
    ///     app.register_inspector_analytics_at(
    ///         "stats",
    ///         "Statistics",
    ///         DockPosition::Right,
    ///         |ui, world| {
    ///             ui.label("Read-only stats here");
    ///         },
    ///     );
    /// }
    /// ```
    fn register_inspector_analytics_at<F>(
        &mut self,
        id: &'static str,
        title: &'static str,
        dock_position: DockPosition,
        ui_fn: F,
    ) -> &mut Self
    where
        F: Fn(&mut egui::Ui, &World) + Send + Sync + 'static;

    /// Register an interactive tab (can mutate world and trigger events).
    ///
    /// The tab will be placed in the Bottom dock by default.
    /// Use [`Self::register_inspector_interactive_at`] to specify a custom dock position.
    fn register_inspector_interactive<F>(
        &mut self,
        id: &'static str,
        title: &'static str,
        ui_fn: F,
    ) -> &mut Self
    where
        F: FnMut(&mut egui::Ui, &mut World) + Send + Sync + 'static;

    /// Register an interactive tab at a specific dock position.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy::prelude::*;
    /// use msg_inspector::prelude::*;
    ///
    /// fn plugin(app: &mut App) {
    ///     app.add_plugins(InspectorPlugin::default());
    ///     app.register_inspector_interactive_at(
    ///         "cheats",
    ///         "Cheats",
    ///         DockPosition::Left,
    ///         |ui, world| {
    ///             if ui.button("Heal Player").clicked() {
    ///                 // Mutate world state
    ///             }
    ///         },
    ///     );
    /// }
    /// ```
    fn register_inspector_interactive_at<F>(
        &mut self,
        id: &'static str,
        title: &'static str,
        dock_position: DockPosition,
        ui_fn: F,
    ) -> &mut Self
    where
        F: FnMut(&mut egui::Ui, &mut World) + Send + Sync + 'static;

    /// Register a custom inspector section coupled to a marker component.
    ///
    /// The section only renders in the Inspector tab when the selected entity
    /// has component `C`. Sections appear between the entity name and the
    /// default component inspector view.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy::prelude::*;
    /// use msg_inspector::prelude::*;
    /// use msg_inspector::egui;
    ///
    /// fn my_section(ui: &mut egui::Ui, world: &mut World, entity: Entity) {
    ///     ui.label("Custom section content");
    /// }
    ///
    /// fn plugin(app: &mut App) {
    ///     app.add_plugins(InspectorPlugin::default());
    ///     app.register_inspector_section::<Transform>("Transform", my_section);
    /// }
    /// ```
    fn register_inspector_section<C: Component>(
        &mut self,
        title: &str,
        render_fn: impl Fn(&mut egui::Ui, &mut World, Entity) + Send + Sync + 'static,
    ) -> &mut Self;

    /// Register a reflected enum as a named bitmask layer set for the wide
    /// bitmask widget ([`bitmask_field_layers`](crate::bitmask_field_layers)).
    ///
    /// Variant `i` (declaration order) names bit `i`, so this fits a
    /// bitflags-style enum declared in bit order. The set lands in the
    /// [`BitmaskRegistry`] resource, keyed by `E`; look it up at a widget call
    /// site with [`BitmaskRegistry::get`]. A type whose
    /// reflected form is not an enum is skipped; call
    /// [`BitmaskRegistry::register_enum`] directly for the `Result` if you need
    /// to detect that.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy::prelude::*;
    /// use msg_inspector::prelude::*;
    ///
    /// #[derive(Reflect)]
    /// enum PhysicsLayer {
    ///     Ground,
    ///     Water,
    ///     Air,
    /// }
    ///
    /// fn plugin(app: &mut App) {
    ///     app.add_plugins(InspectorPlugin::default());
    ///     app.register_bitmask_enum::<PhysicsLayer>();
    /// }
    /// ```
    fn register_bitmask_enum<E: Typed + TypePath>(&mut self) -> &mut Self;
}

impl InspectorExt for App {
    fn register_principal<C: Component>(&mut self) -> &mut Self {
        self.world_mut()
            .resource_mut::<PrincipalRegistry>()
            .register::<C>();
        self
    }

    fn with_name(&mut self, name: impl Into<String>) -> &mut Self {
        self.world_mut()
            .resource_mut::<PrincipalRegistry>()
            .set_last_name(name.into());
        self
    }

    fn with_group(&mut self, group: &str) -> &mut Self {
        self.world_mut()
            .resource_mut::<PrincipalRegistry>()
            .set_last_group(group.to_owned());
        self
    }

    fn register_principal_group<P: PrincipalTuple>(&mut self, name: &str) -> &mut Self {
        self.world_mut()
            .resource_mut::<PrincipalRegistry>()
            .register_group::<P>(name);
        self
    }

    fn register_inspector_tab<T: InspectorTab>(&mut self, tab: T) -> &mut Self {
        self.world_mut()
            .resource_mut::<InspectorTabRegistry>()
            .register(tab);
        self
    }

    fn register_inspector_analytics<F>(
        &mut self,
        id: &'static str,
        title: &'static str,
        ui_fn: F,
    ) -> &mut Self
    where
        F: Fn(&mut egui::Ui, &World) + Send + Sync + 'static,
    {
        self.register_inspector_analytics_at(id, title, DockPosition::Bottom, ui_fn)
    }

    fn register_inspector_analytics_at<F>(
        &mut self,
        id: &'static str,
        title: &'static str,
        dock_position: DockPosition,
        ui_fn: F,
    ) -> &mut Self
    where
        F: Fn(&mut egui::Ui, &World) + Send + Sync + 'static,
    {
        self.register_inspector_tab(AnalyticsTab {
            id,
            title,
            ui_fn,
            dock_position,
        })
    }

    fn register_inspector_interactive<F>(
        &mut self,
        id: &'static str,
        title: &'static str,
        ui_fn: F,
    ) -> &mut Self
    where
        F: FnMut(&mut egui::Ui, &mut World) + Send + Sync + 'static,
    {
        self.register_inspector_interactive_at(id, title, DockPosition::Bottom, ui_fn)
    }

    fn register_inspector_interactive_at<F>(
        &mut self,
        id: &'static str,
        title: &'static str,
        dock_position: DockPosition,
        ui_fn: F,
    ) -> &mut Self
    where
        F: FnMut(&mut egui::Ui, &mut World) + Send + Sync + 'static,
    {
        self.register_inspector_tab(InteractiveTab {
            id,
            title,
            ui_fn,
            dock_position,
        })
    }

    fn register_inspector_section<C: Component>(
        &mut self,
        title: &str,
        render_fn: impl Fn(&mut egui::Ui, &mut World, Entity) + Send + Sync + 'static,
    ) -> &mut Self {
        self.world_mut()
            .resource_mut::<InspectorSectionRegistry>()
            .sections
            .push(InspectorSection {
                marker_type_id: TypeId::of::<C>(),
                title: title.to_string(),
                render_fn: Box::new(render_fn),
            });
        self
    }

    fn register_bitmask_enum<E: Typed + TypePath>(&mut self) -> &mut Self {
        // A non-enum type is skipped; `BitmaskRegistry::register_enum` returns the
        // error for callers that want to detect it.
        let _ = self
            .world_mut()
            .resource_mut::<BitmaskRegistry>()
            .register_enum::<E>();
        self
    }
}

/// Wrapper for read-only analytics tabs using closures.
struct AnalyticsTab<F>
where
    F: Fn(&mut egui::Ui, &World) + Send + Sync + 'static,
{
    id: &'static str,
    title: &'static str,
    ui_fn: F,
    dock_position: DockPosition,
}

impl<F> InspectorTab for AnalyticsTab<F>
where
    F: Fn(&mut egui::Ui, &World) + Send + Sync + 'static,
{
    fn id(&self) -> &'static str {
        self.id
    }

    fn title(&self) -> &str {
        self.title
    }

    fn ui(&mut self, ui: &mut egui::Ui, world: &mut World) {
        (self.ui_fn)(ui, world);
    }

    fn dock_position(&self) -> DockPosition {
        self.dock_position
    }
}

/// Wrapper for interactive tabs using closures.
struct InteractiveTab<F>
where
    F: FnMut(&mut egui::Ui, &mut World) + Send + Sync + 'static,
{
    id: &'static str,
    title: &'static str,
    ui_fn: F,
    dock_position: DockPosition,
}

impl<F> InspectorTab for InteractiveTab<F>
where
    F: FnMut(&mut egui::Ui, &mut World) + Send + Sync + 'static,
{
    fn id(&self) -> &'static str {
        self.id
    }

    fn title(&self) -> &str {
        self.title
    }

    fn ui(&mut self, ui: &mut egui::Ui, world: &mut World) {
        (self.ui_fn)(ui, world);
    }

    fn dock_position(&self) -> DockPosition {
        self.dock_position
    }
}

/// Unified tab type that can represent both built-in and custom tabs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Tab {
    /// A built-in tab provided by the inspector framework.
    Builtin(BuiltinTab),
    /// A custom tab registered by the game, identified by index in the registry.
    Custom(usize),
}

impl From<BuiltinTab> for Tab {
    fn from(tab: BuiltinTab) -> Self {
        Tab::Builtin(tab)
    }
}

/// Built-in tabs provided by the inspector framework.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinTab {
    /// The game viewport.
    GameView,
    /// Entity browser sorted by principal components.
    Entities,
    /// Entity hierarchy browser.
    Hierarchy,
    /// Entity/resource/asset inspector.
    Inspector,
    /// Resource browser.
    Resources,
    /// Asset browser.
    Assets,
    /// Performance diagnostics.
    Diagnostics,
    /// GPU adapter info and render diagnostics.
    Gpu,
}

/// Tab viewer for `egui_dock` that handles both built-in and custom tabs.
pub struct TabViewer<'a> {
    pub world: &'a mut World,
    pub selected_entities: &'a mut SelectedEntities,
    pub selection: &'a mut InspectorSelection,
    pub viewport_rect: &'a mut egui::Rect,
    pub hierarchy_search: &'a mut String,
    pub resources_search: &'a mut String,
    pub inspector_search: &'a mut String,
    pub entities_tab_state: &'a mut EntitiesTabState,
    pub custom_tabs: &'a mut [Box<dyn InspectorTab>],
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = Tab;

    fn ui(&mut self, ui: &mut egui::Ui, window: &mut Self::Tab) {
        match window {
            Tab::Builtin(builtin) => {
                let Some(type_registry_res) = self.world.get_resource::<AppTypeRegistry>() else {
                    ui.label("AppTypeRegistry not available");
                    return;
                };
                let type_registry = type_registry_res.0.clone();
                let type_registry = type_registry.read();

                match builtin {
                    BuiltinTab::GameView => {
                        game_view::render(ui, self.viewport_rect);
                    }
                    BuiltinTab::Entities => {
                        entities::render(
                            ui,
                            self.world,
                            self.selected_entities,
                            self.selection,
                            self.entities_tab_state,
                        );
                    }
                    BuiltinTab::Hierarchy => {
                        hierarchy::render(
                            ui,
                            self.world,
                            self.selected_entities,
                            self.selection,
                            self.hierarchy_search,
                        );
                    }
                    BuiltinTab::Inspector => {
                        inspector::render(
                            ui,
                            self.world,
                            &type_registry,
                            self.selected_entities,
                            self.selection,
                            self.inspector_search,
                        );
                    }
                    BuiltinTab::Resources => {
                        resources::render(
                            ui,
                            &type_registry,
                            self.selection,
                            self.resources_search,
                        );
                    }
                    BuiltinTab::Assets => {
                        assets::render(ui, &type_registry, self.world, self.selection);
                    }
                    BuiltinTab::Diagnostics => {
                        diagnostics::render(ui, self.world);
                    }
                    BuiltinTab::Gpu => {
                        gpu::render(ui, self.world);
                    }
                }
            }
            Tab::Custom(index) => {
                if let Some(tab) = self.custom_tabs.get_mut(*index) {
                    // Check visibility before rendering
                    // Note: We reborrow world as shared reference for the visibility check
                    let is_visible = {
                        let world_ref: &World = self.world;
                        tab.is_visible(world_ref)
                    };

                    if is_visible {
                        tab.ui(ui, self.world);
                    } else {
                        ui.centered_and_justified(|ui| {
                            ui.weak("Tab currently hidden");
                        });
                    }
                } else {
                    ui.label(format!("Custom tab {index} not found"));
                }
            }
        }
    }

    fn title(&mut self, window: &mut Self::Tab) -> egui::WidgetText {
        match window {
            Tab::Builtin(builtin) => match builtin {
                BuiltinTab::GameView => "▶ Game".into(),
                BuiltinTab::Entities => "● Entities".into(),
                BuiltinTab::Hierarchy => "△ Hierarchy".into(),
                BuiltinTab::Inspector => "◆ Inspector".into(),
                BuiltinTab::Resources => "■ Resources".into(),
                BuiltinTab::Assets => "◇ Assets".into(),
                BuiltinTab::Diagnostics => "○ Diagnostics".into(),
                BuiltinTab::Gpu => "● GPU".into(),
            },
            Tab::Custom(index) => {
                if let Some(tab) = self.custom_tabs.get(*index) {
                    tab.title().into()
                } else {
                    format!("Tab {index}").into()
                }
            }
        }
    }

    fn clear_background(&self, window: &Self::Tab) -> bool {
        !matches!(window, Tab::Builtin(BuiltinTab::GameView))
    }

    fn closeable(&mut self, _tab: &mut Self::Tab) -> bool {
        false
    }
}
