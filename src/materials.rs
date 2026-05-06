//! Material system for L-System visualization.
//!
//! Provides configurable PBR materials with procedural texture support,
//! designed for the palette-first material workflow where each material slot
//! (identified by `u16` ID) maps to a Bevy [`StandardMaterial`].
//!
//! # Workflow
//!
//! 1. Add [`setup_material_assets`] as a `Startup` system to create textures and palette.
//! 2. Insert [`MaterialSettingsMap`] as a resource (or use `init_resource`).
//! 3. Add [`sync_material_properties`] and [`apply_foliage_textures`] to your `Update` schedule.
//! 4. Mutate [`MaterialSettingsMap`] from your UI or game logic; the sync system detects
//!    changes automatically via Bevy's change detection.

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::math::{Affine2, Vec2};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, Face, TextureDimension, TextureFormat};
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, futures_lite::future};
use bevy_symbios_texture::bark::{BarkConfig, BarkGenerator};
use bevy_symbios_texture::generator::{TextureError, TextureGenerator, TextureMap};
use bevy_symbios_texture::leaf::{LeafConfig, LeafGenerator};
use bevy_symbios_texture::twig::{TwigConfig, TwigGenerator};
use bevy_symbios_texture::{GeneratedHandles, map_to_images, map_to_images_card};

pub use bevy_symbios_texture::bark::BarkConfig as BarkTexConfig;
pub use bevy_symbios_texture::leaf::LeafConfig as LeafTexConfig;
pub use bevy_symbios_texture::twig::TwigConfig as TwigTexConfig;

/// Available procedural texture types for materials.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum TextureType {
    #[default]
    None,
    Grid,
    Noise,
    Checker,
    /// Procedural leaf silhouette card (alpha-masked, clamp-to-edge).
    Leaf,
    /// Procedural twig composite card (alpha-masked, clamp-to-edge).
    Twig,
    /// Procedural tileable bark texture.
    Bark,
}

impl TextureType {
    pub const ALL: &'static [TextureType] = &[
        TextureType::None,
        TextureType::Grid,
        TextureType::Noise,
        TextureType::Checker,
        TextureType::Leaf,
        TextureType::Twig,
        TextureType::Bark,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            TextureType::None => "None",
            TextureType::Grid => "Grid",
            TextureType::Noise => "Noise",
            TextureType::Checker => "Checker",
            TextureType::Leaf => "Leaf",
            TextureType::Twig => "Twig",
            TextureType::Bark => "Bark",
        }
    }

    /// Returns `true` for foliage card types that need alpha-mask rendering.
    pub fn is_foliage_card(self) -> bool {
        matches!(self, TextureType::Leaf | TextureType::Twig)
    }
}

/// Per-material PBR settings for UI editing and export.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MaterialSettings {
    pub base_color: [f32; 3],
    pub emission_color: [f32; 3],
    pub emission_strength: f32,
    pub roughness: f32,
    pub metallic: f32,
    pub texture: TextureType,
    pub uv_scale: f32,
    /// Config for procedural leaf texture (active when `texture == TextureType::Leaf`).
    pub leaf_config: LeafConfig,
    /// Config for procedural twig texture (active when `texture == TextureType::Twig`).
    pub twig_config: TwigConfig,
    /// Config for procedural bark texture (active when `texture == TextureType::Bark`).
    pub bark_config: BarkConfig,
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
            leaf_config: LeafConfig::default(),
            twig_config: TwigConfig::default(),
            bark_config: BarkConfig::default(),
        }
    }
}

/// Resource holding editable settings for each material ID.
#[derive(Resource)]
pub struct MaterialSettingsMap {
    pub settings: HashMap<u16, MaterialSettings>,
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
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
            },
        );

        Self { settings }
    }
}

