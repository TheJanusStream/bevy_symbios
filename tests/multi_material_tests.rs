use bevy::prelude::*;
use bevy_symbios::LSystemMeshBuilder;
use symbios_turtle_3d::{Skeleton, SkeletonPoint};

#[test]
fn test_multi_material_bucket_generation() {
    let mut s = Skeleton::new();

    // P0 -> P1 (Segment 0, Material 0)
    // Start P0 has mat_id=0. End P1 has mat_id=1.
    // The segment logic uses start_node.material_id.
    // So Segment 0 is Mat 0.

    s.add_node(
        SkeletonPoint {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            radius: 0.1,
            color: Vec4::new(1.0, 0.0, 0.0, 1.0), // Red
            material_id: 0,
            uv_scale: 1.0,
        },
        true,
    );

    // P1 -> P2 (Segment 1, Material 1)
    // Start P1 has mat_id=1. So Segment 1 is Mat 1.

    s.add_node(
        SkeletonPoint {
            position: Vec3::Y,
            rotation: Quat::IDENTITY,
            radius: 0.1,
            color: Vec4::new(1.0, 0.0, 0.0, 1.0),
            material_id: 1, // Determines material for NEXT segment
            uv_scale: 1.0,
        },
        false,
    );

    s.add_node(
        SkeletonPoint {
            position: Vec3::Y * 2.0,
            rotation: Quat::IDENTITY,
            radius: 0.1,
            color: Vec4::new(0.0, 1.0, 0.0, 1.0), // Green
            material_id: 1,
            uv_scale: 1.0,
        },
        false,
    );

    let builder = LSystemMeshBuilder::default();
    let meshes = builder.build(&s);

    assert_eq!(meshes.len(), 2, "Should generate 2 separate meshes");
    assert!(meshes.contains_key(&0), "Missing Material 0 bucket");
    assert!(meshes.contains_key(&1), "Missing Material 1 bucket");

    let mesh0 = meshes.get(&0).unwrap();
    // Segment 0: 2 rings * (8 resolution + 1 wrap) = 18 verts
    assert_eq!(mesh0.count_vertices(), 18, "Mesh 0 vertex count mismatch");

    let mesh1 = meshes.get(&1).unwrap();
    // Segment 1: 2 rings * 9 verts = 18 verts
    assert_eq!(mesh1.count_vertices(), 18, "Mesh 1 vertex count mismatch");
}
