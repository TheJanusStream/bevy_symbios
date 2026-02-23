//! Egui UI helpers for L-System material editing.
//!
//! Provides reusable widgets for editing [`MaterialSettingsMap`] entries,
//! allowing any application with `bevy_egui` to embed material palette controls.

use bevy::platform::collections::HashMap;
use bevy_egui::egui;

use crate::materials::{MaterialSettings, TextureType};

/// Renders a material palette editor widget.
///
/// Shows a collapsible section per material ID with controls for base color,
/// emission, roughness, metallic, texture type, UV scale, and — when a foliage
/// texture type is active — the corresponding [`LeafConfig`], [`TwigConfig`], or
/// [`BarkConfig`] parameters.
///
/// Returns `true` if any material property was modified.
pub fn material_palette_editor(
    ui: &mut egui::Ui,
    settings: &mut HashMap<u8, MaterialSettings>,
) -> bool {
    let mut any_changed = false;

    let mut mat_ids: Vec<u8> = settings.keys().copied().collect();
    mat_ids.sort();

    for mat_id in mat_ids {
        let Some(current) = settings.get(&mat_id).cloned() else {
            continue;
        };

        let mut local_base_color = current.base_color;
        let mut local_emission_color = current.emission_color;
        let mut local_emission_strength = current.emission_strength;
        let mut local_roughness = current.roughness;
        let mut local_metallic = current.metallic;
        let mut local_texture = current.texture;
        let mut local_uv_scale = current.uv_scale;
        let mut local_leaf = current.leaf_config.clone();
        let mut local_twig = current.twig_config.clone();
        let mut local_bark = current.bark_config.clone();

        let mut mat_changed = false;

        ui.collapsing(format!("Material {}", mat_id), |ui| {
            ui.horizontal(|ui| {
                ui.label("Base Color:");
                mat_changed |= ui.color_edit_button_rgb(&mut local_base_color).changed();
            });
            ui.horizontal(|ui| {
                ui.label("Emission:");
                mat_changed |= ui
                    .color_edit_button_rgb(&mut local_emission_color)
                    .changed();
            });
            mat_changed |= ui
                .add(egui::Slider::new(&mut local_emission_strength, 0.0..=10.0).text("Glow"))
                .changed();
            mat_changed |= ui
                .add(egui::Slider::new(&mut local_roughness, 0.0..=1.0).text("Roughness"))
                .changed();
            mat_changed |= ui
                .add(egui::Slider::new(&mut local_metallic, 0.0..=1.0).text("Metallic"))
                .changed();
            mat_changed |= ui
                .add(egui::Slider::new(&mut local_uv_scale, 0.1..=10.0).text("UV Scale"))
                .changed();

            ui.horizontal(|ui| {
                ui.label("Texture:");
                egui::ComboBox::from_id_salt(format!("mat_tex_{}", mat_id))
                    .selected_text(local_texture.name())
                    .show_ui(ui, |ui| {
                        for tex_type in TextureType::ALL {
                            if ui
                                .selectable_label(local_texture == *tex_type, tex_type.name())
                                .clicked()
                            {
                                local_texture = *tex_type;
                                mat_changed = true;
                            }
                        }
                    });
            });

            // Foliage-specific parameter editors
            match local_texture {
                TextureType::Leaf => {
                    mat_changed |= leaf_config_editor(ui, &mut local_leaf, mat_id);
                }
                TextureType::Twig => {
                    mat_changed |= twig_config_editor(ui, &mut local_twig, mat_id);
                }
                TextureType::Bark => {
                    mat_changed |= bark_config_editor(ui, &mut local_bark, mat_id);
                }
                _ => {}
            }
        });

        if mat_changed {
            if let Some(s) = settings.get_mut(&mat_id) {
                s.base_color = local_base_color;
                s.emission_color = local_emission_color;
                s.emission_strength = local_emission_strength;
                s.roughness = local_roughness;
                s.metallic = local_metallic;
                s.texture = local_texture;
                s.uv_scale = local_uv_scale;
                s.leaf_config = local_leaf;
                s.twig_config = local_twig;
                s.bark_config = local_bark;
            }
            any_changed = true;
        }
    }

    any_changed
}

