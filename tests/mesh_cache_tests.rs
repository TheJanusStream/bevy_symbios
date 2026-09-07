use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy_symbios::{LSystemMeshBuilder, MeshCache, compute_skeleton_fingerprint};
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

#[test]
fn build_cached_bumps_hits_and_misses() {
    let mut app = test_app();
    let skel = make_skeleton(&[Vec3::ZERO, Vec3::Y, Vec3::Y * 2.0]);
    let mut cache = MeshCache::new();

    {
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        LSystemMeshBuilder::new().build_cached(&skel, &mut cache, &mut meshes);
    }
    assert_eq!(cache.misses(), 1, "first build must record a miss");
    assert_eq!(cache.hits(), 0);

    {
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        LSystemMeshBuilder::new().build_cached(&skel, &mut cache, &mut meshes);
    }
    assert_eq!(cache.misses(), 1, "second build must hit, not miss");
    assert_eq!(cache.hits(), 1);
}

#[test]
fn reset_stats_clears_counters_but_keeps_entries() {
    let mut app = test_app();
    let skel = make_skeleton(&[Vec3::ZERO, Vec3::Y]);
    let mut cache = MeshCache::new();
    {
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        LSystemMeshBuilder::new().build_cached(&skel, &mut cache, &mut meshes);
    }
    assert_eq!(cache.misses(), 1);
    assert_eq!(cache.len(), 1);
    cache.reset_stats();
    assert_eq!(cache.misses(), 0);
    assert_eq!(cache.hits(), 0);
    assert_eq!(cache.len(), 1, "reset_stats must not drop entries");
}

#[test]
fn get_or_insert_with_supports_external_fingerprints() {
    let skel = make_skeleton(&[Vec3::ZERO, Vec3::Y]);
    let fingerprint = compute_skeleton_fingerprint(&skel, 6);
    let mut cache = MeshCache::new();

    let made: HashMap<u16, Handle<Mesh>> = {
        let mut map = HashMap::default();
        map.insert(0u16, Handle::<Mesh>::default());
        map
    };

    // First call: miss — runs the closure.
    let mut closure_calls = 0;
    let h1 = cache.get_or_insert_with(fingerprint, || {
        closure_calls += 1;
        made.clone()
    });
    assert_eq!(closure_calls, 1);
    assert_eq!(cache.misses(), 1);
    assert_eq!(cache.hits(), 0);
    assert_eq!(cache.len(), 1);

    // Second call: hit — closure not invoked.
    let h2 = cache.get_or_insert_with(fingerprint, || {
        closure_calls += 1;
        made.clone()
    });
    assert_eq!(closure_calls, 1, "closure must not run on hit");
    assert_eq!(cache.hits(), 1);
    assert_eq!(h1.get(&0).unwrap().id(), h2.get(&0).unwrap().id());
}

// ---------------------------------------------------------------------------
// Bounding (#38): the cache used to grow without limit, and its own doc said
// so — "call MeshCache::clear periodically ... or manage a coarser eviction
// policy at the application layer". Every consumer that did not is why these
// exist; on wasm a heap that grows is never handed back.
// ---------------------------------------------------------------------------

fn one_entry() -> HashMap<u16, Handle<Mesh>> {
    let mut map = HashMap::default();
    map.insert(0u16, Handle::<Mesh>::default());
    map
}

#[test]
fn the_default_cache_is_bounded() {
    assert_eq!(
        MeshCache::new().capacity(),
        Some(bevy_symbios::DEFAULT_MESH_CACHE_CAPACITY),
        "an unbounded default is what let a re-rolling session grow forever"
    );
    assert_eq!(MeshCache::unbounded().capacity(), None);
    assert_eq!(MeshCache::with_capacity(64).capacity(), Some(64));
    assert_eq!(
        MeshCache::with_capacity(0).capacity(),
        Some(1),
        "a zero ceiling would miss on every lookup while still bookkeeping"
    );
}

