//! Bevy integration for the Symbios L-System ecosystem.
//!
//! This crate provides tools to convert L-System skeletons from [`symbios_turtle_3d`]
//! into Bevy-compatible meshes, physics colliders, and configurable materials.
//!
//! # Features
//!
//! - **Mesh generation**: Convert skeletons to smooth tube meshes with vertex colors,
//!   UV mapping, and multi-material support via [`LSystemMeshBuilder`].
//! - **Material system**: Configurable PBR materials with procedural textures,
//!   palette-first workflow, and automatic sync via [`materials`].
//! - **Export**: OBJ and GLB export utilities via [`export`].
//! - **Physics colliders** (optional): Generate capsule colliders for physics simulation
//!   via [`ColliderGenerator`]. Requires the `physics` feature.
//! - **Egui UI helpers** (optional): Reusable material palette editor widget via [`ui`].
//!   Requires the `egui` feature.
//!
//! # Feature Flags
//!
//! - `physics`: Enables [`ColliderGenerator`] and [`PositionedCollider`] for Avian3D
//!   physics integration.
//! - `egui`: Enables [`ui::material_palette_editor`] for `bevy_egui`-based material editing.
//!
//! # Example
//!
//! ```ignore
//! use bevy::prelude::*;
//! use bevy_symbios::{LSystemMeshBuilder, materials::*};
//!
//! fn setup(app: &mut App) {
//!     app.init_resource::<MaterialSettingsMap>()
//!        .add_systems(Startup, setup_material_assets)
//!        .add_systems(Update, sync_material_properties);
//! }
//!
//! fn spawn_lsystem(
//!     mut commands: Commands,
//!     mut meshes: ResMut<Assets<Mesh>>,
//!     palette: Res<MaterialPalette>,
//!     skeleton: symbios_turtle_3d::Skeleton,
//! ) {
//!     let mesh_map = LSystemMeshBuilder::new()
//!         .with_resolution(12)
//!         .build(&skeleton);
//!
//!     for (material_id, mesh) in mesh_map {
//!         let material = palette
//!             .materials
//!             .get(&material_id)
//!             .unwrap_or(&palette.primary_material)
//!             .clone();
//!         commands.spawn((
//!             Mesh3d(meshes.add(mesh)),
//!             MeshMaterial3d(material),
//!         ));
//!     }
//! }
//! ```

pub mod export;
pub mod materials;
pub mod mesher;

#[cfg(feature = "physics")]
pub mod collider;

#[cfg(feature = "egui")]
pub mod ui;

pub use mesher::LSystemMeshBuilder;

#[cfg(feature = "physics")]
pub use collider::{ColliderGenerator, PositionedCollider};

/// Re-export of `symbios_turtle_3d` for version compatibility.
pub use symbios_turtle_3d;
