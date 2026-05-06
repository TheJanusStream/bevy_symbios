//! Egui UI helpers for L-System material editing.
//!
//! Provides reusable widgets for editing [`MaterialSettingsMap`] entries,
//! allowing any application with `bevy_egui` to embed material palette controls.
//!
//! Texture-specific config editors ([`LeafConfig`], [`TwigConfig`], [`BarkConfig`])
//! are re-exported from [`bevy_symbios_texture::ui`].

use bevy::platform::collections::HashMap;
use bevy_egui::egui;

use crate::materials::{MaterialSettings, TextureType};

pub use bevy_symbios_texture::ui::{
    bark_config_editor, leaf_config_editor, slider_debounced, twig_config_editor,
};

/// Renders a material palette editor widget.
///
/// Shows a collapsible section per material ID with controls for base color,
/// emission, roughness, metallic, texture type, UV scale, and — when a foliage
/// texture type is active — the corresponding [`LeafConfig`], [`TwigConfig`], or
/// [`BarkConfig`] parameters.
///
/// Returns `true` if any material property was modified in a way that requires
/// texture regeneration (PBR changes apply immediately; foliage config changes
/// commit only when the slider drag ends, preventing excessive re-generation).
///
/// The caller is responsible for writing back to `settings` only when needed.
/// Config values are always written back to prevent visual slider snap-back during
/// drag; the return value indicates whether texture regeneration is needed.
pub fn material_palette_editor(
    ui: &mut egui::Ui,
    settings: &mut HashMap<u16, MaterialSettings>,
) -> bool {
    let mut any_regen = false;

    let mut mat_ids: Vec<u16> = settings.keys().copied().collect();
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

        // mat_regen: triggers texture regeneration (set_changed for sync_material_properties)
        // mat_writeback: slider value changed visually but regen not yet needed (prevents snap-back)
        let mut mat_regen = false;
        let mut mat_writeback = false;

        ui.collapsing(format!("Material {}", mat_id), |ui| {
            // PBR properties: instant regen on any change.
            ui.horizontal(|ui| {
                ui.label("Base Color:");
                mat_regen |= ui.color_edit_button_rgb(&mut local_base_color).changed();
            });
            ui.horizontal(|ui| {
                ui.label("Emission:");
                mat_regen |= ui
                    .color_edit_button_rgb(&mut local_emission_color)
                    .changed();
            });
            mat_regen |= ui
                .add(egui::Slider::new(&mut local_emission_strength, 0.0..=10.0).text("Glow"))
                .changed();
            mat_regen |= ui
                .add(egui::Slider::new(&mut local_roughness, 0.0..=1.0).text("Roughness"))
                .changed();
            mat_regen |= ui
                .add(egui::Slider::new(&mut local_metallic, 0.0..=1.0).text("Metallic"))
                .changed();
            mat_regen |= ui
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
                                mat_regen = true;
                            }
                        }
                    });
            });

            // Foliage-specific parameter editors from bevy_symbios_texture::ui.
            // These return (writeback, regen): writeback is true during drag (prevents snap-back),
            // regen is true only when the drag commits (drag_stopped or non-drag change).
            match local_texture {
                TextureType::Leaf => {
                    let (wb, regen) =
                        leaf_config_editor(ui, &mut local_leaf, egui::Id::new(mat_id));
                    mat_writeback |= wb;
                    mat_regen |= regen;
                }
                TextureType::Twig => {
                    let (wb, regen) =
                        twig_config_editor(ui, &mut local_twig, egui::Id::new(mat_id));
                    mat_writeback |= wb;
                    mat_regen |= regen;
                }
                TextureType::Bark => {
                    let (wb, regen) =
                        bark_config_editor(ui, &mut local_bark, egui::Id::new(mat_id));
                    mat_writeback |= wb;
                    mat_regen |= regen;
                }
                _ => {}
            }
        });

        // Always write back when any widget changed (including mid-drag) to prevent
        // slider snap-back on the next frame.
        if mat_regen || mat_writeback {
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
        }

        if mat_regen {
            any_regen = true;
        }
    }

    any_regen
}
