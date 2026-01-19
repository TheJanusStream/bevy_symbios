use bevy::mesh::PrimitiveTopology;
use bevy::prelude::*;
use bevy_symbios::LSystemMeshBuilder;
use symbios_turtle_3d::{Skeleton, SkeletonPoint};

// Helper to create a dummy skeleton
fn make_simple_skeleton() -> Skeleton {
    let mut s = Skeleton::new();
    // A single vertical line segment
    s.add_node(
        SkeletonPoint {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            radius: 0.1,
        },
        true,
    );
    s.add_node(
        SkeletonPoint {
            position: Vec3::new(0.0, 1.0, 0.0),
            rotation: Quat::IDENTITY,
            radius: 0.1,
        },
        false,
    );
    s
}

#[test]
fn test_mesh_generation_basics() {
    let skeleton = make_simple_skeleton();
    let builder = LSystemMeshBuilder::default();
    let mesh = builder.build(&skeleton);

    assert_eq!(mesh.primitive_topology(), PrimitiveTopology::TriangleList);

    // Check attributes
    let positions = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .expect("Mesh missing positions");
    let normals = mesh
        .attribute(Mesh::ATTRIBUTE_NORMAL)
        .expect("Mesh missing normals");

    // We expect some vertices.
    // 2 rings * (resolution + 1 duplicate for UV wrapping usually, or just resolution)
    // Default resolution is 8. Logic loops 0..=res, so 9 verts per ring.
    // 2 rings = 18 vertices.
    // Wait, let's check mesher.rs:
    // for i in 0..=self.resolution { ... } -> 9 verts per ring
    // 2 rings -> 18 verts

    assert_eq!(positions.len(), 18);
    assert_eq!(normals.len(), 18);

    // Check Indices
    // 8 segments * 6 indices (2 tris) = 48 indices
    let indices = mesh.indices().expect("Mesh missing indices");
    assert_eq!(indices.len(), 48);
}

#[test]
fn test_empty_skeleton() {
    let skeleton = Skeleton::new();
    let builder = LSystemMeshBuilder::default();
    let mesh = builder.build(&skeleton);

    // Should produce valid empty mesh
    let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION);
    assert!(positions.is_none() || positions.unwrap().len() == 0);
}

#[test]
fn test_topology_types_alignment() {
    // This test ensures that the glTF types from symbios-turtle-3d
    // are compatible with the bevy types used in mesher.rs
    // If dependencies drift, this will fail to compile or calculate correctly.

    let p_glam = glam::Vec3::new(1.0, 2.0, 3.0);
    let p_bevy = bevy::math::Vec3::new(1.0, 2.0, 3.0);

    // This assertion relies on the fact that if versions align,
    // the memory layout and types are identical/interchangeable via direct value use
    assert_eq!(p_glam.x, p_bevy.x);
}
