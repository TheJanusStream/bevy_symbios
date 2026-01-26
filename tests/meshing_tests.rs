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
            color: Vec4::ONE,
            material_id: 0,
            uv_scale: 1.0,
        },
        true,
    );
    s.add_node(
        SkeletonPoint {
            position: Vec3::new(0.0, 1.0, 0.0),
            rotation: Quat::IDENTITY,
            radius: 0.1,
            color: Vec4::ONE,
            material_id: 0,
            uv_scale: 1.0,
        },
        false,
    );
    s
}

#[test]
fn test_mesh_generation_basics() {
    let skeleton = make_simple_skeleton();
    let builder = LSystemMeshBuilder::default();
    let meshes = builder.build(&skeleton);

    // We expect material 0 to exist
    let mesh = meshes.get(&0).expect("Mesh for material 0 not generated");

    assert_eq!(mesh.primitive_topology(), PrimitiveTopology::TriangleList);

    // Check attributes
    let positions = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .expect("Mesh missing positions");
    let normals = mesh
        .attribute(Mesh::ATTRIBUTE_NORMAL)
        .expect("Mesh missing normals");
    let colors = mesh
        .attribute(Mesh::ATTRIBUTE_COLOR)
        .expect("Mesh missing colors");

    // 2 rings * (8 resolution + 1 duplicate for wrapping) = 18 verts
    assert_eq!(positions.len(), 18);
    assert_eq!(normals.len(), 18);
    assert_eq!(colors.len(), 18);

    // Check Indices
    // 8 segments * 6 indices (2 tris) = 48 indices
    let indices = mesh.indices().expect("Mesh missing indices");
    assert_eq!(indices.len(), 48);
}

#[test]
fn test_empty_skeleton() {
    let skeleton = Skeleton::new();
    let builder = LSystemMeshBuilder::default();
    let meshes = builder.build(&skeleton);

    // Should return an empty map or no mesh for id 0
    assert!(meshes.is_empty());
}

#[test]
fn test_topology_types_alignment() {
    let p_glam = glam::Vec3::new(1.0, 2.0, 3.0);
    let p_bevy = bevy::math::Vec3::new(1.0, 2.0, 3.0);
    assert_eq!(p_glam.x, p_bevy.x);
}
