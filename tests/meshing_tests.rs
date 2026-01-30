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

#[test]
fn test_vertex_sharing_same_material() {
    // 3 points, same material → 2 segments share the middle ring.
    // Without sharing: 4 rings = 36 vertices. With sharing: 3 rings = 27 vertices.
    let mut s = Skeleton::new();
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
            position: Vec3::Y,
            rotation: Quat::IDENTITY,
            radius: 0.1,
            color: Vec4::ONE,
            material_id: 0,
            uv_scale: 1.0,
        },
        false,
    );
    s.add_node(
        SkeletonPoint {
            position: Vec3::Y * 2.0,
            rotation: Quat::IDENTITY,
            radius: 0.1,
            color: Vec4::ONE,
            material_id: 0,
            uv_scale: 1.0,
        },
        false,
    );

    let meshes = LSystemMeshBuilder::default().build(&s);
    let mesh = meshes.get(&0).unwrap();

    // 3 unique rings * (8 resolution + 1 wrap) = 27 vertices
    assert_eq!(
        mesh.count_vertices(),
        27,
        "Vertex sharing should reduce 4 rings to 3"
    );

    // 2 segments * 8 quads * 2 triangles * 3 indices = 96 indices
    assert_eq!(mesh.indices().unwrap().len(), 96);
}

#[test]
fn test_no_vertex_sharing_across_materials() {
    // 3 points, material changes at boundary → no sharing possible.
    // Each segment gets 2 independent rings = 4 rings = 36 vertices.
    let mut s = Skeleton::new();
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
            position: Vec3::Y,
            rotation: Quat::IDENTITY,
            radius: 0.1,
            color: Vec4::ONE,
            material_id: 1, // Different material for next segment
            uv_scale: 1.0,
        },
        false,
    );
    s.add_node(
        SkeletonPoint {
            position: Vec3::Y * 2.0,
            rotation: Quat::IDENTITY,
            radius: 0.1,
            color: Vec4::ONE,
            material_id: 1,
            uv_scale: 1.0,
        },
        false,
    );

    let meshes = LSystemMeshBuilder::default().build(&s);

    // Material 0: 1 segment = 2 rings = 18 vertices
    let mesh0 = meshes.get(&0).unwrap();
    assert_eq!(mesh0.count_vertices(), 18);

    // Material 1: 1 segment = 2 rings = 18 vertices
    let mesh1 = meshes.get(&1).unwrap();
    assert_eq!(mesh1.count_vertices(), 18);
}

#[test]
fn test_resolution_clamping() {
    let skeleton = make_simple_skeleton();

    // Excessive resolution should be clamped to MAX_RESOLUTION (128)
    let meshes_high = LSystemMeshBuilder::new()
        .with_resolution(1_000_000)
        .build(&skeleton);
    let mesh_high = meshes_high.get(&0).unwrap();
    // 2 rings * (128 resolution + 1 wrap) = 258 vertices
    assert_eq!(mesh_high.count_vertices(), 258, "Should clamp to max 128");

    // Low resolution should be clamped to minimum 3
    let meshes_low = LSystemMeshBuilder::new()
        .with_resolution(1)
        .build(&skeleton);
    let mesh_low = meshes_low.get(&0).unwrap();
    // 2 rings * (3 resolution + 1 wrap) = 8 vertices
    assert_eq!(mesh_low.count_vertices(), 8, "Should clamp to min 3");
}
