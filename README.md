# bevy_symbios

Bevy integration for the [Symbios](https://crates.io/crates/symbios) L-System ecosystem.

Converts L-System skeletons into Bevy meshes and physics colliders for procedural plant generation, organic structures, and generative art.

## Features

- **Mesh Generation**: Smooth tube meshes from skeleton strands using parallel transport
- **Multi-Material Support**: Separate meshes per `u16` material ID for palette-driven PBR (bark, leaves, etc.)
- **Vertex Colors**: Per-vertex RGBA colors from skeleton data
- **UV Mapping**: Arc-length parameterized UVs with aspect-ratio preservation
- **Mesh Caching**: Optional fingerprint-keyed `MeshCache` to avoid re-meshing identical L-systems
- **Procedural Materials**: 23 procedural texture generators (Leaf, Twig, Bark, Brick, Plank, …) plus Grid/Noise/Checker previews
- **OBJ + GLB Export**: Pure data conversion for tooling / asset pipelines
- **Egui Editor** (optional): Drop-in `material_palette_editor` widget for live PBR + per-texture-config editing
- **Physics Colliders** (optional): Compound capsule colliders for Avian3D physics
- **Robot Spawning** (optional): Spawn articulated rigid-body robots from `symbios-robot` blueprints
- **Asset Loaders** (optional): `.lsys` grammars and `.matpalette.json` palettes loaded through `AssetServer` (hot-reload friendly)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
bevy_symbios = "0.4"
```

Feature flags:

| Feature         | Implies   | Enables                                                     |
|-----------------|-----------|-------------------------------------------------------------|
| `physics`       |           | `ColliderGenerator`, `PositionedCollider` (Avian3D)         |
| `egui`          |           | `ui::material_palette_editor` (via `bevy_egui`)             |
| `robot`         | `physics` | `spawn_robot`, `SpawnedRobot`, `ImuSensor`, `TouchSensor`   |
| `asset-loader`  |           | `LSystemAssetPlugin`, `.lsys` + `.matpalette.json` loaders  |

```toml
[dependencies]
bevy_symbios = { version = "0.4", features = ["physics", "egui", "asset-loader"] }
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

The `materials` module provides a palette-first PBR workflow with live editing support
driven by an explicit-trigger observer:

```rust
use bevy::prelude::*;
use bevy_symbios::materials::{
    MaterialSettingsMap, MaterialPalette, MaterialSettingsChanged,
    setup_material_assets, on_material_settings_changed, apply_foliage_textures,
};

// In your app setup:
app.init_resource::<MaterialSettingsMap>()
    .add_systems(Startup, setup_material_assets)
    .add_observer(on_material_settings_changed)
    .add_systems(Update, apply_foliage_textures);

// After editing MaterialSettingsMap, ask the observer to re-apply the palette:
fn on_edit(mut commands: Commands /* , ... */) {
    commands.trigger(MaterialSettingsChanged);
}
```

`MaterialSettingsMap` holds editable settings per material ID (base color, emission,
roughness, metallic, texture type, UV scale). The default seeds three entries (IDs 0/1/2)
but the map accepts any `u16` key. `on_material_settings_changed` is an **observer** —
nothing happens until you `commands.trigger(MaterialSettingsChanged)` after mutating the
map, which keeps re-application off the hot path and lets the UI decide when to commit
in-progress edits. For procedural foliage configs the observer spawns an async
generation task; `apply_foliage_textures` drains those tasks each frame and applies the
resulting handles to the `MaterialPalette`.

### Material Palette Editor (requires `egui` feature)

```toml
bevy_symbios = { version = "0.4", features = ["egui"] }
```

```rust
use bevy_symbios::ui::material_palette_editor;

// Inside an egui panel — returns true when a property changed in a way that
// warrants texture regeneration. Pair with `commands.trigger(MaterialSettingsChanged)`.
let regen = material_palette_editor(ui, &mut material_settings.settings);
```

The editor exposes PBR sliders plus a **Texture parameters** subsection that dispatches
to the per-variant editor for any of the 23 `bevy_symbios_texture` generators (Leaf,
Twig, Bark, Window, StainedGlass, IronGrille, Ground, Rock, Brick, Plank, Shingle,
Stucco, Concrete, Metal, Pavers, Ashlar, Cobblestone, Thatch, Marble, Corrugated,
Asphalt, Wainscoting, Encaustic).

### Mesh Caching

For scenes that re-spawn the same L-system many times (props, tile-based foliage,
deterministic regrowth) use `MeshCache` to avoid re-meshing identical skeletons:

```rust
use bevy::prelude::*;
use bevy_symbios::{LSystemMeshBuilder, MeshCache};

#[derive(Resource, Default)]
struct Cache(MeshCache);

fn spawn_cached(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cache: ResMut<Cache>,
    skeleton: symbios_turtle_3d::Skeleton,
) {
    let handles = LSystemMeshBuilder::new()
        .with_resolution(12)
        .build_cached(&skeleton, &mut cache.0, &mut meshes);

    for (mat_id, mesh_handle) in handles {
        commands.spawn(Mesh3d(mesh_handle));
        let _ = mat_id; // look up material from your palette
    }
}
```

`build_cached` fingerprints the skeleton (positions, rotations, radii, colors, material
IDs, UV scales) plus the builder's resolution. A matching fingerprint returns the cached
`Handle<Mesh>` map; otherwise it builds, inserts, and bumps the miss counter. Inspect
`cache.hits()` / `cache.misses()` for instrumentation, or use the lower-level
`MeshCache::get_or_insert_with` with `compute_skeleton_fingerprint` if you need to drive
caching outside `build_cached`. The cache does not LRU-evict — call `clear()`
periodically in long-running scenes that generate many unique skeletons.

### Robot Spawning (requires `robot` feature)

```rust
use bevy::prelude::*;
use bevy_symbios::{spawn_robot, materials::MaterialPalette};
use symbios_robot::RobotBlueprint;

fn spawn(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    palette: Res<MaterialPalette>,
    blueprint: Res<MyBlueprint>,
) {
    let spawned = spawn_robot(
        &mut commands,
        &blueprint.0,
        &palette,
        &mut meshes,
        Transform::from_xyz(0.0, 1.0, 0.0),
    );
    // spawned.modules / .joints / .sensors are returned for parenting or tagging.
}
# #[derive(Resource)] struct MyBlueprint(RobotBlueprint);
```

Module shapes (`Box`, `Cylinder`, `Sphere`, `Capsule`) are converted to both a Bevy
mesh and an Avian3D collider. `JointType::Fixed`, `Hinge`, `Ball`, and `Prismatic` map
to the corresponding Avian joints; `Screw` is approximated as `Fixed` with a warning
since Avian3D has no helical constraint. Hinge and prismatic joints pick the
`AxisLimit` whose `axis` aligns with the drive axis and install a motor with that
entry's effort/velocity. `IMU` and `Touch` sensors attach `ImuSensor` / `TouchSensor`
marker components.

### Asset Loaders (requires `asset-loader` feature)

Load `.lsys` grammars and `.matpalette.json` files through Bevy's `AssetServer`:

```rust
use bevy::prelude::*;
use bevy_symbios::loader::{LSystemAssetPlugin, LSystemSource, MaterialSettingsSource};

App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(LSystemAssetPlugin)
    .add_systems(Startup, |mut commands: Commands, assets: Res<AssetServer>| {
        commands.insert_resource(Handles {
            grammar: assets.load("example_tree.lsys"),
            palette: assets.load("example_palette.matpalette.json"),
        });
    })
    .run();
# #[derive(Resource)] struct Handles { grammar: Handle<LSystemSource>, palette: Handle<MaterialSettingsSource> }
```

The plugin registers `LSystemSource` (wrapping `symbios::System`) and
`MaterialSettingsSource` (wrapping `HashMap<u16, MaterialSettings>`) plus their loaders.
For runtime hot-reload, enable Bevy's `file_watcher` feature in your consuming crate
and listen for `AssetEvent::Modified` — see [`examples/hot_reload.rs`](examples/hot_reload.rs)
and the [`assets/`](assets/) sample files.

For one-shot parsing without the asset pipeline, the pure parsers are also exported:
`loader::parse_lsys_source(&str)` and `loader::parse_material_settings(&[u8])`.

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

`ExportFormat::{Obj, Glb}` (with `name()` / `extension()` helpers) is provided for
UI / CLI selection. The GLB writer emits `POSITION`, `NORMAL`, `COLOR_0`, and indexed
primitives with one PBR material per bucket; vertex tangents and UVs are not yet
included in the GLB output.

## API Reference

### `LSystemMeshBuilder`

| Method                                       | Description                                                                    |
|----------------------------------------------|--------------------------------------------------------------------------------|
| `new()`                                      | Create builder with default resolution (8)                                     |
| `with_resolution(n)`                         | Set vertices per ring (clamped to 3..=128)                                     |
| `build(&skeleton)`                           | Convert to `HashMap<u16, Mesh>` (one mesh per material ID)                     |
| `build_cached(&skeleton, &mut cache, meshes)`| Cache-aware variant returning `HashMap<u16, Handle<Mesh>>`; see [`MeshCache`]  |

### `MeshCache`

| Method / Free fn                             | Description                                                                    |
|----------------------------------------------|--------------------------------------------------------------------------------|
| `new()` / `default()`                        | Empty cache                                                                    |
| `len()` / `is_empty()`                       | Entry count                                                                    |
| `contains(&skeleton, resolution)`            | Probe a fingerprint without bumping counters                                   |
| `hits()` / `misses()` / `reset_stats()`      | Cumulative counters since construction or last reset                           |
| `clear()`                                    | Drop all cached entries (counters preserved)                                   |
| `get_or_insert_with(fingerprint, build)`     | Lookup-or-build by explicit fingerprint                                        |
| `compute_skeleton_fingerprint(&skel, res)`   | Free function — derive the same fingerprint `build_cached` uses                |

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
