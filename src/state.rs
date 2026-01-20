//! Core state types for the inspector UI.

use std::any::TypeId;

use bevy::{asset::UntypedAssetId, prelude::*};
use bevy_egui::egui;
use bevy_inspector_egui::bevy_inspector::hierarchy::SelectedEntities;
use egui_dock::{DockState, NodeIndex, Style};

use crate::tabs::{BuiltinTab, InspectorTab, InspectorTabRegistry, Tab};

/// Resource controlling whether the inspector panel is visible.
///
/// Toggle with the Delete key.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct InspectorEnabled(pub bool);

impl Default for InspectorEnabled {
    fn default() -> Self {
        Self(true)
    }
}

/// Stores the game viewport rectangle in screen/egui coordinates.
///
/// Used to determine if the mouse is over the game area vs egui panels.
#[derive(Resource, Clone, Copy, Debug)]
pub struct GameViewportRect {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl Default for GameViewportRect {
    fn default() -> Self {
        Self {
            min_x: 0.0,
            min_y: 0.0,
            max_x: f32::MAX,
            max_y: f32::MAX,
        }
    }
}

impl GameViewportRect {
    /// Check if a point (x, y) is inside this rectangle.
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }
}

/// What is currently selected for inspection.
#[derive(Debug, Eq, PartialEq)]
pub enum InspectorSelection {
    /// One or more entities selected.
    Entities,
    /// A resource selected by type id and name.
    Resource(TypeId, String),
    /// An asset selected by type id, name, and handle.
    Asset(TypeId, String, UntypedAssetId),
}

/// Core UI state for the inspector dock.
#[derive(Resource)]
pub struct UiState {
    /// The dock state managing tab layout.
    pub state: DockState<Tab>,
    /// Current viewport rectangle for the game view.
    pub viewport_rect: egui::Rect,
    /// Currently selected entities.
    pub selected_entities: SelectedEntities,
    /// What is currently selected for inspection.
    pub selection: InspectorSelection,
    /// Search filter for hierarchy tab.
    pub hierarchy_search: String,
    /// Custom tabs extracted from the registry for rendering.
    custom_tabs: Vec<Box<dyn InspectorTab>>,
}

impl UiState {
    /// Create a new UiState with default dock layout.
    pub fn new() -> Self {
        Self {
            state: DockState::new(vec![Tab::Builtin(BuiltinTab::GameView)]),
            selected_entities: SelectedEntities::default(),
            selection: InspectorSelection::Entities,
            viewport_rect: egui::Rect::NOTHING,
            hierarchy_search: String::new(),
            custom_tabs: Vec::new(),
        }
    }

    /// Build the default dock layout with built-in tabs and any custom tabs.
    pub fn build_default_layout(&mut self, num_custom_tabs: usize) {
        let mut state = DockState::new(vec![Tab::Builtin(BuiltinTab::GameView)]);
        let tree = state.main_surface_mut();

        // Right panel: Inspector
        let [main, _] = tree.split_right(
            NodeIndex::root(),
            0.8,
            vec![Tab::Builtin(BuiltinTab::Inspector)],
        );

        // Left panel: Diagnostics at top
        let [main, left_panel] =
            tree.split_left(main, 0.2, vec![Tab::Builtin(BuiltinTab::Diagnostics)]);

        // Hierarchy and Resources below diagnostics (as tabs in the same panel)
        tree.split_below(
            left_panel,
            0.2,
            vec![
                Tab::Builtin(BuiltinTab::Hierarchy),
                Tab::Builtin(BuiltinTab::Resources),
            ],
        );

        // Bottom panel: Assets and custom tabs
        let mut bottom_tabs: Vec<Tab> = vec![Tab::Builtin(BuiltinTab::Assets)];

        // Add custom tabs to the bottom panel
        for i in 0..num_custom_tabs {
            bottom_tabs.push(Tab::Custom(i));
        }

        tree.split_below(main, 0.8, bottom_tabs);

        self.state = state;
    }

    /// Set the custom tabs from the registry.
    pub fn set_custom_tabs(&mut self, tabs: Vec<Box<dyn InspectorTab>>) {
        self.custom_tabs = tabs;
    }

    /// Render the dock UI.
    pub fn ui(&mut self, world: &mut World, ctx: &mut egui::Context) {
        let mut tab_viewer = crate::tabs::TabViewer {
            world,
            viewport_rect: &mut self.viewport_rect,
            selected_entities: &mut self.selected_entities,
            selection: &mut self.selection,
            hierarchy_search: &mut self.hierarchy_search,
            custom_tabs: &mut self.custom_tabs,
        };
        egui_dock::DockArea::new(&mut self.state)
            .style(Style::from_egui(ctx.style().as_ref()))
            .show(ctx, &mut tab_viewer);
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}

/// System to initialize UiState after InspectorTabRegistry is populated.
pub fn initialize_ui_state(mut commands: Commands, mut registry: ResMut<InspectorTabRegistry>) {
    // Take ownership of the custom tabs from the registry
    let custom_tabs: Vec<Box<dyn InspectorTab>> = std::mem::take(&mut registry.tabs);
    let num_custom_tabs = custom_tabs.len();

    let mut ui_state = UiState::new();

    // Build default layout with built-in tabs and custom tabs
    ui_state.build_default_layout(num_custom_tabs);

    // Store the custom tabs in UiState for rendering
    ui_state.set_custom_tabs(custom_tabs);

    commands.insert_resource(ui_state);
}
