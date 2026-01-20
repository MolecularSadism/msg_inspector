//! Integration tests for msg_inspector with Bevy 0.17.

use bevy::prelude::*;
use msg_inspector::prelude::*;

#[test]
fn test_inspector_enabled_resource_defaults_to_true() {
    let enabled = InspectorEnabled::default();
    assert!(enabled.0, "InspectorEnabled should default to true");
}

#[test]
fn test_game_viewport_rect_default_values() {
    let rect = GameViewportRect::default();
    assert_eq!(rect.min_x, 0.0);
    assert_eq!(rect.min_y, 0.0);
    assert_eq!(rect.max_x, f32::MAX);
    assert_eq!(rect.max_y, f32::MAX);
}

#[test]
fn test_game_viewport_rect_contains() {
    let rect = GameViewportRect {
        min_x: 10.0,
        min_y: 20.0,
        max_x: 100.0,
        max_y: 200.0,
    };

    // Inside the rect
    assert!(rect.contains(50.0, 100.0));
    assert!(rect.contains(10.0, 20.0)); // Min edge
    assert!(rect.contains(100.0, 200.0)); // Max edge

    // Outside the rect
    assert!(!rect.contains(5.0, 100.0)); // Left of min_x
    assert!(!rect.contains(50.0, 10.0)); // Above min_y
    assert!(!rect.contains(150.0, 100.0)); // Right of max_x
    assert!(!rect.contains(50.0, 250.0)); // Below max_y
}

#[test]
fn test_crosshair_config_default() {
    let config = CrosshairConfig::default();
    // Just verify it creates without panicking and has a reasonable color
    assert_ne!(config.color, Color::NONE);
}

#[test]
fn test_ui_state_new() {
    let ui_state = UiState::new();
    // Verify default state is created correctly
    assert_eq!(ui_state.selection, InspectorSelection::Entities);
    assert!(ui_state.hierarchy_search.is_empty());
}

#[test]
fn test_inspector_selection_equality() {
    let sel1 = InspectorSelection::Entities;
    let sel2 = InspectorSelection::Entities;
    assert_eq!(sel1, sel2);
}

#[test]
fn test_dock_position_default() {
    let pos = DockPosition::default();
    assert_eq!(pos, DockPosition::Bottom);
}

#[test]
fn test_builtin_tab_enum() {
    // Verify all builtin tabs can be created
    let tabs = [
        BuiltinTab::GameView,
        BuiltinTab::Hierarchy,
        BuiltinTab::Inspector,
        BuiltinTab::Resources,
        BuiltinTab::Assets,
        BuiltinTab::Diagnostics,
    ];

    // Verify equality works
    assert_eq!(BuiltinTab::GameView, BuiltinTab::GameView);
    assert_ne!(BuiltinTab::GameView, BuiltinTab::Hierarchy);

    // Verify Tab enum conversion works
    for tab in tabs {
        let wrapped = Tab::from(tab);
        assert_eq!(wrapped, Tab::Builtin(tab));
    }
}

#[test]
fn test_tab_registry_operations() {
    let registry = InspectorTabRegistry::default();

    // Initially empty
    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);
    assert!(registry.tabs().is_empty());
}

/// Test that demonstrates the custom tab registration API compiles correctly.
/// This is a compile-time verification test.
#[allow(dead_code)]
fn _compile_test_custom_tab_registration() {
    struct TestTab;

    impl InspectorTab for TestTab {
        fn id(&self) -> &'static str {
            "test"
        }

        fn title(&self) -> &str {
            "Test Tab"
        }

        fn ui(&mut self, ui: &mut msg_inspector::egui::Ui, _world: &mut World) {
            ui.label("Test");
        }

        fn dock_position(&self) -> DockPosition {
            DockPosition::Bottom
        }

        fn is_visible(&self, _world: &World) -> bool {
            true
        }
    }

    // This verifies the API surface is correct at compile time
    let mut registry = InspectorTabRegistry::default();
    registry.register(TestTab);
}
