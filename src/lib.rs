//! # `msg_inspector`
//!
//! A generic, modular Bevy UI framework for development panels and inspectors.
//!
//! This crate provides a registration-based API where game modules can locally
//! declare dev features, supporting pluggable analytics (read-only) and
//! interactive (mutable) tabs.
//!
//! ## Features
//!
//! - **Built-in tabs**: `GameView`, Hierarchy, Inspector, Resources, Assets, Diagnostics
//! - **Entity picking**: Click entities in the viewport to select them
//! - **Viewport management**: Automatic camera viewport clipping to dock area
//! - **Tab registration**: Games can register custom tabs via [`InspectorExt`] trait
//!
//! ## Built-in Tabs
//!
//! | Tab | Description |
//! |-----|-------------|
//! | Game | The game viewport, clipped to not overlap with panels |
//! | Hierarchy | Entity tree browser with search filtering |
//! | Inspector | Entity component inspector using reflection |
//! | Resources | Browse all registered resources |
//! | Assets | Browse all loaded asset handles |
//! | Diagnostics | FPS, frame time, and entity count metrics |
//!
//! ## Quick Start
//!
//! ```
//! use bevy::prelude::*;
//! use msg_inspector::prelude::*;
//!
//! fn setup(mut commands: Commands) {
//!     // Mark your main camera for viewport management
//!     commands.spawn((Camera2d, InspectorMainCamera));
//! }
//! 
//! fn plugin(app: &mut App) {
//!     app.add_plugins((DefaultPlugins, InspectorPlugin));
//!     app.register_inspector_interactive("cheats", "Cheats", |ui, world| {
//!         if ui.button("Heal Player").clicked() {
//!             // Mutate world state
//!         }
//!     })
//!     .add_systems(Startup, setup);
//! }
//! ```
//!
//! ## Registering Custom Tabs
//!
//! ### Analytics Tab (Read-Only)
//!
//! Use for displaying stats without modifying game state:
//!
//! ```
//! use bevy::prelude::*;
//! use msg_inspector::prelude::*;
//!
//! fn plugin(app: &mut App) {
//!     app.add_plugins(InspectorPlugin);
//!     app.register_inspector_analytics("stats", "Statistics", |ui, world| {
//!         let count = world.entities().len();
//!         ui.label(format!("Entities: {count}"));
//!     });
//! }
//! ```
//!
//! ### Interactive Tab (Mutable)
//!
//! Use when you need to trigger events or modify state:
//!
//! ```
//! use bevy::prelude::*;
//! use msg_inspector::prelude::*;
//!
//! fn plugin(app: &mut App) {
//!     app.add_plugins(InspectorPlugin);
//!     app.register_inspector_interactive("spawner", "Spawner", |ui, world| {
//!         if ui.button("Spawn Entity").clicked() {
//!             world.commands().spawn_empty();
//!         }
//!     });
//! }
//! ```
//!
//! ## Blocking Game Input Over Panels
//!
//! Use [`egui_pointer_over_area`] to prevent game clicks when the cursor is over panels:
//!
//! ```
//! use bevy::prelude::*;
//! use msg_inspector::prelude::*;
//!
//! fn my_click_system() {}
//!
//! let mut app = App::new();
//! app.add_systems(Update, my_click_system.run_if(not(egui_pointer_over_area)));
//! ```
//!
//! ## Toggle Visibility
//!
//! Press the **Delete** key to toggle the inspector panel visibility.

mod panel;
mod picking;
pub mod prelude;
mod state;
pub mod tabs;
mod viewport;

use bevy::{prelude::*, render::alpha::AlphaMode};
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::DefaultInspectorConfigPlugin;

pub use panel::show_ui_system;
pub use picking::{CrosshairConfig, handle_picking_clicks, update_picked_entity_marker};
pub use state::{GameViewportRect, InspectorEnabled, InspectorSelection, UiState};
pub use tabs::{BuiltinTab, DockPosition, InspectorExt, InspectorTab, InspectorTabRegistry, Tab};
pub use viewport::{InspectorMainCamera, egui_pointer_over_area, set_camera_viewport};

// Re-export egui so consumers don't need to depend on bevy-inspector-egui directly
pub use bevy_inspector_egui::egui;

/// Main plugin for the inspector framework.
///
/// Provides a dockable panel UI with built-in tabs for entity inspection,
/// hierarchy browsing, resource/asset exploration, and performance diagnostics.
///
/// Toggle visibility with the Delete key.
pub struct InspectorPlugin;

impl Plugin for InspectorPlugin {
    fn build(&self, app: &mut App) {
        // Core plugins
        app.add_plugins(EguiPlugin::default())
            .add_plugins(DefaultInspectorConfigPlugin);

        // State management
        app.register_type::<InspectorEnabled>()
            .register_type::<picking::PickedEntityMarker>()
            .register_type::<picking::CrosshairConfig>()
            .init_resource::<InspectorEnabled>()
            .init_resource::<GameViewportRect>()
            .init_resource::<InspectorTabRegistry>()
            .init_resource::<picking::CrosshairConfig>();

        // Initialize UiState after tab registry so built-in tabs can be set up
        app.add_systems(Startup, state::initialize_ui_state);

        // Core systems
        app.add_systems(Startup, panel::setup.before(state::initialize_ui_state))
            .add_systems(
                bevy_inspector_egui::bevy_egui::EguiPrimaryContextPass,
                show_ui_system,
            )
            .add_systems(PostUpdate, set_camera_viewport.after(show_ui_system))
            .add_systems(Update, panel::toggle_inspector)
            .add_systems(Update, handle_picking_clicks)
            .add_systems(Update, update_picked_entity_marker);

        // Type registrations for reflection
        app.register_type::<Option<Handle<Image>>>()
            .register_type::<AlphaMode>();
    }
}
