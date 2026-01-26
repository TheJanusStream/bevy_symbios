use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use bevy_symbios::LSystemMeshBuilder;
use symbios_turtle_3d::{Skeleton, SkeletonPoint};

fn get_uvs(mesh: &Mesh) -> &[[f32; 2]] {
    match mesh.attribute(Mesh::ATTRIBUTE_UV_0).expect("Missing UVs") {
        VertexAttributeValues::Float32x2(uvs) => uvs,
        _ => panic!("UVs should be Float32x2"),
    }
}

#[test]
fn test_uv_coordinates_present() {
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

    let builder = LSystemMeshBuilder::default();
    let meshes = builder.build(&s);
    let mesh = meshes.get(&0).expect("Mesh 0 not generated");

    let uvs = mesh
        .attribute(Mesh::ATTRIBUTE_UV_0)
        .expect("Mesh missing UV coordinates");

    // 2 rings * (8 resolution + 1 wrap) = 18 UVs
    assert_eq!(uvs.len(), 18, "UV count should match vertex count");
}

#[test]
fn test_uv_u_wraps_around() {
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

    let builder = LSystemMeshBuilder::new().with_resolution(8);
    let meshes = builder.build(&s);
    let mesh = meshes.get(&0).unwrap();

    let uv_data = get_uvs(mesh);

    // First ring: indices 0..9
    // U should go from 0.0 to 1.0 around the ring
    let first_u = uv_data[0][0];
    let last_u = uv_data[8][0]; // Last vertex of first ring (resolution=8, so index 8)

    assert!(
        (first_u - 0.0).abs() < 0.001,
        "First U should be 0.0, got {}",
        first_u
    );
    assert!(
        (last_u - 1.0).abs() < 0.001,
        "Last U should be 1.0, got {}",
        last_u
    );
}

#[test]
fn test_uv_v_increases_with_length() {
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
            position: Vec3::Y * 2.0, // 2 units long
            rotation: Quat::IDENTITY,
            radius: 0.1,
            color: Vec4::ONE,
            material_id: 0,
            uv_scale: 1.0,
        },
        false,
    );

    let builder = LSystemMeshBuilder::default();
    let meshes = builder.build(&s);
    let mesh = meshes.get(&0).unwrap();

    let uv_data = get_uvs(mesh);

    // First ring V (at origin)
    let v_start = uv_data[0][1];
    // Second ring V (at Y=2)
    let v_end = uv_data[9][1]; // First vertex of second ring

    assert!(
        (v_start - 0.0).abs() < 0.001,
        "V at start should be 0.0, got {}",
        v_start
    );
    assert!(
        v_end > v_start,
        "V should increase along the strand (start={}, end={})",
        v_start,
        v_end
    );
}

#[test]
fn test_uv_no_nans() {
    // Test with edge cases that might produce NaNs
    let mut s = Skeleton::new();
    s.add_node(
        SkeletonPoint {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            radius: 0.001, // Very small radius
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
            radius: 0.001,
            color: Vec4::ONE,
            material_id: 0,
            uv_scale: 1.0,
        },
        false,
    );

    let builder = LSystemMeshBuilder::default();
    let meshes = builder.build(&s);
    let mesh = meshes.get(&0).unwrap();

    let uv_data = get_uvs(mesh);

    for (i, uv) in uv_data.iter().enumerate() {
        assert!(
            uv[0].is_finite() && uv[1].is_finite(),
            "UV at index {} contains non-finite values: {:?}",
            i,
            uv
        );
    }
}

#[test]
fn test_uv_scale_multiplies_v_coordinate() {
    let scale = 3.0_f32;

    // Build a baseline mesh with uv_scale=1.0
    let mut s1 = Skeleton::new();
    s1.add_node(
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
    s1.add_node(
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
    let baseline_v = {
        let meshes = LSystemMeshBuilder::default().build(&s1);
        let uvs = get_uvs(meshes.get(&0).unwrap());
        uvs[9][1] // V of first vertex in second ring
    };

    // Build a scaled mesh with uv_scale=3.0
    let mut s2 = Skeleton::new();
    s2.add_node(
        SkeletonPoint {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            radius: 0.1,
            color: Vec4::ONE,
            material_id: 0,
            uv_scale: scale,
        },
        true,
    );
    s2.add_node(
        SkeletonPoint {
            position: Vec3::Y,
            rotation: Quat::IDENTITY,
            radius: 0.1,
            color: Vec4::ONE,
            material_id: 0,
            uv_scale: scale,
        },
        false,
    );
    let scaled_v = {
        let meshes = LSystemMeshBuilder::default().build(&s2);
        let uvs = get_uvs(meshes.get(&0).unwrap());
        uvs[9][1]
    };

    assert!(
        (scaled_v - baseline_v * scale).abs() < 0.001,
        "uv_scale={} should multiply V: expected ~{}, got {}",
        scale,
        baseline_v * scale,
        scaled_v
    );
}

#[test]
fn test_uv_v_continuous_across_tapered_segments() {
    // Two segments with different radii: the V coordinate at their shared boundary
    // must be identical (no discontinuity from per-segment circumference scaling).
    let mut s = Skeleton::new();
    s.add_node(
        SkeletonPoint {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            radius: 0.2, // Thick
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
            radius: 0.1, // Taper
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
            radius: 0.05, // Thinner
            color: Vec4::ONE,
            material_id: 0,
            uv_scale: 1.0,
        },
        false,
    );

    let builder = LSystemMeshBuilder::new().with_resolution(8);
    let meshes = builder.build(&s);
    let mesh = meshes.get(&0).unwrap();
    let uvs = get_uvs(mesh);

    // With vertex sharing and resolution=8, there are 3 rings of 9 vertices each.
    // Ring 0: indices 0..9, Ring 1 (shared): indices 9..18, Ring 2: indices 18..27
    assert_eq!(uvs.len(), 27, "3 shared rings * 9 verts = 27");

    // V at ring 0 should be 0
    assert!(
        uvs[0][1].abs() < 0.001,
        "V at start should be 0.0, got {}",
        uvs[0][1]
    );

    // V should strictly increase across all rings
    let v_ring0 = uvs[0][1];
    let v_ring1 = uvs[9][1];
    let v_ring2 = uvs[18][1];

    assert!(
        v_ring1 > v_ring0,
        "V should increase: ring0={}, ring1={}",
        v_ring0,
        v_ring1
    );
    assert!(
        v_ring2 > v_ring1,
        "V should increase: ring1={}, ring2={}",
        v_ring1,
        v_ring2
    );

    // All vertices within the same ring must share the same V coordinate
    for i in 0..9 {
        assert!(
            (uvs[9 + i][1] - v_ring1).abs() < 0.0001,
            "Ring 1 vertex {} V mismatch: expected {}, got {}",
            i,
            v_ring1,
            uvs[9 + i][1]
        );
    }
}
