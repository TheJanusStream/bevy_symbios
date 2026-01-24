# bevy_symbios

Bevy integration for the [Symbios](https://crates.io/crates/symbios) L-System ecosystem.

Converts L-System skeletons into Bevy meshes and physics colliders for procedural plant generation, organic structures, and generative art.

## Features

- **Mesh Generation**: Smooth tube meshes from skeleton strands using parallel transport
- **Multi-Material Support**: Separate meshes per material ID for varied materials (bark, leaves, etc.)
- **Vertex Colors**: Per-vertex RGBA colors from skeleton data
- **UV Mapping**: Arc-length parameterized UVs with aspect-ratio preservation
- **Physics Colliders** (optional): Capsule colliders for Avian3D physics

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
bevy_symbios = "0.1"
```

For physics support with [Avian3D](https://github.com/Jondolf/avian):

```toml
[dependencies]
bevy_symbios = { version = "0.1", features = ["physics"] }
```

## Usage

### Basic Mesh Generation

```rust
use bevy::prelude::*;
use bevy_symbios::{LSystemMeshBuilder, symbios_turtle_3d::Skeleton};

fn spawn_tree(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    skeleton: Skeleton,
) {
    // Convert skeleton to meshes (one per material ID)
    let mesh_map = LSystemMeshBuilder::new()
        .with_resolution(12)  // Vertices around tube circumference
        .build(&skeleton);

    // Spawn each material's mesh
    for (material_id, mesh) in mesh_map {
        let color = match material_id {
            0 => Color::srgb(0.4, 0.3, 0.2),  // Bark
            1 => Color::srgb(0.3, 0.6, 0.2),  // Leaves
            _ => Color::WHITE,
        };

        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                ..default()
            })),
        ));
    }
}
```

### Physics Colliders

Generate capsule colliders for physics simulation (requires `physics` feature):

```rust
use bevy::prelude::*;
use bevy_symbios::{ColliderGenerator, symbios_turtle_3d::Skeleton};

fn spawn_with_colliders(
    mut commands: Commands,
    skeleton: Skeleton,
) {
    // Generate colliders, filtering out thin branches
    let colliders = ColliderGenerator::new()
        .with_min_radius(0.05)  // Ignore twigs thinner than 5cm
        .build(&skeleton);

    for positioned in colliders {
        commands.spawn((
            positioned.transform,
            positioned.collider,
        ));
    }
}
```

### Working with Symbios

This crate works with skeletons from the [symbios-turtle-3d](https://crates.io/crates/symbios-turtle-3d) interpreter:

```rust
use symbios::{LSystem, Parser};
use bevy_symbios::symbios_turtle_3d::{Interpreter3D, Skeleton};

// Parse L-System grammar
let grammar = r#"
    axiom: F
    rules:
      F -> F[+F]F[-F]F
"#;
let lsystem = Parser::parse(grammar).unwrap();

// Generate string after iterations
let expanded = lsystem.expand(4);

// Interpret as 3D skeleton
let mut interpreter = Interpreter3D::new();
let skeleton: Skeleton = interpreter.interpret(&expanded);

// Now use LSystemMeshBuilder to create meshes
```

## API Reference

### `LSystemMeshBuilder`

| Method | Description |
|--------|-------------|
| `new()` | Create builder with default resolution (8) |
| `with_resolution(n)` | Set vertices per ring (min 3) |
| `build(&skeleton)` | Convert to `HashMap<u8, Mesh>` |

### `ColliderGenerator` (requires `physics` feature)

| Method | Description |
|--------|-------------|
| `new()` | Create generator with no filtering |
| `with_min_radius(r)` | Skip segments thinner than `r` |
| `build(&skeleton)` | Generate `Vec<PositionedCollider>` |

### `PositionedCollider`

| Field | Type | Description |
|-------|------|-------------|
| `transform` | `Transform` | World-space position and rotation |
| `collider` | `Collider` | Avian3D capsule collider |
| `radius` | `f32` | Average segment radius |
| `length` | `f32` | Segment length |

## Mesh Attributes

Generated meshes include:

| Attribute | Description |
|-----------|-------------|
| `POSITION` | Vertex positions |
| `NORMAL` | Smooth normals |
| `COLOR` | RGBA vertex colors from `SkeletonPoint::color` |
| `UV_0` | Texture coordinates (U: around tube, V: along strand) |

## Compatibility

| bevy_symbios | Bevy | Avian3D |
|--------------|------|---------|
| 0.1.x | 0.17 | 0.4 |

## License

MIT
