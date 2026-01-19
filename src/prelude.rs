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
//! - [`egui_pointer_over_area`] - Run condition for blocking game input over panels

pub use crate::{
    InspectorPlugin,
    state::{GameViewportRect, InspectorEnabled, InspectorSelection, UiState},
    tabs::{BuiltinTab, DockPosition, InspectorExt, InspectorTab, InspectorTabRegistry, Tab},
    viewport::{InspectorMainCamera, egui_pointer_over_area},
};
