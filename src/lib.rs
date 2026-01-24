//! Bevy integration for the Symbios L-System ecosystem.
//!
//! This crate provides tools to convert L-System skeletons from [`symbios_turtle_3d`]
//! into Bevy-compatible meshes and physics colliders.
//!
//! # Features
//!
//! - **Mesh generation**: Convert skeletons to smooth tube meshes with vertex colors,
//!   UV mapping, and multi-material support via [`LSystemMeshBuilder`].
//! - **Physics colliders** (optional): Generate capsule colliders for physics simulation
//!   via [`ColliderGenerator`]. Requires the `physics` feature.
//!
//! # Feature Flags
//!
//! - `physics`: Enables [`ColliderGenerator`] and [`PositionedCollider`] for Avian3D
//!   physics integration.
//!
//! # Example
//!
//! ```ignore
//! use bevy::prelude::*;
//! use bevy_symbios::{LSystemMeshBuilder, symbios_turtle_3d::Skeleton};
//!
//! fn spawn_lsystem(
//!     mut commands: Commands,
//!     mut meshes: ResMut<Assets<Mesh>>,
//!     mut materials: ResMut<Assets<StandardMaterial>>,
//!     skeleton: Skeleton,
//! ) {
//!     let mesh_map = LSystemMeshBuilder::new()
//!         .with_resolution(12)
//!         .build(&skeleton);
//!
//!     for (material_id, mesh) in mesh_map {
//!         commands.spawn((
//!             Mesh3d(meshes.add(mesh)),
//!             MeshMaterial3d(materials.add(Color::srgb(0.4, 0.7, 0.3))),
//!         ));
//!     }
//! }
//! ```

pub mod mesher;

#[cfg(feature = "physics")]
pub mod collider;

pub use mesher::LSystemMeshBuilder;

#[cfg(feature = "physics")]
pub use collider::{ColliderGenerator, PositionedCollider};

/// Re-export of `symbios_turtle_3d` for version compatibility.
pub use symbios_turtle_3d;
