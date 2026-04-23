//! Example demonstrating the Entities tab with principal component registration.
//!
//! Run with:
//! ```sh
//! cargo run --example entities_tab
//! ```
//!
//! This spawns several entities with different component combinations to
//! exercise the Entities tab:
//!
//! - **Characters** group containing **Enemy** and **Npc** principals
//! - **Pickup** entities with a custom display name ("Lootables")
//! - Entities with *multiple* principals (e.g., Enemy + Npc)
//! - Plain entities with no principal (appear under "Uncategorized")

use bevy::prelude::*;
use msg_inspector::prelude::*;

// ---- Principal components ------------------------------------------------

#[derive(Component)]
struct Enemy;

#[derive(Component)]
struct Npc;

#[derive(Component)]
struct Pickup;

// ---- Extra marker components (non-principal) -----------------------------

#[derive(Component)]
struct Health(f32);

#[derive(Component)]
struct Loot;

// ---- Setup ---------------------------------------------------------------

fn setup(mut commands: Commands) {
    // Camera — the inspector auto-discovers cameras rendering to the
    // primary window, so no tagging is required.
    commands.spawn(Camera2d);

    // Enemies
    for i in 0..5 {
        commands.spawn((
            Name::new(format!("Goblin {i}")),
            Enemy,
            Health(50.0 + i as f32 * 10.0),
            Transform::default(),
        ));
    }

    commands.spawn((
        Name::new("Dragon Boss"),
        Enemy,
        Health(500.0),
        Transform::default(),
    ));

    // NPCs
    for name in ["Shopkeeper", "Blacksmith", "Innkeeper"] {
        commands.spawn((
            Name::new(name),
            Npc,
            Health(100.0),
            Transform::default(),
        ));
    }

    // Pickups
    for i in 0..4 {
        commands.spawn((
            Name::new(format!("Health Potion {i}")),
            Pickup,
            Transform::default(),
        ));
    }

    commands.spawn((
        Name::new("Gold Coin"),
        Pickup,
        Loot,
        Transform::default(),
    ));

    // Entities with multiple principals (appear in several trees)
    commands.spawn((
        Name::new("Rogue Trader"),
        Npc,
        Enemy,
        Health(80.0),
        Transform::default(),
    ));

    commands.spawn((
        Name::new("Mimic Chest"),
        Enemy,
        Pickup,
        Health(30.0),
        Transform::default(),
    ));

    // Uncategorized entities (no principal)
    commands.spawn((
        Name::new("Background Music"),
        Transform::default(),
    ));

    commands.spawn((
        Name::new("Particle System"),
        Transform::default(),
    ));

    commands.spawn((
        Name::new("Trigger Zone"),
        Transform::default(),
    ));

    // A truly anonymous entity
    commands.spawn(Transform::default());
}

// ---- App -----------------------------------------------------------------

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(InspectorPlugin::default())
        // Group Enemy and Npc under a "Characters" parent category
        .register_principal_group::<(Enemy, Npc)>("Characters")
        // Register Pickup as a standalone principal with a custom display name
        .register_principal::<Pickup>().with_name("Lootables")
        .add_systems(Startup, setup)
        .run();
}
