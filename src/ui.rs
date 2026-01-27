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
/// emission, roughness, metallic, texture type, and UV scale.
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
                .add(
                    egui::Slider::new(&mut local_emission_strength, 0.0..=10.0).text("Glow"),
                )
                .changed();
            mat_changed |= ui
                .add(
                    egui::Slider::new(&mut local_roughness, 0.0..=1.0).text("Roughness"),
                )
                .changed();
            mat_changed |= ui
                .add(
                    egui::Slider::new(&mut local_metallic, 0.0..=1.0).text("Metallic"),
                )
                .changed();
            mat_changed |= ui
                .add(
                    egui::Slider::new(&mut local_uv_scale, 0.1..=10.0).text("UV Scale"),
                )
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
            }
            any_changed = true;
        }
    }

    any_changed
}
