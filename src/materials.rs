//! Material system for L-System visualization.
//!
//! Provides configurable PBR materials with procedural texture support,
//! designed for the palette-first material workflow where each material slot
//! (identified by `u8` ID) maps to a Bevy [`StandardMaterial`].
//!
//! # Workflow
//!
//! 1. Add [`setup_material_assets`] as a `Startup` system to create textures and palette.
//! 2. Insert [`MaterialSettingsMap`] as a resource (or use `init_resource`).
//! 3. Add [`sync_material_properties`] to your `Update` schedule to keep materials in sync.
//! 4. Mutate [`MaterialSettingsMap`] from your UI or game logic; the sync system detects
//!    changes automatically via Bevy's change detection.

use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::math::{Affine2, Vec2};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Available procedural texture types for materials.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextureType {
    #[default]
    None,
    Grid,
    Noise,
    Checker,
}

impl TextureType {
    pub const ALL: &'static [TextureType] = &[
        TextureType::None,
        TextureType::Grid,
        TextureType::Noise,
        TextureType::Checker,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            TextureType::None => "None",
            TextureType::Grid => "Grid",
            TextureType::Noise => "Noise",
            TextureType::Checker => "Checker",
        }
    }
}

/// Per-material PBR settings for UI editing and export.
#[derive(Clone)]
pub struct MaterialSettings {
    pub base_color: [f32; 3],
    pub emission_color: [f32; 3],
    pub emission_strength: f32,
    pub roughness: f32,
    pub metallic: f32,
    pub texture: TextureType,
    pub uv_scale: f32,
}

impl Default for MaterialSettings {
    fn default() -> Self {
        Self {
            base_color: [1.0, 1.0, 1.0],
            emission_color: [0.0, 0.0, 0.0],
            emission_strength: 0.0,
            roughness: 0.5,
            metallic: 0.0,
            texture: TextureType::None,
            uv_scale: 1.0,
        }
    }
}

/// Resource holding editable settings for each material ID.
#[derive(Resource)]
pub struct MaterialSettingsMap {
    pub settings: HashMap<u8, MaterialSettings>,
}

impl Default for MaterialSettingsMap {
    fn default() -> Self {
        let mut settings = HashMap::new();

        settings.insert(
            0,
            MaterialSettings {
                base_color: [0.2, 0.8, 0.2],
                emission_color: [0.5, 1.0, 0.5],
                emission_strength: 0.0,
                roughness: 0.2,
                metallic: 0.8,
                texture: TextureType::None,
                uv_scale: 1.0,
            },
        );

        settings.insert(
            1,
            MaterialSettings {
                base_color: [1.0, 1.0, 1.0],
                emission_color: [0.0, 1.0, 1.0],
                emission_strength: 2.0,
                roughness: 0.1,
                metallic: 0.0,
                texture: TextureType::None,
                uv_scale: 1.0,
            },
        );

        settings.insert(
            2,
            MaterialSettings {
                base_color: [0.5, 0.5, 0.5],
                emission_color: [0.0, 0.0, 0.0],
                emission_strength: 0.0,
                roughness: 0.9,
                metallic: 0.0,
                texture: TextureType::None,
                uv_scale: 1.0,
            },
        );

        Self { settings }
    }
}

/// Stores material handles mapped by material ID.
#[derive(Resource)]
pub struct MaterialPalette {
    pub materials: HashMap<u8, Handle<StandardMaterial>>,
    /// Default material handle used as fallback.
    pub primary_material: Handle<StandardMaterial>,
}

/// Stores procedural texture handles for material customization.
#[derive(Resource)]
pub struct ProceduralTextures {
    pub textures: HashMap<TextureType, Handle<Image>>,
}

// ---------------------------------------------------------------------------
// Procedural texture generators
// ---------------------------------------------------------------------------

fn generate_grid_texture(size: u32, line_width: u32) -> Vec<u8> {
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let on_grid = (x % (size / 8) < line_width) || (y % (size / 8) < line_width);
            let val = if on_grid { 255 } else { 180 };
            data.extend_from_slice(&[val, val, val, 255]);
        }
    }
    data
}

