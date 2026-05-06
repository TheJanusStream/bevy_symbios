use bevy::prelude::*;
use bevy_symbios::{LSystemMeshBuilder, MeshCache};
use symbios_turtle_3d::{Skeleton, SkeletonPoint};

fn make_skeleton(positions: &[Vec3]) -> Skeleton {
    let mut s = Skeleton::new();
    for (i, p) in positions.iter().enumerate() {
        let point = SkeletonPoint {
            position: *p,
            rotation: Quat::IDENTITY,
            radius: 0.1,
            color: Vec4::ONE,
            material_id: 0,
            uv_scale: 1.0,
        };
        if i == 0 {
            s.start_strand(point, None);
        } else {
            s.push_node(point);
        }
    }
    s
}

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<Mesh>();
    app
}

#[test]
fn cache_hit_returns_same_handle() {
    let mut app = test_app();
    let skel = make_skeleton(&[Vec3::ZERO, Vec3::Y, Vec3::Y * 2.0]);
    let mut cache = MeshCache::new();

    let handles_first = {
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        LSystemMeshBuilder::new().build_cached(&skel, &mut cache, &mut meshes)
    };
    let handles_second = {
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        LSystemMeshBuilder::new().build_cached(&skel, &mut cache, &mut meshes)
    };

    assert_eq!(cache.len(), 1, "Same skeleton must produce a single entry");
    let h1 = handles_first.get(&0).unwrap();
    let h2 = handles_second.get(&0).unwrap();
    assert_eq!(
        h1.id(),
        h2.id(),
        "Cache hit must return the same Mesh handle"
    );
}

#[test]
fn cache_hit_does_not_grow_assets() {
    let mut app = test_app();
    let skel = make_skeleton(&[Vec3::ZERO, Vec3::Y, Vec3::Y * 2.0]);
    let mut cache = MeshCache::new();

    {
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        LSystemMeshBuilder::new().build_cached(&skel, &mut cache, &mut meshes);
    }
    let count_after_first = app.world().resource::<Assets<Mesh>>().len();

    for _ in 0..10 {
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        LSystemMeshBuilder::new().build_cached(&skel, &mut cache, &mut meshes);
    }
    let count_after_repeats = app.world().resource::<Assets<Mesh>>().len();

    assert_eq!(
        count_after_first, count_after_repeats,
        "Cache hits must not insert new Mesh assets"
    );
}

#[test]
fn cache_miss_on_skeleton_change() {
    let mut app = test_app();
    let skel_a = make_skeleton(&[Vec3::ZERO, Vec3::Y]);
    let skel_b = make_skeleton(&[Vec3::ZERO, Vec3::Y * 2.0]);
    let mut cache = MeshCache::new();

    {
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        LSystemMeshBuilder::new().build_cached(&skel_a, &mut cache, &mut meshes);
        LSystemMeshBuilder::new().build_cached(&skel_b, &mut cache, &mut meshes);
    }

    assert_eq!(
        cache.len(),
        2,
        "Different skeletons must produce different fingerprints"
    );
    assert!(cache.contains(&skel_a, 8));
    assert!(cache.contains(&skel_b, 8));
}

#[test]
fn cache_miss_on_resolution_change() {
    let mut app = test_app();
    let skel = make_skeleton(&[Vec3::ZERO, Vec3::Y]);
    let mut cache = MeshCache::new();

    {
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        LSystemMeshBuilder::new()
            .with_resolution(8)
            .build_cached(&skel, &mut cache, &mut meshes);
        LSystemMeshBuilder::new()
            .with_resolution(16)
            .build_cached(&skel, &mut cache, &mut meshes);
    }

    assert_eq!(
        cache.len(),
        2,
        "Different resolutions must yield different cache entries"
    );
}

#[test]
fn clear_drops_all_entries() {
    let mut app = test_app();
    let skel = make_skeleton(&[Vec3::ZERO, Vec3::Y]);
    let mut cache = MeshCache::new();

    {
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        LSystemMeshBuilder::new().build_cached(&skel, &mut cache, &mut meshes);
    }
    assert_eq!(cache.len(), 1);
    cache.clear();
    assert!(cache.is_empty());
}
