# msg_inspector

A generic, modular Bevy UI framework for development panels and inspectors.

This crate provides a registration-based API where game modules can locally declare dev features, supporting pluggable analytics (read-only) and interactive (mutable) tabs.

## Features

- **Built-in tabs**: GameView, Hierarchy, Inspector, Resources, Assets, Diagnostics
- **Entity picking**: Click entities in the viewport to select them
- **Viewport management**: Automatic camera viewport clipping to dock area
- **Tab registration**: Games can register custom tabs via `InspectorExt` trait

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
msg_inspector = { git = "https://github.com/MolecularSadism/msg_inspector", tag = "v0.2.0" }
bevy = "0.17"
```

## Built-in Tabs

| Tab | Description |
|-----|-------------|
| Game | The game viewport, clipped to not overlap with panels |
| Hierarchy | Entity tree browser with search filtering |
| Inspector | Entity component inspector using reflection |
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
        .add_plugins(InspectorPlugin)
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

## Blocking Game Input Over Panels

Use `egui_pointer_over_area` to prevent game clicks when the cursor is over panels:

```rust
app.add_systems(Update, my_click_system.run_if(not(egui_pointer_over_area)));
```

## Toggle Visibility

Press the **Delete** key to toggle the inspector panel visibility.

## Bevy Version Compatibility

| `msg_inspector` | Bevy |
|-----------------|------|
| 0.2             | 0.17 |
| 0.1             | 0.16 |

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Contributing

Contributions are welcome! This crate is part of the [MolecularSadism](https://github.com/MolecularSadism) game development libraries.