fn generate_noise_texture(size: u32, seed: u32) -> Vec<u8> {
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let hash = ((x.wrapping_mul(374761393))
                ^ (y.wrapping_mul(668265263))
                ^ seed.wrapping_mul(1013904223))
            .wrapping_mul(1664525);
            let val = ((hash >> 24) & 0xFF) as u8;
            let blended = 128 + (val as i32 - 128) / 2;
            data.extend_from_slice(&[blended as u8, blended as u8, blended as u8, 255]);
        }
    }
    data
}

fn generate_checker_texture(size: u32, tile_size: u32) -> Vec<u8> {
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let checker = ((x / tile_size) + (y / tile_size)).is_multiple_of(2);
            let val = if checker { 220 } else { 160 };
            data.extend_from_slice(&[val, val, val, 255]);
        }
    }
    data
}

fn create_image(data: Vec<u8>, size: u32) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        default(),
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..default()
    });
    image
}

// ---------------------------------------------------------------------------
// Bevy systems
// ---------------------------------------------------------------------------

/// Startup system that creates procedural textures and a default material palette.
///
/// Inserts [`ProceduralTextures`] and [`MaterialPalette`] resources.
/// Pair with [`sync_material_properties`] in your update schedule to keep
/// materials in sync with [`MaterialSettingsMap`].
pub fn setup_material_assets(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    const TEX_SIZE: u32 = 256;
    let mut proc_textures = HashMap::new();

    proc_textures.insert(
        TextureType::Grid,
        images.add(create_image(generate_grid_texture(TEX_SIZE, 2), TEX_SIZE)),
    );
    proc_textures.insert(
        TextureType::Noise,
        images.add(create_image(generate_noise_texture(TEX_SIZE, 42), TEX_SIZE)),
    );
    proc_textures.insert(
        TextureType::Checker,
        images.add(create_image(
            generate_checker_texture(TEX_SIZE, 32),
            TEX_SIZE,
        )),
    );

    commands.insert_resource(ProceduralTextures {
        textures: proc_textures,
    });

    let mut palette = HashMap::new();

    let mat_0 = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.2,
        metallic: 0.8,
        reflectance: 0.5,
        ..default()
    });
    palette.insert(0, mat_0.clone());

    let mat_1 = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.1,
        metallic: 0.0,
        emissive: LinearRgba::rgb(0.0, 2.0, 2.0),
        ..default()
    });
    palette.insert(1, mat_1);

    let mat_2 = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.5, 0.5),
        perceptual_roughness: 0.9,
        metallic: 0.0,
        ..default()
    });
    palette.insert(2, mat_2);

    commands.insert_resource(MaterialPalette {
        materials: palette,
        primary_material: mat_0,
    });
}

/// Update system that synchronizes [`MaterialSettingsMap`] values to the
/// [`MaterialPalette`]'s `StandardMaterial` handles.
///
/// Uses Bevy's change detection — only processes when [`MaterialSettingsMap`]
/// has been mutated since the last run. Automatically creates new material
/// handles for IDs that don't yet exist in the palette.
pub fn sync_material_properties(
    material_settings: Res<MaterialSettingsMap>,
    mut palette: ResMut<MaterialPalette>,
    proc_textures: Res<ProceduralTextures>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !material_settings.is_changed() {
        return;
    }

    for (mat_id, settings) in &material_settings.settings {
        let handle = palette
            .materials
            .entry(*mat_id)
            .or_insert_with(|| materials.add(StandardMaterial::default()))
            .clone();
        let Some(mat) = materials.get_mut(&handle) else {
            continue;
        };

        mat.base_color = Color::srgb_from_array(settings.base_color);
        mat.perceptual_roughness = settings.roughness;
        mat.metallic = settings.metallic;

        let emission_linear = Color::srgb_from_array(settings.emission_color).to_linear()
            * settings.emission_strength;
        mat.emissive = emission_linear;

        mat.base_color_texture = match settings.texture {
            TextureType::None => None,
            other => proc_textures.textures.get(&other).cloned(),
        };

        mat.uv_transform = Affine2::from_scale(Vec2::splat(settings.uv_scale));
    }
}
