use bevy::prelude::*;
use bevy_symbios::ColliderGenerator;
use symbios_turtle_3d::{Skeleton, SkeletonPoint};

fn make_simple_skeleton() -> Skeleton {
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
    s
}

#[test]
fn test_basic_collider_generation() {
    let skeleton = make_simple_skeleton();
    let generator = ColliderGenerator::new();
    let colliders = generator.build(&skeleton);

    assert_eq!(
        colliders.len(),
        1,
        "Should generate one collider for one segment"
    );

    let collider = &colliders[0];
    assert!(
        (collider.radius - 0.1).abs() < 0.001,
        "Radius should be 0.1"
    );
    assert!(
        (collider.length - 1.0).abs() < 0.001,
        "Length should be 1.0"
    );

    // Center should be at Y=0.5
    let center = collider.transform.translation;
    assert!((center.y - 0.5).abs() < 0.001, "Center Y should be 0.5");
}

#[test]
fn test_empty_skeleton_colliders() {
    let skeleton = Skeleton::new();
    let generator = ColliderGenerator::new();
    let colliders = generator.build(&skeleton);

    assert!(
        colliders.is_empty(),
        "Empty skeleton should produce no colliders"
    );
}

#[test]
fn test_min_radius_filtering() {
    let mut s = Skeleton::new();

    // Thin segment (radius 0.01)
    s.add_node(
        SkeletonPoint {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            radius: 0.01,
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
            radius: 0.01,
            color: Vec4::ONE,
            material_id: 0,
            uv_scale: 1.0,
        },
        false,
    );

    // Thick segment (radius 0.1)
    s.add_node(
        SkeletonPoint {
            position: Vec3::new(1.0, 0.0, 0.0),
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
            position: Vec3::new(1.0, 1.0, 0.0),
            rotation: Quat::IDENTITY,
            radius: 0.1,
            color: Vec4::ONE,
            material_id: 0,
            uv_scale: 1.0,
        },
        false,
    );

    // Without filtering: both segments
    let generator = ColliderGenerator::new();
    let colliders = generator.build(&s);
    assert_eq!(
        colliders.len(),
        2,
        "Should generate 2 colliders without filtering"
    );

    // With filtering: only thick segment
    let generator = ColliderGenerator::new().with_min_radius(0.05);
    let colliders = generator.build(&s);
    assert_eq!(
        colliders.len(),
        1,
        "Should generate 1 collider with min_radius=0.05"
    );
    assert!(
        (colliders[0].radius - 0.1).abs() < 0.001,
        "Should keep thick segment"
    );
}

#[test]
fn test_collider_orientation() {
    let mut s = Skeleton::new();

    // Horizontal segment along X axis
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
            position: Vec3::X,
            rotation: Quat::IDENTITY,
            radius: 0.1,
            color: Vec4::ONE,
            material_id: 0,
            uv_scale: 1.0,
        },
        false,
    );

    let generator = ColliderGenerator::new();
    let colliders = generator.build(&s);

    assert_eq!(colliders.len(), 1);

    let collider = &colliders[0];
    // The rotation should point Y axis toward X direction
    let rotated_y = collider.transform.rotation * Vec3::Y;
    assert!(
        (rotated_y - Vec3::X).length() < 0.001,
        "Collider Y axis should point along segment direction (X)"
    );
}

#[test]
fn test_multi_segment_strand() {
    let mut s = Skeleton::new();

    // 3 points = 2 segments
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

    let generator = ColliderGenerator::new();
    let colliders = generator.build(&s);

    assert_eq!(
        colliders.len(),
        2,
        "Should generate 2 colliders for 2 segments"
    );

    // Check centers are at Y=0.5 and Y=1.5
    let centers: Vec<f32> = colliders
        .iter()
        .map(|c| c.transform.translation.y)
        .collect();
    assert!(centers.iter().any(|&y| (y - 0.5).abs() < 0.001));
    assert!(centers.iter().any(|&y| (y - 1.5).abs() < 0.001));
}
