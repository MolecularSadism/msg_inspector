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
    /// Cached 1-second min FPS (from the shortest frame).
    pub min_fps: f32,
    /// Cached 1-second max frame time in ms (from the longest frame).
    pub max_frame_time: f32,
}

impl Default for FrameTimeHistory {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
            avg_fps: 0.0,
            avg_frame_time: 0.0,
            min_fps: 0.0,
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
            self.min_fps = 0.0;
            self.max_frame_time = 0.0;
        } else {
            let count = self.entries.len() as f32;
            let sum_delta: f32 = self.entries.iter().map(|(_, d)| d).sum();
            self.avg_frame_time = (sum_delta / count) * 1000.0;
            self.avg_fps = count / sum_delta;

            let max_delta = self.entries.iter().map(|(_, d)| *d).fold(0.0_f32, f32::max);
            self.min_fps = 1.0 / max_delta;
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

/// Render a styled section header with separator.
fn section_header(ui: &mut egui::Ui, text: &str) {
    ui.add_space(2.0);
    ui.label(egui::RichText::new(text).strong().size(14.0));
    ui.separator();
    ui.add_space(2.0);
}

/// Render the diagnostics tab.
pub fn render(ui: &mut egui::Ui, world: &World) {
    section_header(ui, "▶ Performance");

    let (avg_fps, max_fps, avg_frame_time, max_frame_time) = world
        .get_resource::<FrameTimeHistory>()
        .map(|h| (h.avg_fps, h.min_fps, h.avg_frame_time, h.max_frame_time))
        .unwrap_or((0.0, 0.0, 0.0, 0.0));

    let fps_color = if avg_fps >= 60.0 {
        egui::Color32::GREEN
    } else if avg_fps >= 30.0 {
        egui::Color32::YELLOW
    } else {
        egui::Color32::RED
    };

    let frame_color = if avg_frame_time <= 16.7 {
        egui::Color32::GREEN
    } else if avg_frame_time <= 33.3 {
        egui::Color32::YELLOW
    } else {
        egui::Color32::RED
    };

    egui::Grid::new("perf_grid")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            ui.label(egui::RichText::new("FPS (Min):").weak());
            ui.label(
                egui::RichText::new(format!("{avg_fps:.1} Hz ({max_fps:.1} Hz)"))
                    .color(fps_color)
                    .strong(),
            );
            ui.end_row();

            ui.label(egui::RichText::new("Frame Time (Max):").weak());
            ui.label(
                egui::RichText::new(format!("{avg_frame_time:.1} ms ({max_frame_time:.1} ms)"))
                    .color(frame_color)
                    .strong(),
            );
            ui.end_row();
        });

    ui.add_space(8.0);
    section_header(ui, "● Entities");

    let total_entities = world.entities().len();

    egui::Grid::new("entity_grid")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            ui.label(egui::RichText::new("Total Entities:").weak());
            ui.label(egui::RichText::new(format!("{total_entities}")).strong());
            ui.end_row();
        });

    // User-registered counters
    if let Some(counters) = world.get_resource::<DiagnosticsCounters>()
        && !counters.counters.is_empty()
    {
        ui.add_space(8.0);
        section_header(ui, "■ Counters");

        egui::Grid::new("counter_grid")
            .num_columns(2)
            .spacing([12.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                for entry in &counters.counters {
                    let count = (entry.count_fn)(world);
                    ui.label(egui::RichText::new(format!("{}:", entry.label)).weak());
                    ui.label(egui::RichText::new(format!("{count}")).strong());
                    ui.end_row();
                }
            });
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
