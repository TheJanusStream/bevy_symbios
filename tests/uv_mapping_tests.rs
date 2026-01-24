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
            roughness: 0.5,
            metallic: 0.0,
            texture_id: 0,
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
            roughness: 0.5,
            metallic: 0.0,
            texture_id: 0,
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
            roughness: 0.5,
            metallic: 0.0,
            texture_id: 0,
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
            roughness: 0.5,
            metallic: 0.0,
            texture_id: 0,
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
            roughness: 0.5,
            metallic: 0.0,
            texture_id: 0,
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
            roughness: 0.5,
            metallic: 0.0,
            texture_id: 0,
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
            roughness: 0.5,
            metallic: 0.0,
            texture_id: 0,
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
            roughness: 0.5,
            metallic: 0.0,
            texture_id: 0,
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
