//! Example: the wide bitmask layer widget with an enum-registered layer set.
//!
//! Run with:
//! ```sh
//! cargo run --example bitmask_layers
//! ```
//!
//! Registers `PhysicsLayer` as a named bitmask layer set and renders an
//! interactive tab that edits a `u32` mask with [`bitmask_field_layers`],
//! toggling between the enum's layer names and bare bit indices. The per-layer
//! checkbox list starts folded to save panel space.

use bevy::prelude::*;
use msg_inspector::prelude::*;

/// A bitflags-style layer enum: each variant names the bit at its declaration
/// index (`Ground` → bit 0, `Water` → bit 1, ...).
#[derive(Reflect)]
enum PhysicsLayer {
    Ground,
    Water,
    Air,
    Fire,
    Ice,
    Lava,
}

fn setup(mut commands: Commands) {
    commands.spawn((Camera2d, InspectorMainCamera));
}

fn main() {
    // Interactive-tab state, owned by the closure across frames.
    let mut mask: u32 = 0b0010_1011;
    let mut show_names = true;

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(InspectorPlugin::default())
        .register_bitmask_enum::<PhysicsLayer>()
        .register_inspector_interactive("collision_mask", "Collision Mask", move |ui, world| {
            ui.checkbox(&mut show_names, "Show layer names");
            ui.add_space(4.0);

            let registry = world.resource::<BitmaskRegistry>();
            let layers = show_names.then(|| registry.get::<PhysicsLayer>()).flatten();
            bitmask_field_layers(ui, &mut mask, layers);

            ui.add_space(4.0);
            ui.label(format!("mask = {mask:#010b}"));
        })
        .add_systems(Startup, setup)
        .run();
}
