//! GPU diagnostics tab.
//!
//! Displays GPU adapter information and per-pass render timings sourced from
//! Bevy's [`RenderDiagnosticsPlugin`](bevy::render::diagnostic::RenderDiagnosticsPlugin)
//! and [`MeshAllocatorDiagnosticPlugin`](bevy::render::diagnostic::MeshAllocatorDiagnosticPlugin).
//!
//! Important characteristics (top-level GPU/CPU frame time, primitives) are
//! shown first. Verbose pipeline statistics and per-asset breakdowns are
//! hidden by default; enable the `verbose-gpu-diagnostics` cargo feature to
//! show every diagnostic the render world records.

use bevy::diagnostic::DiagnosticsStore;
use bevy::prelude::*;
use bevy::render::renderer::RenderAdapterInfo;
use bevy_egui::egui;

/// Render a styled section header with separator.
fn section_header(ui: &mut egui::Ui, text: &str) {
    ui.add_space(2.0);
    ui.label(egui::RichText::new(text).strong().size(14.0));
    ui.separator();
    ui.add_space(2.0);
}

/// Render the GPU tab.
pub fn render(ui: &mut egui::Ui, world: &World) {
    section_header(ui, "● Adapter");
    render_adapter(ui, world);

    ui.add_space(8.0);
    section_header(ui, "▶ Render Passes");
    render_diagnostics(ui, world);
}

fn render_adapter(ui: &mut egui::Ui, world: &World) {
    egui::Grid::new("gpu_adapter_grid")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            let Some(info) = world.get_resource::<RenderAdapterInfo>() else {
                ui.label(egui::RichText::new("Adapter:").weak());
                ui.label(egui::RichText::new("unavailable").italics());
                ui.end_row();
                return;
            };

            ui.label(egui::RichText::new("Adapter:").weak());
            ui.label(egui::RichText::new(&info.0.name).strong());
            ui.end_row();

            ui.label(egui::RichText::new("Backend:").weak());
            ui.label(egui::RichText::new(format!("{:?}", info.0.backend)).strong());
            ui.end_row();

            ui.label(egui::RichText::new("Device Type:").weak());
            ui.label(egui::RichText::new(format!("{:?}", info.0.device_type)).strong());
            ui.end_row();

            if !info.0.driver.is_empty() {
                ui.label(egui::RichText::new("Driver:").weak());
                let driver = if info.0.driver_info.is_empty() {
                    info.0.driver.clone()
                } else {
                    format!("{} ({})", info.0.driver, info.0.driver_info)
                };
                ui.label(egui::RichText::new(driver).strong());
                ui.end_row();
            }
        });
}

fn render_diagnostics(ui: &mut egui::Ui, world: &World) {
    let Some(store) = world.get_resource::<DiagnosticsStore>() else {
        ui.label(
            egui::RichText::new("DiagnosticsStore unavailable")
                .italics()
                .weak(),
        );
        return;
    };

    let mut entries: Vec<Entry> = store
        .iter()
        .filter_map(|d| {
            let path = d.path().as_str();
            classify(path).map(|kind| Entry {
                kind,
                path: path.to_string(),
                suffix: d.suffix.trim().to_string(),
                value: d.smoothed().or_else(|| d.value()).unwrap_or(f64::NAN),
            })
        })
        .filter(|e| !e.value.is_nan())
        .collect();

    if entries.is_empty() {
        ui.label(
            egui::RichText::new(
                "No render diagnostics recorded yet. Bevy's RenderDiagnosticsPlugin records \
                 measurements once frames are presented; per-pass GPU timings and pipeline \
                 statistics additionally require a Vulkan or DX12 backend.",
            )
            .italics()
            .weak(),
        );
        return;
    }

    // Important first, then by path within each kind.
    entries.sort_by(|a, b| {
        a.kind
            .order()
            .cmp(&b.kind.order())
            .then_with(|| a.path.cmp(&b.path))
    });

    egui::Grid::new("gpu_diagnostics_grid")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            for entry in &entries {
                ui.label(egui::RichText::new(format!("{}:", entry.label())).weak());
                ui.label(
                    egui::RichText::new(format_value(entry.value, &entry.suffix)).strong(),
                );
                ui.end_row();
            }
        });
}

struct Entry {
    kind: Kind,
    path: String,
    suffix: String,
    value: f64,
}

