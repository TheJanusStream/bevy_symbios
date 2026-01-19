use bevy::prelude::*;
use bevy_symbios::LSystemMeshBuilder;
use symbios_turtle_3d::{Skeleton, SkeletonPoint};

#[test]
fn test_180_degree_singularity() {
    // The "Fold-Back" Problem.
    // If a strand goes Up, then immediately Down, the tangent reverses (0,1,0) -> (0,-1,0).
    // Parallel Transport frames often rely on cross products that vanish in this case.

    let mut s = Skeleton::new();

    // Point 0: Origin
    s.add_node(
        SkeletonPoint {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            radius: 0.1,
        },
        true,
    );

    // Point 1: Up
    s.add_node(
        SkeletonPoint {
            position: Vec3::new(0.0, 1.0, 0.0),
            rotation: Quat::IDENTITY,
            radius: 0.1,
        },
        false,
    );

    // Point 2: Back to Origin (180 deg turn)
    s.add_node(
        SkeletonPoint {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            radius: 0.1,
        },
        false,
    );

    let builder = LSystemMeshBuilder::default();
    let mesh = builder.build(&s);

    // Verify we got a mesh
    let positions = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .unwrap()
        .as_float3()
        .unwrap();

    // Verify no NaNs in the output
    for pos in positions {
        assert!(
            !pos[0].is_nan() && !pos[1].is_nan() && !pos[2].is_nan(),
            "Mesh contains NaN vertices at singularity"
        );
    }
}

#[test]
fn test_zero_length_segment_collapse() {
    // Two points at exact same location (should be filtered or handled)
    let mut s = Skeleton::new();
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
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            radius: 0.1,
        },
        false,
    );

    let builder = LSystemMeshBuilder::default();
    let mesh = builder.build(&s);

    // Should ideally be empty or handle it gracefully without NaN normals
    if let Some(normals) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
        let norms = normals.as_float3().unwrap();
        for n in norms {
            assert!(
                !n[0].is_nan(),
                "NaN normal generated from zero-length segment"
            );
        }
    }
}
