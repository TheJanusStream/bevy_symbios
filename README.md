# bevy_symbios

Bevy integration for the [Symbios](https://crates.io/crates/symbios) L-System ecosystem.

Converts L-System skeletons into Bevy meshes and physics colliders for procedural plant generation, organic structures, and generative art.

## Features

- **Mesh Generation**: Smooth tube meshes from skeleton strands using parallel transport
- **Multi-Material Support**: Separate meshes per material ID for palette-driven PBR (bark, leaves, etc.)
- **Vertex Colors**: Per-vertex RGBA colors from skeleton data
- **UV Mapping**: Arc-length parameterized UVs with aspect-ratio preservation
- **Physics Colliders** (optional): Compound capsule colliders for Avian3D physics
- **Robot Spawning** (optional): Spawn articulated rigid-body robots from `symbios-robot` blueprints

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
bevy_symbios = "0.3"
```

For physics support with [Avian3D](https://github.com/Jondolf/avian):

```toml
[dependencies]
bevy_symbios = { version = "0.3", features = ["physics"] }
```

For robot spawning (enables `physics` automatically):

```toml
[dependencies]
bevy_symbios = { version = "0.3", features = ["robot"] }
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

    // Define a material palette: each material ID maps to PBR properties
    let palette: Vec<StandardMaterial> = vec![
        StandardMaterial {                       // ID 0: Bark
            base_color: Color::WHITE,            // Tinted by vertex colors
            perceptual_roughness: 0.9,
            metallic: 0.0,
            ..default()
        },
        StandardMaterial {                       // ID 1: Leaves
            base_color: Color::WHITE,
            perceptual_roughness: 0.6,
            metallic: 0.1,
            ..default()
        },
    ];

    // Spawn each material's mesh with its palette entry
    for (material_id, mesh) in mesh_map {
        let mat = palette
            .get(material_id as usize)
            .cloned()
            .unwrap_or_default();

        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(mat)),
        ));
    }
}
```

### Multi-Material Workflow

The material system separates **PBR surface properties** from **local color variation**:

- **Material ID** (`SkeletonPoint::material_id`) — Selects a palette entry that defines
  surface properties like roughness, metallic, and emissive. Each unique ID produces a
  separate mesh, so different Bevy `StandardMaterial`s can be applied per group.
- **Vertex Colors** (`SkeletonPoint::color`) — Baked into mesh vertices as `ATTRIBUTE_COLOR`.
  These provide per-vertex tinting (e.g. darker bark at branch bases, lighter tips on
  leaves) without needing additional materials or textures.

Set `base_color: Color::WHITE` on your palette materials so vertex colors pass through
unmodified. Any non-white base color will multiply with the vertex color.

### Physics Colliders

Generate a compound capsule collider for physics simulation (requires `physics` feature):

```rust
use bevy::prelude::*;
use bevy_symbios::{ColliderGenerator, symbios_turtle_3d::Skeleton};

fn spawn_with_collider(
    mut commands: Commands,
    skeleton: Skeleton,
) {
    // Generate a single compound collider, filtering out thin branches
    if let Some(collider) = ColliderGenerator::new()
        .with_min_radius(0.05)  // Ignore twigs thinner than 5cm
        .build(&skeleton)
    {
        commands.spawn((Transform::default(), collider));
    }
}
```

### Working with Symbios

This crate works with skeletons from the [symbios-turtle-3d](https://crates.io/crates/symbios-turtle-3d) interpreter:

```rust
use symbios::System;
use symbios_turtle_3d::{TurtleConfig, TurtleInterpreter};
use bevy_symbios::LSystemMeshBuilder;

// Parse and derive an L-System
let mut sys = System::new();
sys.set_axiom("F").unwrap();
sys.add_rule("p1: F -> F[+F]F[-F]F").unwrap();
sys.derive(4).unwrap();

// Interpret derived state as a 3D skeleton
let mut interpreter = TurtleInterpreter::new(TurtleConfig::default());
interpreter.populate_standard_symbols(&sys.interner);
let skeleton = interpreter.build_skeleton(&sys.state);

// Now use LSystemMeshBuilder to create meshes
let meshes = LSystemMeshBuilder::new()
    .with_resolution(8)
    .build(&skeleton);
```

### Material Palette System

The `materials` module provides a palette-first PBR workflow with live editing support:

```rust
use bevy::prelude::*;
use bevy_symbios::materials::{
    MaterialSettingsMap, MaterialPalette,
    setup_material_assets, sync_material_properties,
};

// In your app setup:
app.init_resource::<MaterialSettingsMap>()
    .add_systems(Startup, setup_material_assets)
    .add_systems(Update, sync_material_properties);
```

`MaterialSettingsMap` holds editable settings per material ID (base color, emission, roughness, metallic, texture type, UV scale). The default provides 3 entries but the map accepts any `u8` key. `sync_material_properties` updates the Bevy `StandardMaterial` handles in `MaterialPalette` each frame without requiring geometry rebuilds.

### Material Palette Editor (requires `egui` feature)

```toml
bevy_symbios = { version = "0.3", features = ["egui"] }
```

```rust
use bevy_symbios::ui::material_palette_editor;

// Inside an egui panel:
material_palette_editor(ui, &mut material_settings.settings);
```

### Export

The `export` module converts meshes to OBJ and GLB (binary glTF) formats:

```rust
use bevy_symbios::export::{mesh_to_obj, meshes_to_obj, meshes_to_glb, ExportFormat};

// OBJ string from a single mesh (vertex_offset=0 for standalone)
let obj_string = mesh_to_obj(&mesh, "tree", 0);

// Combined OBJ from all material buckets
let obj_combined = meshes_to_obj(&mesh_map, "tree");

// GLB binary with embedded PBR materials
let glb_bytes = meshes_to_glb(&mesh_map, &material_settings_map.settings);
```

## API Reference

### `LSystemMeshBuilder`

| Method               | Description                                |
|----------------------|--------------------------------------------|
| `new()`              | Create builder with default resolution (8) |
| `with_resolution(n)` | Set vertices per ring (min 3)              |
| `build(&skeleton)`   | Convert to `HashMap<u8, Mesh>`             |

### `ColliderGenerator` (requires `physics` feature)

| Method                   | Description                                              |
|--------------------------|----------------------------------------------------------|
| `new()`                  | Create generator with no filtering                       |
| `with_min_radius(r)`     | Skip segments thinner than `r`                           |
| `build(&skeleton)`       | Generate `Option<Collider>` (single compound collider)   |
| `build_parts(&skeleton)` | Generate `Vec<PositionedCollider>` (individual segments) |

### `PositionedCollider`

| Field       | Type        | Description                                    |
|-------------|-------------|------------------------------------------------|
| `transform` | `Transform` | World-space position and rotation              |
| `collider`  | `Collider`  | Avian3D capsule (or sphere for short segments) |
| `radius`    | `f32`       | Average segment radius                         |
| `length`    | `f32`       | Segment length                                 |

## Mesh Attributes

Generated meshes include:

| Attribute  | Description                                                                 |
|------------|-----------------------------------------------------------------------------|
| `POSITION` | Vertex positions                                                            |
| `NORMAL`   | Smooth normals                                                              |
| `COLOR`    | RGBA vertex colors for local tinting (`SkeletonPoint::color`)               |
| `UV_0`     | Texture coordinates (U: around tube, V: along strand, scaled by `uv_scale`) |
| `TANGENT`  | Tangent vectors (auto-generated for normal mapping)                         |

## License

MIT
