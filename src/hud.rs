//! In-game HUD overlay with wand slot visualization.
//!
//! Renders a row of wand slots in the top-left corner of the screen, directly
//! below the matter row. Each slot displays a sprite from the `ValueSlots`
//! atlas — `NotchEmpty` for unselected slots and `NotchFull` for the selected
//! slot.
//!
//! # Setup
//!
//! 1. Add [`HudPlugin`] to your app.
//! 2. Register your `ValueSlots` atlas image with egui and insert a
//!    [`WandSlotAtlas`] resource describing the UV rectangles for each slice.
//! 3. Update [`WandInventory`] each frame to control slot count and selection.
//!
//! # Example
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_egui::EguiContexts;
//! use msg_inspector::hud::{HudPlugin, WandInventory, WandSlotAtlas};
//! use msg_inspector::egui;
//!
//! fn setup_hud(
//!     mut commands: Commands,
//!     mut contexts: EguiContexts,
//!     asset_server: Res<AssetServer>,
//! ) {
//!     let texture_id = contexts.add_image(asset_server.load("ValueSlots.png"));
//!
//!     // NotchEmpty is the left half of the atlas, NotchFull is the right half.
//!     commands.insert_resource(WandSlotAtlas {
//!         texture_id,
//!         notch_empty_uv: egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(0.5, 1.0)),
//!         notch_full_uv:  egui::Rect::from_min_max(egui::pos2(0.5, 0.0), egui::pos2(1.0, 1.0)),
//!         slot_size: egui::vec2(16.0, 16.0),
//!     });
//! }
//!
//! fn update_wands(mut inv: ResMut<WandInventory>) {
//!     inv.slot_count = 4;
//!     inv.selected_index = 1;
//! }
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(HudPlugin)
//!         .add_systems(Startup, setup_hud)
//!         .add_systems(Update, update_wands)
//!         .run();
//! }
//! ```

use bevy::prelude::*;
use bevy_egui::{EguiContext, PrimaryEguiContext};
use bevy_inspector_egui::egui;

// ── Public types ──────────────────────────────────────────────────────────────

/// HUD display mode.
///
/// Controls which overlay is active. In [`Hud::None`] (the default) the
/// standard game HUD rows — including the wand slot row — are displayed.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug, Reflect)]
#[reflect(Resource)]
pub enum Hud {
    /// Default game HUD: show base rows (matter row, wand row, …).
    #[default]
    None,
}

/// Tracks wand inventory state for the HUD wand slot row.
///
/// Update each frame to reflect the current inventory. The HUD system reads
/// [`slot_count`](Self::slot_count) and [`selected_index`](Self::selected_index)
/// to decide which slots show `NotchFull` vs `NotchEmpty`.
#[derive(Resource, Default, Clone, Reflect)]
#[reflect(Resource)]
pub struct WandInventory {
    /// Number of wand slots to display.
    pub slot_count: usize,
    /// Zero-based index of the currently selected wand slot.
    pub selected_index: usize,
}

/// Egui texture configuration for wand slot sprites from `ValueSlots.atlas`.
///
/// Insert this resource once, after registering your atlas image with egui via
/// [`EguiContexts::add_image`](bevy_egui::EguiContexts::add_image).
/// All UV rectangles are normalised (0.0 – 1.0).
#[derive(Resource)]
pub struct WandSlotAtlas {
    /// Egui `TextureId` for the full `ValueSlots` atlas image.
    pub texture_id: egui::TextureId,
    /// Normalised UV rect of the `NotchEmpty` slice.
    pub notch_empty_uv: egui::Rect,
    /// Normalised UV rect of the `NotchFull` slice.
    pub notch_full_uv: egui::Rect,
    /// Display size (pixels) of each slot icon in the HUD.
    pub slot_size: egui::Vec2,
}

// ── Plugin ────────────────────────────────────────────────────────────────────

/// Plugin that renders the wand slot row in the top-left HUD area.
///
/// The row is rendered only while [`Hud`] is [`Hud::None`] and is positioned
/// one row below the matter row. It requires a [`WandSlotAtlas`] resource to
/// draw sprites; without it the row is silently skipped.
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Hud>()
            .init_resource::<WandInventory>()
            .register_type::<Hud>()
            .register_type::<WandInventory>()
            .add_systems(
                bevy_inspector_egui::bevy_egui::EguiPrimaryContextPass,
                render_wand_hud,
            );
    }
}

// ── Systems ───────────────────────────────────────────────────────────────────

/// Pixel height reserved for the matter row above the wand slots.
const MATTER_ROW_HEIGHT: f32 = 20.0;
/// Outer margin from the window edges.
const HUD_MARGIN: f32 = 4.0;
/// Gap between adjacent wand slot icons.
const SLOT_GAP: f32 = 2.0;

/// Render the wand slot row as an egui overlay in the top-left corner.
///
/// Skipped when:
/// - [`Hud`] is not [`Hud::None`]
/// - [`WandInventory::slot_count`] is zero
/// - [`WandSlotAtlas`] has not been inserted
pub fn render_wand_hud(world: &mut World) {
    // Only render in the default HUD mode.
    if world.get_resource::<Hud>().copied().unwrap_or(Hud::None) != Hud::None {
        return;
    }

    let inventory = match world.get_resource::<WandInventory>() {
        Some(inv) if inv.slot_count > 0 => inv.clone(),
        _ => return,
    };

    // Snapshot atlas data so we can release the borrow before accessing EguiContext.
    let (texture_id, notch_empty_uv, notch_full_uv, slot_size) =
        match world.get_resource::<WandSlotAtlas>() {
            Some(atlas) => (
                atlas.texture_id,
                atlas.notch_empty_uv,
                atlas.notch_full_uv,
                atlas.slot_size,
            ),
            None => return,
        };

    let Ok(egui_ctx) = world
        .query_filtered::<&mut EguiContext, With<PrimaryEguiContext>>()
        .single(world)
    else {
        return;
    };
    let mut egui_ctx = egui_ctx.clone();
    let ctx = egui_ctx.get_mut();

    // Top-left position: left margin, just below the matter row.
    let top = HUD_MARGIN + MATTER_ROW_HEIGHT + SLOT_GAP;

    egui::Area::new(egui::Id::new("wand_hud_slots"))
        .fixed_pos(egui::pos2(HUD_MARGIN, top))
        .order(egui::Order::Foreground)
        .interactable(false)
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.x = SLOT_GAP;
            ui.horizontal(|ui| {
                for i in 0..inventory.slot_count {
                    let uv = if i == inventory.selected_index {
                        notch_full_uv
                    } else {
                        notch_empty_uv
                    };

                    ui.add(
                        egui::Image::new(egui::load::SizedTexture::new(texture_id, slot_size))
                            .uv(uv),
                    );
                }
            });
        });
}