fn leaf_config_editor(
    ui: &mut egui::Ui,
    cfg: &mut bevy_symbios_texture::leaf::LeafConfig,
    mat_id: u8,
) -> bool {
    let mut changed = false;
    ui.collapsing(format!("Leaf Config##lc_{mat_id}"), |ui| {
        ui.horizontal(|ui| {
            ui.label("Base Color:");
            changed |= ui.color_edit_button_rgb(&mut cfg.color_base).changed();
        });
        ui.horizontal(|ui| {
            ui.label("Edge Color:");
            changed |= ui.color_edit_button_rgb(&mut cfg.color_edge).changed();
        });
        changed |= ui
            .add(
                egui::Slider::new(&mut cfg.serration_strength, 0.0..=0.5)
                    .text("Serration"),
            )
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut cfg.vein_angle, 1.0..=5.0).text("Vein Angle"))
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut cfg.vein_count, 2.0..=12.0)
                    .text("Vein Count"),
            )
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut cfg.lobe_count, 0.0..=6.0)
                    .text("Lobe Count"),
            )
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut cfg.lobe_depth, 0.0..=1.0)
                    .text("Lobe Depth"),
            )
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut cfg.micro_detail, 0.0..=1.0)
                    .text("Micro Detail"),
            )
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut cfg.normal_strength, 0.0..=8.0)
                    .text("Normal Strength"),
            )
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut cfg.petiole_length, 0.0..=0.3)
                    .text("Petiole"),
            )
            .changed();
    });
    changed
}

fn twig_config_editor(
    ui: &mut egui::Ui,
    cfg: &mut bevy_symbios_texture::twig::TwigConfig,
    mat_id: u8,
) -> bool {
    let mut changed = false;
    ui.collapsing(format!("Twig Config##tc_{mat_id}"), |ui| {
        ui.horizontal(|ui| {
            ui.label("Stem Color:");
            changed |= ui.color_edit_button_rgb(&mut cfg.stem_color).changed();
        });
        changed |= ui
            .add(
                egui::Slider::new(&mut cfg.stem_half_width, 0.005..=0.05)
                    .text("Stem Width"),
            )
            .changed();
        let mut leaf_pairs = cfg.leaf_pairs;
        changed |= ui
            .add(egui::Slider::new(&mut leaf_pairs, 1..=8).text("Leaf Pairs"))
            .changed();
        cfg.leaf_pairs = leaf_pairs;
        changed |= ui
            .add(
                egui::Slider::new(&mut cfg.leaf_angle, 0.0..=std::f64::consts::PI)
                    .text("Leaf Angle"),
            )
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut cfg.leaf_scale, 0.1..=0.6)
                    .text("Leaf Scale"),
            )
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut cfg.stem_curve, 0.0..=0.15)
                    .text("Stem Curve"),
            )
            .changed();
        let mut sympodial = cfg.sympodial;
        changed |= ui.checkbox(&mut sympodial, "Sympodial").changed();
        cfg.sympodial = sympodial;

        // Inline leaf appearance sub-section
        changed |= leaf_config_editor(ui, &mut cfg.leaf, mat_id + 128);
    });
    changed
}

fn bark_config_editor(
    ui: &mut egui::Ui,
    cfg: &mut bevy_symbios_texture::bark::BarkConfig,
    mat_id: u8,
) -> bool {
    let mut changed = false;
    ui.collapsing(format!("Bark Config##bc_{mat_id}"), |ui| {
        ui.horizontal(|ui| {
            ui.label("Light Color:");
            changed |= ui.color_edit_button_rgb(&mut cfg.color_light).changed();
        });
        ui.horizontal(|ui| {
            ui.label("Dark Color:");
            changed |= ui.color_edit_button_rgb(&mut cfg.color_dark).changed();
        });
        changed |= ui
            .add(egui::Slider::new(&mut cfg.scale, 1.0..=12.0).text("Scale"))
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut cfg.warp_u, 0.0..=0.5).text("Warp H"))
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut cfg.warp_v, 0.0..=1.5).text("Warp V"))
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut cfg.normal_strength, 0.0..=8.0)
                    .text("Normal Strength"),
            )
            .changed();
        let mut octaves = cfg.octaves;
        changed |= ui
            .add(egui::Slider::new(&mut octaves, 1..=8).text("Octaves"))
            .changed();
        cfg.octaves = octaves;
    });
    changed
}