/// Stores material handles mapped by material ID.
#[derive(Resource)]
pub struct MaterialPalette {
    pub materials: HashMap<u16, Handle<StandardMaterial>>,
    /// Default material handle used as fallback.
    pub primary_material: Handle<StandardMaterial>,
}

/// Stores procedural texture handles for simple (non-async) material customization.
#[derive(Resource)]
pub struct ProceduralTextures {
    pub textures: HashMap<TextureType, Handle<Image>>,
}

/// Tracks in-flight async foliage texture generation tasks.
///
/// Keyed by material ID. Each entry holds the background task and whether the
/// result should be uploaded with card (clamp-to-edge) or tile (repeat) samplers.
///
/// `pending_type` records the texture type of the currently running task so that
/// `sync_material_properties` can skip re-spawning while a same-type task is
/// already in flight.  It is cleared when the task completes so the next config
/// change triggers a fresh generation.
#[derive(Resource, Default)]
pub struct FoliageTextureTasks {
    tasks: HashMap<u16, (Task<Result<TextureMap, TextureError>>, bool)>,
    /// Texture type of the currently pending task per material ID.
    pending_type: HashMap<u16, TextureType>,
}

// ---------------------------------------------------------------------------
// Procedural texture generators (simple/inline)
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
        RenderAssetUsages::default(),
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
/// Inserts [`ProceduralTextures`], [`MaterialPalette`], and [`FoliageTextureTasks`] resources.
/// Pair with [`sync_material_properties`] and [`apply_foliage_textures`] in your update schedule.
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
    commands.insert_resource(FoliageTextureTasks::default());

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

/// Event fired by callers after they mutate [`MaterialSettingsMap`].
///
/// Triggers [`on_material_settings_changed`], which writes the latest settings
/// onto the [`MaterialPalette`]'s `StandardMaterial` handles. Use
/// `commands.trigger(MaterialSettingsChanged)` after editing settings — there
/// is no implicit `is_changed()` polling.
#[derive(Event, Default)]
pub struct MaterialSettingsChanged;

/// Observer that synchronizes [`MaterialSettingsMap`] values to the
/// [`MaterialPalette`]'s `StandardMaterial` handles whenever
/// [`MaterialSettingsChanged`] is triggered.
///
/// For simple procedural textures (Grid, Noise, Checker) the handle is applied
/// immediately.  For foliage textures (Leaf, Twig, Bark) an async generation
/// task is spawned into [`FoliageTextureTasks`]; call [`apply_foliage_textures`]
/// each frame to collect results and update the material.
///
/// Register with `app.add_observer(on_material_settings_changed)`.
pub fn on_material_settings_changed(
    _: On<MaterialSettingsChanged>,
    material_settings: Res<MaterialSettingsMap>,
    mut palette: ResMut<MaterialPalette>,
    proc_textures: Res<ProceduralTextures>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut foliage_tasks: ResMut<FoliageTextureTasks>,
) {
    let pool = AsyncComputeTaskPool::get();

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

        mat.uv_transform = Affine2::from_scale(Vec2::splat(settings.uv_scale));

        match settings.texture {
            TextureType::None => {
                mat.base_color_texture = None;
                mat.normal_map_texture = None;
                mat.metallic_roughness_texture = None;
                mat.alpha_mode = AlphaMode::Opaque;
                mat.double_sided = false;
                mat.cull_mode = Some(Face::Back);
                foliage_tasks.tasks.remove(mat_id);
                foliage_tasks.pending_type.remove(mat_id);
            }
            TextureType::Grid | TextureType::Noise | TextureType::Checker => {
                mat.base_color_texture = proc_textures.textures.get(&settings.texture).cloned();
                mat.normal_map_texture = None;
                mat.metallic_roughness_texture = None;
                mat.alpha_mode = AlphaMode::Opaque;
                mat.double_sided = false;
                mat.cull_mode = Some(Face::Back);
                foliage_tasks.tasks.remove(mat_id);
                foliage_tasks.pending_type.remove(mat_id);
            }
            TextureType::Leaf => {
                // Apply alpha-mask settings immediately; texture arrives async.
                mat.alpha_mode = AlphaMode::Mask(0.5);
                mat.double_sided = true;
                mat.cull_mode = None;

                // Only spawn if no same-type task is already in flight.
                if foliage_tasks.pending_type.get(mat_id) != Some(&TextureType::Leaf) {
                    let config = settings.leaf_config.clone();
                    let task =
                        pool.spawn(async move { LeafGenerator::new(config).generate(512, 512) });
                    foliage_tasks.tasks.insert(*mat_id, (task, true));
                    foliage_tasks
                        .pending_type
                        .insert(*mat_id, TextureType::Leaf);
                }
            }
            TextureType::Twig => {
                mat.alpha_mode = AlphaMode::Mask(0.5);
                mat.double_sided = true;
                mat.cull_mode = None;

                if foliage_tasks.pending_type.get(mat_id) != Some(&TextureType::Twig) {
                    let config = settings.twig_config.clone();
                    let task =
                        pool.spawn(async move { TwigGenerator::new(config).generate(512, 512) });
                    foliage_tasks.tasks.insert(*mat_id, (task, true));
                    foliage_tasks
                        .pending_type
                        .insert(*mat_id, TextureType::Twig);
                }
            }
            TextureType::Bark => {
                mat.alpha_mode = AlphaMode::Opaque;
                mat.double_sided = false;
                mat.cull_mode = Some(Face::Back);

                if foliage_tasks.pending_type.get(mat_id) != Some(&TextureType::Bark) {
                    let config = settings.bark_config.clone();
                    let task =
                        pool.spawn(async move { BarkGenerator::new(config).generate(512, 512) });
                    foliage_tasks.tasks.insert(*mat_id, (task, false));
                    foliage_tasks
                        .pending_type
                        .insert(*mat_id, TextureType::Bark);
                }
            }
        }
    }
}

