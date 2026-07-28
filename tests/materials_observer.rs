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

/// `TextureType` and `TextureConfig` are both `$type`-tagged. Adjacent tagging
/// on the outer enum keeps the inner config in its own map — internal tagging
/// spliced the two together and emitted `$type` twice, which serde then refused
/// to read back with `duplicate field $type`.
#[test]
fn procedural_texture_type_round_trips() {
    let bark = TextureType::all_procedural_kinds()
        .into_iter()
        .find(|c| c.label() == "Bark")
        .expect("Bark is a registered generator");
    let value = TextureType::procedural(bark);

    let json = serde_json::to_string(&value).unwrap();
    assert_eq!(
        json.matches(r#""$type""#).count(),
        2,
        "one tag for the outer variant, one for the inner config: {json}"
    );
    assert!(
        json.starts_with(r#"{"$type":"Procedural","config":{"$type":"Bark""#),
        "unexpected shape: {json}"
    );

    let round_tripped: TextureType = serde_json::from_str(&json).unwrap();
    assert_eq!(round_tripped.kind(), "Bark");
    assert_eq!(serde_json::to_string(&round_tripped).unwrap(), json);
}

/// The round-trip must survive the whole `MaterialSettings` struct, since that
/// is what `.matpalette.json` actually stores.
#[test]
fn material_settings_with_procedural_texture_round_trips() {
    let leaf = TextureType::all_procedural_kinds()
        .into_iter()
        .find(|c| c.label() == "Leaf")
        .expect("Leaf is a registered generator");
    let settings = MaterialSettings {
        base_color: [0.2, 0.8, 0.2],
        texture: TextureType::procedural(leaf),
        ..default()
    };

    let json = serde_json::to_string(&settings).unwrap();
    let back: MaterialSettings = serde_json::from_str(&json).unwrap();
    assert_eq!(back.texture.kind(), "Leaf");
    assert!(back.texture.is_foliage_card());
    assert_eq!(back.base_color, [0.2, 0.8, 0.2]);
}

#[test]
fn payload_free_texture_types_still_serialise_as_bare_tags() {
    assert_eq!(
        serde_json::to_string(&TextureType::None).unwrap(),
        r#"{"$type":"None"}"#
    );
    let grid: TextureType = serde_json::from_str(r#"{"$type":"Grid"}"#).unwrap();
    assert_eq!(grid.kind(), "Grid");
}
