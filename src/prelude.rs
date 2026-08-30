//! Common re-exports for convenient usage.
//!
//! ```
//! use msg_inspector::prelude::*;
//! ```
//!
//! This prelude includes:
//! - [`InspectorPlugin`] - The main plugin to add to your app
//! - [`InspectorMainCamera`] - Marker component for viewport management
//! - [`InspectorExt`] - Extension trait for registering custom tabs
//! - [`InspectorTab`] - Trait for implementing custom tabs
//! - [`CrosshairConfig`] - Configuration for entity selection crosshair
//! - [`PickingIgnore`] - Marker excluding an entity from viewport picking
//! - [`collect_relationships`] - Relationship components and their entities for an entity
//! - [`BitmaskRegistry`] - Named bitmask layers registered from reflected enums
//! - [`bitmask_field_layers`] - Wide bitmask editor with a checkbox per layer
//! - [`egui_pointer_over_area`] - Run condition for blocking game input over panels

pub use crate::{
    BitmaskRegistry, InspectorPlugin,
    picking::{CrosshairConfig, PickingIgnore},
    state::{GameViewportRect, InspectorEnabled, InspectorSelection, UiState},
    tabs::{
        ActiveCategory, BuiltinTab, DiagnosticsCounters, DockPosition, InspectorExt,
        InspectorSectionRegistry, InspectorTab, InspectorTabRegistry, PrincipalRegistry,
        PrincipalTuple, RelationshipEntry, RelationshipKind, Tab, collect_relationships,
    },
    viewport::{InspectorMainCamera, egui_pointer_over_area},
    widgets::{
        BitmaskLayers, Card, CardAction, bitmask_field, bitmask_field_layers, bitmask_field_with,
        draw_cards, draw_cards_with_salt,
    },
};