impl Entry {
    /// Friendlier label: drop the `render/` prefix; show the trailing field.
    fn label(&self) -> String {
        self.path
            .strip_prefix("render/")
            .map(str::to_string)
            .unwrap_or_else(|| self.path.clone())
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Kind {
    /// Total or top-level CPU/GPU elapsed time per frame.
    FrameElapsed,
    /// Per-pass CPU/GPU elapsed time.
    PassElapsed,
    /// Primitives that survived clipping (closest thing to a draw-cost signal).
    Primitives,
    /// Mesh allocator counters: slabs, allocations, byte sizes.
    MeshAllocator,
    /// Pipeline statistics: shader invocations, clipper invocations, etc.
    PipelineStats,
    /// Per-asset diagnostics from `RenderAssetDiagnosticPlugin`.
    AssetCounters,
}

impl Kind {
    fn order(self) -> u8 {
        match self {
            Kind::FrameElapsed => 0,
            Kind::PassElapsed => 1,
            Kind::Primitives => 2,
            Kind::MeshAllocator => 3,
            Kind::PipelineStats => 4,
            Kind::AssetCounters => 5,
        }
    }
}

fn classify(path: &str) -> Option<Kind> {
    if path.starts_with("mesh_allocator") {
        return Some(Kind::MeshAllocator);
    }
    if path.starts_with("render_asset/") || path.starts_with("erased_render_asset/") {
        return cfg_verbose().then_some(Kind::AssetCounters);
    }
    let rest = path.strip_prefix("render/")?;
    let field = rest.rsplit('/').next().unwrap_or(rest);
    let depth = rest.split('/').count();

    match field {
        "elapsed_cpu" | "elapsed_gpu" => Some(if depth <= 1 {
            Kind::FrameElapsed
        } else {
            Kind::PassElapsed
        }),
        "clipper_primitives_out" => Some(Kind::Primitives),
        "vertex_shader_invocations"
        | "clipper_invocations"
        | "fragment_shader_invocations"
        | "compute_shader_invocations" => cfg_verbose().then_some(Kind::PipelineStats),
        _ => None,
    }
}

#[inline]
fn cfg_verbose() -> bool {
    cfg!(feature = "verbose-gpu-diagnostics")
}

fn format_value(value: f64, suffix: &str) -> String {
    let suffix_part = if suffix.is_empty() {
        String::new()
    } else {
        format!(" {suffix}")
    };
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}{suffix_part}", value as i64)
    } else {
        format!("{value:.3}{suffix_part}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_top_level_elapsed() {
        assert_eq!(classify("render/elapsed_cpu"), Some(Kind::FrameElapsed));
        assert_eq!(classify("render/elapsed_gpu"), Some(Kind::FrameElapsed));
    }

    #[test]
    fn classify_per_pass_elapsed() {
        assert_eq!(
            classify("render/main_opaque_pass_3d/elapsed_cpu"),
            Some(Kind::PassElapsed)
        );
    }

    #[test]
    fn classify_primitives() {
        assert_eq!(
            classify("render/main_opaque_pass_3d/clipper_primitives_out"),
            Some(Kind::Primitives)
        );
    }

    #[test]
    fn classify_mesh_allocator() {
        assert_eq!(
            classify("mesh_allocator_slabs"),
            Some(Kind::MeshAllocator)
        );
    }

    #[test]
    fn classify_unknown_path_is_ignored() {
        assert_eq!(classify("fps"), None);
        assert_eq!(classify("entity_count"), None);
    }

    #[cfg(not(feature = "verbose-gpu-diagnostics"))]
    #[test]
    fn pipeline_stats_hidden_without_verbose() {
        assert_eq!(
            classify("render/main_opaque_pass_3d/vertex_shader_invocations"),
            None
        );
    }

    #[cfg(feature = "verbose-gpu-diagnostics")]
    #[test]
    fn pipeline_stats_visible_with_verbose() {
        assert_eq!(
            classify("render/main_opaque_pass_3d/vertex_shader_invocations"),
            Some(Kind::PipelineStats)
        );
    }

    #[test]
    fn ordering_puts_frame_first() {
        let mut ks = vec![
            Kind::AssetCounters,
            Kind::PipelineStats,
            Kind::PassElapsed,
            Kind::FrameElapsed,
            Kind::Primitives,
            Kind::MeshAllocator,
        ];
        ks.sort_by_key(|k| k.order());
        assert_eq!(ks[0], Kind::FrameElapsed);
        assert_eq!(ks[1], Kind::PassElapsed);
        assert_eq!(ks[2], Kind::Primitives);
    }
}
