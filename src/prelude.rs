//! Common re-exports for convenient usage.
//!
//! ```ignore
//! use msg_inspector::prelude::*;
//! ```
//!
//! This prelude includes:
//! - [`InspectorPlugin`] - The main plugin to add to your app
//! - [`InspectorMainCamera`] - Marker component for viewport management
//! - [`InspectorExt`] - Extension trait for registering custom tabs
//! - [`InspectorTab`] - Trait for implementing custom tabs
//! - [`CrosshairConfig`] - Configuration for entity selection crosshair
//! - [`egui_pointer_over_area`] - Run condition for blocking game input over panels

pub use crate::{
    picking::CrosshairConfig,
    state::{GameViewportRect, InspectorEnabled, InspectorSelection, UiState},
    tabs::{BuiltinTab, DockPosition, InspectorExt, InspectorTab, InspectorTabRegistry, Tab},
    viewport::{InspectorMainCamera, egui_pointer_over_area},
    InspectorPlugin,
};
