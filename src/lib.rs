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
//! - **Custom counters**: Track component counts in the Diagnostics tab via [`InspectorPlugin::with_counter`]
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
//!     app.add_plugins((DefaultPlugins, InspectorPlugin::default()));
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
//!     app.add_plugins(InspectorPlugin::default());
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
//!     app.add_plugins(InspectorPlugin::default());
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
#[cfg(target_os = "windows")]
mod win32;

use std::sync::Mutex;

use bevy::{prelude::*, render::alpha::AlphaMode};
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::DefaultInspectorConfigPlugin;

pub use panel::{GameWindow, show_ui_system};
pub use picking::{
    CrosshairConfig, handle_picking_clicks, handle_picking_clicks_two_window,
    update_picked_entity_marker,
};
pub use state::{GameViewportRect, InspectorEnabled, InspectorSelection, UiState};
pub use tabs::{
    BuiltinTab, DiagnosticsCounters, DockPosition, FrameTimeHistory, InspectorExt, InspectorTab,
    InspectorTabRegistry, Tab,
};
pub use viewport::{
    InspectorMainCamera, egui_pointer_over_area, route_cameras_to_game_window,
    set_camera_viewport, sync_game_window_position,
};

// Re-export egui so consumers don't need to depend on bevy-inspector-egui directly
pub use bevy_inspector_egui::egui;

/// Controls how the inspector and game are windowed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InspectorMode {
    /// Game renders in a separate borderless window that snaps into the inspector's
    /// GameView tab area. On Windows, the game window is set as "owned" by the
    /// inspector window so it always stays in front without being globally on top.
    #[default]
    TwoWindow,
    /// Legacy mode: game viewport is clipped inside egui dock panels in one window.
    SingleWindow,
}

/// Resource holding the current inspector windowing mode.
#[derive(Resource, Clone, Copy, Debug)]
pub struct InspectorModeRes(pub InspectorMode);

/// Resource holding the entity ID of the game window (in TwoWindow mode).
#[derive(Resource, Clone, Copy, Debug)]
pub struct GameWindowEntity(pub Entity);

/// Main plugin for the inspector framework.
///
/// Provides a dockable panel UI with built-in tabs for entity inspection,
/// hierarchy browsing, resource/asset exploration, and performance diagnostics.
///
/// Toggle visibility with the Delete key.
///
/// # Custom Counters
///
/// Use [`with_counter`](Self::with_counter) and
/// [`with_custom_counter`](Self::with_custom_counter) to register component
/// counters displayed in the Diagnostics tab:
///
/// ```
/// use bevy::prelude::*;
/// use msg_inspector::prelude::*;
///
/// #[derive(Component)]
/// struct Collider;
///
/// #[derive(Component)]
/// struct RigidBody;
///
/// fn plugin(app: &mut App) {
///     app.add_plugins(
///         InspectorPlugin::default()
///             .with_counter::<Collider>()
///             .with_counter::<RigidBody>()
///     );
/// }
/// ```
pub struct InspectorPlugin {
    counters: Mutex<tabs::DiagnosticsCounters>,
    mode: InspectorMode,
}

impl Default for InspectorPlugin {
    fn default() -> Self {
        Self {
            counters: Mutex::new(tabs::DiagnosticsCounters::default()),
            mode: InspectorMode::default(),
        }
    }
}

impl InspectorPlugin {
    /// Register a component counter in the diagnostics tab.
    ///
    /// The counter displays the number of entities that have component `C`.
    /// The label is derived from the type name automatically.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy::prelude::*;
    /// use msg_inspector::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Collider;
    ///
    /// fn plugin(app: &mut App) {
    ///     app.add_plugins(
    ///         InspectorPlugin::default()
    ///             .with_counter::<Collider>()
    ///     );
    /// }
    /// ```
    #[must_use]
    pub fn with_counter<C: Component>(self) -> Self {
        self.counters.lock().unwrap().add_component_counter::<C>();
        self
    }

