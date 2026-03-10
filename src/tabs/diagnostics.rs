//! Performance diagnostics tab.
//!
//! Displays FPS, frame time, entity counts, and user-registered component counters.

use bevy::prelude::*;
use bevy_egui::egui;

/// A single counter entry displayed in the diagnostics tab.
struct CounterEntry {
    label: String,
    count_fn: Box<dyn Fn(&World) -> usize + Send + Sync>,
}

/// Resource holding user-registered counters for the diagnostics tab.
///
/// Use [`InspectorExt::with_counter`] or [`InspectorExt::with_custom_counter`]
/// to register counters.
///
/// [`InspectorExt::with_counter`]: crate::tabs::InspectorExt::with_counter
/// [`InspectorExt::with_custom_counter`]: crate::tabs::InspectorExt::with_custom_counter
#[derive(Resource, Default)]
pub struct DiagnosticsCounters {
    counters: Vec<CounterEntry>,
}

impl DiagnosticsCounters {
    /// Register a counter that counts entities matching a component type.
    ///
    /// The label is derived from the type name (short form).
    pub fn add_component_counter<C: Component>(&mut self) {
        let full_name = std::any::type_name::<C>();
        let label = short_type_name(full_name);

        self.counters.push(CounterEntry {
            label,
            count_fn: Box::new(|world: &World| {
                let Some(id) = world.components().get_id(std::any::TypeId::of::<C>()) else {
                    return 0;
                };
                world
                    .archetypes()
                    .iter()
                    .filter(|arch| arch.contains(id))
                    .map(|arch| arch.len() as usize)
                    .sum()
            }),
        });
    }

    /// Register a counter with a custom label and count function.
    pub fn add_custom(&mut self, label: impl Into<String>, count_fn: impl Fn(&World) -> usize + Send + Sync + 'static) {
        self.counters.push(CounterEntry {
            label: label.into(),
            count_fn: Box::new(count_fn),
        });
    }
}

/// Extract the short type name from a fully-qualified path.
///
/// e.g. `my_game::components::Collider` → `Collider`
fn short_type_name(full: &str) -> String {
    // Handle generics: take the last segment before any `<`
    let base = full.split('<').next().unwrap_or(full);
    base.rsplit("::").next().unwrap_or(base).to_string()
}

/// Render the diagnostics tab.
pub fn render(ui: &mut egui::Ui, world: &World) {
    ui.heading("Performance");
    ui.separator();

    let Some(time) = world.get_resource::<Time>() else {
        ui.label("Time resource not available");
        return;
    };
    let fps = 1.0 / time.delta_secs();
    let frame_time = time.delta_secs() * 1000.0;

    // Performance metrics in a grid
    ui.columns(2, |columns| {
        columns[0].label("FPS:");
        columns[1].colored_label(
            if fps >= 60.0 {
                egui::Color32::GREEN
            } else if fps >= 30.0 {
                egui::Color32::YELLOW
            } else {
                egui::Color32::RED
            },
            format!("{fps:.1} Hz"),
        );

        columns[0].label("Frame Time:");
        columns[1].colored_label(
            if frame_time <= 16.7 {
                egui::Color32::GREEN
            } else if frame_time <= 33.3 {
                egui::Color32::YELLOW
            } else {
                egui::Color32::RED
            },
            format!("{frame_time:.1} ms"),
        );
    });

    ui.add_space(10.0);
    ui.heading("Entities");
    ui.separator();

    // Total entity count (always shown)
    let total_entities = world.entities().len();

    ui.columns(2, |columns| {
        columns[0].label("Total Entities:");
        columns[1].label(format!("{total_entities}"));
    });

    // User-registered counters
    if let Some(counters) = world.get_resource::<DiagnosticsCounters>() {
        if !counters.counters.is_empty() {
            ui.columns(2, |columns| {
                for entry in &counters.counters {
                    let count = (entry.count_fn)(world);
                    columns[0].label(format!("{}:", entry.label));
                    columns[1].label(format!("{count}"));
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_type_name_simple() {
        assert_eq!(short_type_name("my_game::Collider"), "Collider");
    }

    #[test]
    fn short_type_name_nested() {
        assert_eq!(
            short_type_name("my_game::components::physics::Collider"),
            "Collider"
        );
    }

    #[test]
    fn short_type_name_no_path() {
        assert_eq!(short_type_name("Collider"), "Collider");
    }

    #[test]
    fn short_type_name_generic() {
        assert_eq!(
            short_type_name("my_game::Handle<my_game::Mesh>"),
            "Handle"
        );
    }

    #[test]
    fn diagnostics_counters_default_empty() {
        let counters = DiagnosticsCounters::default();
        assert!(counters.counters.is_empty());
    }

    #[test]
    fn diagnostics_counters_add_custom() {
        let mut counters = DiagnosticsCounters::default();
        counters.add_custom("Test", |_| 42);
        assert_eq!(counters.counters.len(), 1);
        assert_eq!(counters.counters[0].label, "Test");
    }
}
