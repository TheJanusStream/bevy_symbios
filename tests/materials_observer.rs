use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy_symbios::materials::*;

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<Image>()
        .init_asset::<StandardMaterial>();
    app
}

fn wire_palette(app: &mut App) -> Handle<StandardMaterial> {
    let mat_handle = {
        let mut mats = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
        mats.add(StandardMaterial::default())
    };
    let mut palette_map = HashMap::new();
    palette_map.insert(0u16, mat_handle.clone());
    app.insert_resource(MaterialPalette {
        materials: palette_map,
        primary_material: mat_handle.clone(),
    });
    app.insert_resource(ProceduralTextures {
        textures: HashMap::new(),
    });
    app.init_resource::<FoliageTextureTasks>();
    app.init_resource::<MaterialSettingsMap>();
    app.add_observer(on_material_settings_changed);
    mat_handle
}

#[test]
fn observer_applies_base_color_in_same_frame() {
    let mut app = test_app();
    let mat_handle = wire_palette(&mut app);

    {
        let mut settings = app.world_mut().resource_mut::<MaterialSettingsMap>();
        settings.settings.entry(0).or_default().base_color = [0.1, 0.2, 0.3];
    }

    app.world_mut().trigger(MaterialSettingsChanged);

    let mats = app.world().resource::<Assets<StandardMaterial>>();
    let mat = mats.get(&mat_handle).expect("material exists");
    let expected = Color::srgb_from_array([0.1, 0.2, 0.3]);
    assert_eq!(mat.base_color, expected);
}

#[test]
fn observer_applies_pbr_properties_in_same_frame() {
    let mut app = test_app();
    let mat_handle = wire_palette(&mut app);

    {
        let mut settings = app.world_mut().resource_mut::<MaterialSettingsMap>();
        let entry = settings.settings.entry(0).or_default();
        entry.roughness = 0.42;
        entry.metallic = 0.73;
        entry.emission_color = [1.0, 0.5, 0.25];
        entry.emission_strength = 2.0;
    }

    app.world_mut().trigger(MaterialSettingsChanged);

    let mats = app.world().resource::<Assets<StandardMaterial>>();
    let mat = mats.get(&mat_handle).expect("material exists");
    assert!((mat.perceptual_roughness - 0.42).abs() < 1e-6);
    assert!((mat.metallic - 0.73).abs() < 1e-6);
    let emissive = mat.emissive;
    assert!(emissive.red > 0.0 && emissive.green > 0.0 && emissive.blue > 0.0);
}

#[test]
fn observer_does_not_run_without_trigger() {
    let mut app = test_app();
    let mat_handle = wire_palette(&mut app);

    let original = {
        let mats = app.world().resource::<Assets<StandardMaterial>>();
        mats.get(&mat_handle).unwrap().base_color
    };

    {
        let mut settings = app.world_mut().resource_mut::<MaterialSettingsMap>();
        settings.settings.entry(0).or_default().base_color = [0.9, 0.0, 0.0];
    }

    // Run a frame WITHOUT triggering — the polling system is gone, so the
    // material must remain unchanged.
    app.update();

    let mats = app.world().resource::<Assets<StandardMaterial>>();
    let after = mats.get(&mat_handle).unwrap().base_color;
    assert_eq!(
        after, original,
        "Without an explicit trigger the observer must not run"
    );
}
