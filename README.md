# msg_inspector

[![CI](https://github.com/MolecularSadism/msg_inspector/workflows/CI/badge.svg)](https://github.com/MolecularSadism/msg_inspector/actions)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/MolecularSadism/msg_inspector#license)
[![Bevy](https://img.shields.io/badge/Bevy-0.18-blue.svg)](https://bevyengine.org/)
[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](https://www.rust-lang.org/)


A generic, modular Bevy UI framework for development panels and inspectors.

This crate provides a registration-based API where game modules can locally declare dev features, supporting pluggable analytics (read-only) and interactive (mutable) tabs.

## Features

- **Built-in tabs**: GameView, Hierarchy, Inspector, Resources, Assets, Diagnostics
- **Relationship navigation**: The Inspector lists every relationship component on the selected
  entity (`ChildOf`, `Children`, and any custom relationship) with a select button per referenced
  entity, so inspection can follow the relationship graph
- **Entity picking**: Alt+Left click inside the game viewport selects the topmost sprite under
  the cursor (Shift/Ctrl extends the selection); mark sprites with `PickingIgnore` to exclude them
- **Viewport management**: Automatic camera viewport clipping to dock area
- **Tab registration**: Games can register custom tabs via `InspectorExt` trait

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
msg_inspector = { git = "https://github.com/MolecularSadism/msg_inspector", tag = "v0.3.0" }
bevy = "0.18"
```

## Built-in Tabs

| Tab | Description |
|-----|-------------|
| Game | The game viewport, clipped to not overlap with panels |
| Hierarchy | Entity tree browser with search filtering |
| Inspector | Entity component inspector using reflection, with relationship navigation |
| Resources | Browse all registered resources |
| Assets | Browse all loaded asset handles |
| Diagnostics | FPS, frame time, and entity count metrics |

## Quick Start

```rust
use bevy::prelude::*;
use msg_inspector::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(InspectorPlugin::default())
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    // Mark your main camera for viewport management
    commands.spawn((
        Camera2d,
        InspectorMainCamera,
    ));
}
```

## Registering Custom Tabs

### Analytics Tab (Read-Only)

Use for displaying stats without modifying game state:

```rust
app.register_inspector_analytics("physics_stats", "Physics", |ui, world| {
    if let Some(rapier) = world.get_resource::<RapierContext>() {
        ui.label(format!("Bodies: {}", rapier.bodies.len()));
    }
});
```

### Interactive Tab (Mutable)

Use when you need to trigger events or modify state:

```rust
app.register_inspector_interactive("spawner", "Actors", |ui, world| {
    if ui.button("Spawn Enemy").clicked() {
        world.commands().spawn(EnemyBundle::default());
    }
});
```

## Custom Diagnostics Counters

Register component counters to track how many entities have a given component, displayed in the Diagnostics tab:

```rust
#[derive(Component)]
struct Collider;

#[derive(Component)]
struct RigidBody;

app.add_plugins(
    InspectorPlugin::default()
        .with_counter::<Collider>()
        .with_counter::<RigidBody>()
);
```

You can also register arbitrary counters with a custom label and count function:

```rust
app.add_plugins(
    InspectorPlugin::default()
        .with_custom_counter("Visible Meshes", |world| {
            // any logic that returns a usize
            world.entities().len() as usize
        })
);
```

## Blocking Game Input Over Panels

Use `egui_pointer_over_area` to prevent game clicks when the cursor is over panels:

```rust
app.add_systems(Update, my_click_system.run_if(not(egui_pointer_over_area)));
```

## Relationship Navigation

Inspecting a single entity shows a **Relationships** section above the component list. Each
relationship component the entity carries gets its own group — `ChildOf` and `Children`, plus any
component declared with `#[relationship]` / `#[relationship_target]` — listing the entities it
references by name and id. Discovery is type-erased through Bevy's relationship accessors, so
custom relationships show up without registering anything:

```rust
#[derive(Component)]
#[relationship(relationship_target = Followers)]
struct Follows(Entity);

#[derive(Component)]
#[relationship_target(relationship = Follows)]
struct Followers(Vec<Entity>);
```

Click the `⏵` button next to an entry to make that entity the inspection target; hold Shift or
Ctrl to add it to the current selection instead of replacing it. Arrows show the direction:
`→` for a relationship pointing away from this entity, `←` for a target collection pointing back
at it.

## Entity Picking

Hold **Alt** and left-click inside the game viewport to select the topmost sprite under the
cursor; hold Shift or Ctrl as well to extend the current selection. Plain left clicks are left
to the game. Picking uses the camera marked `InspectorMainCamera`, so it stays accurate while
the viewport is clipped to the Game tab, and tests sprites in their local frame — rotation,
scale, anchors, sprite rects, and texture-atlas frames all count.

Sprites that cover the view without being useful pick targets (a camera-following, screen-sized
canvas sprite, for instance) can opt out:

```rust
commands.spawn((Sprite::default(), PickingIgnore));
```

## Toggle Visibility

Press the **Delete** key to toggle the inspector panel visibility.

## Bevy Version Compatibility

| `msg_inspector` | Bevy |
|-----------------|------|
| 0.4             | 0.18 |
| 0.3             | 0.18 |
| 0.2             | 0.17 |
| 0.1             | 0.16 |

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Contributing

Contributions are welcome! This crate is part of the [MolecularSadism](https://github.com/MolecularSadism) game development libraries.