    /// Register a custom counter with a label and count function in the diagnostics tab.
    ///
    /// # Example
    ///
    /// ```
    /// use bevy::prelude::*;
    /// use msg_inspector::prelude::*;
    ///
    /// fn plugin(app: &mut App) {
    ///     app.add_plugins(
    ///         InspectorPlugin::default()
    ///             .with_custom_counter("Visible", |world| {
    ///                 world.entities().len() as usize
    ///             })
    ///     );
    /// }
    /// ```
    #[must_use]
    pub fn with_custom_counter(
        self,
        label: &str,
        count_fn: impl Fn(&World) -> usize + Send + Sync + 'static,
    ) -> Self {
        self.counters.lock().unwrap().add_custom(label, count_fn);
        self
    }

    /// Set the inspector windowing mode.
    ///
    /// - [`InspectorMode::TwoWindow`] (default): Game renders in a separate borderless
    ///   window that snaps into the inspector's GameView tab area.
    /// - [`InspectorMode::SingleWindow`]: Legacy mode where the game viewport is clipped
    ///   inside the egui dock panels.
    #[must_use]
    pub fn mode(mut self, mode: InspectorMode) -> Self {
        self.mode = mode;
        self
    }
}

impl Plugin for InspectorPlugin {
    fn build(&self, app: &mut App) {
        let mode = self.mode;

        // Core plugins
        app.add_plugins(EguiPlugin::default())
            .add_plugins(DefaultInspectorConfigPlugin);

        // Mode resource
        app.insert_resource(InspectorModeRes(mode));

        // State management
        app.register_type::<InspectorEnabled>()
            .register_type::<picking::PickedEntityMarker>()
            .register_type::<picking::CrosshairConfig>()
            .init_resource::<InspectorEnabled>()
            .init_resource::<GameViewportRect>()
            .init_resource::<InspectorTabRegistry>()
            .init_resource::<picking::CrosshairConfig>()
            .init_resource::<tabs::FrameTimeHistory>();

        // Initialize UiState after tab registry so built-in tabs can be set up
        app.add_systems(Startup, state::initialize_ui_state);

        // Shared systems (both modes)
        app.add_systems(Startup, panel::setup.before(state::initialize_ui_state))
            .add_systems(
                bevy_inspector_egui::bevy_egui::EguiPrimaryContextPass,
                show_ui_system,
            )
            .add_systems(Update, panel::toggle_inspector)
            .add_systems(Update, update_picked_entity_marker)
            .add_systems(Update, tabs::update_frame_time_history);

        // Mode-specific systems
        match mode {
            InspectorMode::TwoWindow => {
                app.add_systems(
                    Startup,
                    panel::spawn_game_window.before(state::initialize_ui_state),
                );
                app.add_systems(
                    PostUpdate,
                    (
                        route_cameras_to_game_window,
                        sync_game_window_position.after(show_ui_system),
                    ),
                );
                app.add_systems(Update, handle_picking_clicks_two_window);

                // Win32: set up owner-window relationship for Z-order
                #[cfg(target_os = "windows")]
                {
                    app.init_resource::<win32::OwnerWindowEstablished>();
                    app.add_systems(Update, win32::setup_owner_window);
                }
            }
            InspectorMode::SingleWindow => {
                app.add_systems(PostUpdate, set_camera_viewport.after(show_ui_system));
                app.add_systems(Update, handle_picking_clicks);
            }
        }

        // Type registrations for reflection
        app.register_type::<Option<Handle<Image>>>()
            .register_type::<AlphaMode>();
    }

    fn finish(&self, app: &mut App) {
        // Insert counters collected during plugin configuration.
        // Done in finish() so it runs after build(), avoiding resource ordering issues.
        let counters = std::mem::take(&mut *self.counters.lock().unwrap());
        app.insert_resource(counters);
    }
}
