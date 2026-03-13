//! Performance diagnostics tab.
//!
//! Displays FPS, frame time, entity counts, and user-registered component counters.

use std::collections::VecDeque;

use bevy::prelude::*;
use bevy_egui::egui;

/// Tracks frame times over the last 1 second to compute averages and maxima.
#[derive(Resource)]
pub struct FrameTimeHistory {
    /// Ring buffer of (elapsed_seconds, delta_seconds) pairs.
    entries: VecDeque<(f64, f32)>,
    /// Cached 1-second average FPS.
    pub avg_fps: f32,
    /// Cached 1-second average frame time in ms.
    pub avg_frame_time: f32,
    /// Cached 1-second max FPS (from the shortest frame).
    pub max_fps: f32,
    /// Cached 1-second max frame time in ms (from the longest frame).
    pub max_frame_time: f32,
}

impl Default for FrameTimeHistory {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
            avg_fps: 0.0,
            avg_frame_time: 0.0,
            max_fps: 0.0,
            max_frame_time: 0.0,
        }
    }
}

impl FrameTimeHistory {
    /// Record the current frame and recompute averages.
    fn update(&mut self, elapsed: f64, delta: f32) {
        self.entries.push_back((elapsed, delta));

        // Remove entries older than 1 second
        let cutoff = elapsed - 1.0;
        while let Some(&(t, _)) = self.entries.front() {
            if t < cutoff {
                self.entries.pop_front();
            } else {
                break;
            }
        }

        if self.entries.is_empty() {
            self.avg_fps = 0.0;
            self.avg_frame_time = 0.0;
            self.max_fps = 0.0;
            self.max_frame_time = 0.0;
        } else {
            let count = self.entries.len() as f32;
            let sum_delta: f32 = self.entries.iter().map(|(_, d)| d).sum();
            self.avg_frame_time = (sum_delta / count) * 1000.0;
            self.avg_fps = count / sum_delta;

            let min_delta = self.entries.iter().map(|(_, d)| *d).fold(f32::INFINITY, f32::min);
            let max_delta = self.entries.iter().map(|(_, d)| *d).fold(0.0_f32, f32::max);
            self.max_fps = 1.0 / min_delta;
            self.max_frame_time = max_delta * 1000.0;
        }
    }
}

/// A single counter entry displayed in the diagnostics tab.
struct CounterEntry {
    label: String,
    count_fn: Box<dyn Fn(&World) -> usize + Send + Sync>,
}

/// Holds user-registered counters for the diagnostics tab.
///
/// Use [`InspectorPlugin::with_counter`] or [`InspectorPlugin::with_custom_counter`]
/// to register counters.
///
/// [`InspectorPlugin::with_counter`]: crate::InspectorPlugin::with_counter
/// [`InspectorPlugin::with_custom_counter`]: crate::InspectorPlugin::with_custom_counter
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

/// System that updates the [`FrameTimeHistory`] resource each frame.
pub fn update_frame_time_history(time: Res<Time>, mut history: ResMut<FrameTimeHistory>) {
    history.update(time.elapsed_secs_f64(), time.delta_secs());
}

/// Render the diagnostics tab.
pub fn render(ui: &mut egui::Ui, world: &World) {
    ui.heading("Performance");
    ui.separator();

    // Retrieve 1-second averages and maxima (updated by the frame_time_history_system)
    let (avg_fps, max_fps, avg_frame_time, max_frame_time) = world
        .get_resource::<FrameTimeHistory>()
        .map(|h| (h.avg_fps, h.max_fps, h.avg_frame_time, h.max_frame_time))
        .unwrap_or((0.0, 0.0, 0.0, 0.0));

    // Performance metrics in a grid
    ui.columns(2, |columns| {
        columns[0].label("FPS (Max):");
        columns[1].colored_label(
            if avg_fps >= 60.0 {
                egui::Color32::GREEN
            } else if avg_fps >= 30.0 {
                egui::Color32::YELLOW
            } else {
                egui::Color32::RED
            },
            format!("{avg_fps:.1} Hz ({max_fps:.1} Hz)"),
        );

        columns[0].label("Frame Time (Max):");
        columns[1].colored_label(
            if avg_frame_time <= 16.7 {
                egui::Color32::GREEN
            } else if avg_frame_time <= 33.3 {
                egui::Color32::YELLOW
            } else {
                egui::Color32::RED
            },
            format!("{avg_frame_time:.1} ms ({max_frame_time:.1} ms)"),
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
