//! Tab definitions and registration for the inspector.

mod assets;
mod diagnostics;
mod game_view;
mod hierarchy;
mod inspector;
mod resources;

use bevy::prelude::*;
use bevy_egui::egui;
use bevy_inspector_egui::bevy_inspector::hierarchy::SelectedEntities;

use crate::state::InspectorSelection;

/// Trait for registering custom dev panel tabs.
///
/// Implement this trait to create custom tabs with full control over
/// rendering and state management.
///
/// # Example
///
/// ```ignore
/// use bevy::prelude::*;
/// use msg_inspector::prelude::*;
///
/// /// A custom tab that displays player statistics.
/// struct PlayerStatsTab {
///     title: String,
/// }
///
/// impl InspectorTab for PlayerStatsTab {
///     fn id(&self) -> &'static str {
///         "player_stats"
///     }
///
///     fn title(&self) -> &str {
///         &self.title
///     }
///
///     fn ui(&mut self, ui: &mut egui::Ui, world: &mut World) {
///         ui.heading("Player Statistics");
///
///         // Query world for player data
///         let mut query = world.query::<&Transform>();
///         ui.label(format!("Entities with Transform: {}", query.iter(world).count()));
///     }
///
///     fn dock_position(&self) -> DockPosition {
///         DockPosition::Right
///     }
///
///     fn is_visible(&self, world: &World) -> bool {
///         // Only show when there are entities in the world
///         world.entities().len() > 0
///     }
/// }
///
/// // Register the custom tab
/// fn setup_inspector(app: &mut App) {
///     app.register_inspector_tab(PlayerStatsTab {
///         title: "Player Stats".to_string(),
///     });
/// }
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
    pub fn tabs(&self) -> &[Box<dyn InspectorTab>] {
        &self.tabs
    }

    /// Get mutable access to all registered tabs.
    pub fn tabs_mut(&mut self) -> &mut [Box<dyn InspectorTab>] {
        &mut self.tabs
    }

    /// Number of registered custom tabs.
    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    /// Whether there are any registered custom tabs.
    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }
}

/// Extension trait for App to register inspector tabs.
pub trait InspectorExt {
    /// Register a custom tab with full InspectorTab implementation.
    fn register_inspector_tab<T: InspectorTab>(&mut self, tab: T) -> &mut Self;

    /// Register a read-only analytics tab (no world mutation).
    ///
    /// The tab will be placed in the Bottom dock by default.
    /// Use [`register_inspector_analytics_at`] to specify a custom dock position.
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
    /// ```ignore
    /// app.register_inspector_analytics_at(
    ///     "stats",
    ///     "Statistics",
    ///     DockPosition::Right,
    ///     |ui, world| {
    ///         ui.label("Read-only stats here");
    ///     },
    /// );
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
    /// Use [`register_inspector_interactive_at`] to specify a custom dock position.
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
    /// ```ignore
    /// app.register_inspector_interactive_at(
    ///     "cheats",
    ///     "Cheats",
    ///     DockPosition::Left,
    ///     |ui, world| {
    ///         if ui.button("Heal Player").clicked() {
    ///             // Mutate world state
    ///         }
    ///     },
    /// );
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
}

impl InspectorExt for App {
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
}

/// Tab viewer for egui_dock that handles both built-in and custom tabs.
pub struct TabViewer<'a> {
    pub world: &'a mut World,
    pub selected_entities: &'a mut SelectedEntities,
    pub selection: &'a mut InspectorSelection,
    pub viewport_rect: &'a mut egui::Rect,
    pub hierarchy_search: &'a mut String,
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
                        );
                    }
                    BuiltinTab::Resources => {
                        resources::render(ui, &type_registry, self.selection);
                    }
                    BuiltinTab::Assets => {
                        assets::render(ui, &type_registry, self.world, self.selection);
                    }
                    BuiltinTab::Diagnostics => {
                        diagnostics::render(ui, self.world);
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
                    ui.label(format!("Custom tab {} not found", index));
                }
            }
        }
    }

    fn title(&mut self, window: &mut Self::Tab) -> egui::WidgetText {
        match window {
            Tab::Builtin(builtin) => match builtin {
                BuiltinTab::GameView => "Game".into(),
                BuiltinTab::Hierarchy => "Hierarchy".into(),
                BuiltinTab::Inspector => "Inspector".into(),
                BuiltinTab::Resources => "Resources".into(),
                BuiltinTab::Assets => "Assets".into(),
                BuiltinTab::Diagnostics => "Diagnostics".into(),
            },
            Tab::Custom(index) => {
                if let Some(tab) = self.custom_tabs.get(*index) {
                    tab.title().into()
                } else {
                    format!("Tab {}", index).into()
                }
            }
        }
    }

    fn clear_background(&self, window: &Self::Tab) -> bool {
        !matches!(window, Tab::Builtin(BuiltinTab::GameView))
    }
}