/// Update system that polls completed foliage texture tasks and applies the
/// generated handles to the corresponding [`StandardMaterial`].
///
/// Independent of [`on_material_settings_changed`] — runs every frame to
/// drain task results regardless of whether settings changed.
///
/// Marks [`MaterialPalette`] as changed whenever any texture is applied so that
/// downstream systems (e.g. prop-material caches) know to refresh their copies.
pub fn apply_foliage_textures(
    mut foliage_tasks: ResMut<FoliageTextureTasks>,
    mut palette: ResMut<MaterialPalette>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let mut finished = Vec::new();

    for (mat_id, (task, is_card)) in foliage_tasks.tasks.iter_mut() {
        if let Some(result) = block_on(future::poll_once(task)) {
            finished.push((*mat_id, result, *is_card));
        }
    }

    if finished.is_empty() {
        return;
    }

    for (mat_id, result, is_card) in finished {
        foliage_tasks.tasks.remove(&mat_id);
        // Clear pending marker so the next config change can spawn a fresh task.
        foliage_tasks.pending_type.remove(&mat_id);

        let map = match result {
            Ok(m) => m,
            Err(e) => {
                bevy::log::error!("Foliage texture generation failed for mat {mat_id}: {e}");
                continue;
            }
        };

        let handles: GeneratedHandles = if is_card {
            map_to_images_card(map, &mut images)
        } else {
            map_to_images(map, &mut images)
        };

        let Some(mat_handle) = palette.bypass_change_detection().materials.get(&mat_id) else {
            continue;
        };
        let Some(mat) = materials.get_mut(mat_handle) else {
            continue;
        };

        mat.base_color_texture = Some(handles.albedo);
        mat.normal_map_texture = Some(handles.normal);
        mat.metallic_roughness_texture = Some(handles.roughness);
    }

    // Signal downstream systems (like prop-material caches) that the underlying
    // palette material assets have changed, without modifying the palette itself.
    palette.set_changed();
}