#[test]
fn inserting_past_capacity_evicts_and_counts() {
    let mut cache = MeshCache::with_capacity(10);
    for fp in 0..10u64 {
        cache.get_or_insert_with(fp, one_entry);
    }
    assert_eq!(cache.len(), 10, "at the ceiling, nothing has gone yet");
    assert_eq!(cache.evictions(), 0);

    cache.get_or_insert_with(999, one_entry);
    assert!(
        cache.len() <= 10,
        "the eleventh entry must not push the map past its ceiling: {}",
        cache.len()
    );
    assert!(cache.evictions() > 0, "and the drop must be reported");

    // The ceiling holds no matter how much more goes in.
    for fp in 1000..1100u64 {
        cache.get_or_insert_with(fp, one_entry);
    }
    assert!(cache.len() <= 10, "len drifted to {}", cache.len());
}

#[test]
fn eviction_takes_the_least_recently_used_not_the_oldest_inserted() {
    let mut cache = MeshCache::with_capacity(4);
    for fp in 0..4u64 {
        cache.get_or_insert_with(fp, one_entry);
    }
    // Touch the oldest insert so it becomes the newest use.
    cache.get_or_insert_with(0, one_entry);
    assert_eq!(
        cache.hits(),
        1,
        "precondition: that was a hit, not a rebuild"
    );

    // Overflow until the map has been forced to shed entries.
    for fp in 100..110u64 {
        cache.get_or_insert_with(fp, one_entry);
    }
    assert!(cache.evictions() > 0, "precondition: something was evicted");

    // Fingerprint 1 was inserted after 0 but never touched again, so a
    // recency-blind policy would have kept it and dropped 0.
    let mut rebuilt = false;
    cache.get_or_insert_with(1, || {
        rebuilt = true;
        one_entry()
    });
    assert!(
        rebuilt,
        "the untouched entry should have been the one to go"
    );
}

#[test]
fn an_unbounded_cache_still_grows_for_a_caller_that_asks_for_it() {
    let mut cache = MeshCache::unbounded();
    for fp in 0..2000u64 {
        cache.get_or_insert_with(fp, one_entry);
    }
    assert_eq!(cache.len(), 2000);
    assert_eq!(cache.evictions(), 0);
}

#[test]
fn set_capacity_evicts_immediately_and_none_disables_eviction() {
    let mut cache = MeshCache::unbounded();
    for fp in 0..100u64 {
        cache.get_or_insert_with(fp, one_entry);
    }
    assert_eq!(cache.len(), 100);

    cache.set_capacity(Some(10));
    assert!(
        cache.len() <= 10,
        "lowering the ceiling must take effect at once, not on the next insert: {}",
        cache.len()
    );

    cache.set_capacity(None);
    for fp in 200..400u64 {
        cache.get_or_insert_with(fp, one_entry);
    }
    assert!(cache.len() > 10, "None must switch eviction back off");
}

#[test]
fn build_cached_respects_the_ceiling() {
    let mut app = test_app();
    let mut cache = MeshCache::with_capacity(2);

    for i in 0..8 {
        let skel = make_skeleton(&[Vec3::ZERO, Vec3::Y * (i as f32 + 1.0)]);
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        LSystemMeshBuilder::new().build_cached(&skel, &mut cache, &mut meshes);
    }

    assert!(
        cache.len() <= 2,
        "the builder's own path must evict too, not just get_or_insert_with: {}",
        cache.len()
    );
    assert_eq!(cache.misses(), 8, "eight distinct skeletons, eight builds");
    assert!(cache.evictions() > 0);
}

#[test]
fn reset_stats_clears_evictions_too() {
    let mut cache = MeshCache::with_capacity(2);
    for fp in 0..20u64 {
        cache.get_or_insert_with(fp, one_entry);
    }
    assert!(cache.evictions() > 0);
    cache.reset_stats();
    assert_eq!(cache.evictions(), 0);
    assert_eq!(cache.hits(), 0);
    assert_eq!(cache.misses(), 0);
    assert!(!cache.is_empty(), "entries survive a stats reset");
}
