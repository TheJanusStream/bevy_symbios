//! Egui UI helpers for L-System material editing.
//!
//! Provides reusable widgets for editing [`crate::materials::MaterialSettingsMap`]
//! entries, allowing any application with `bevy_egui` to embed material palette controls.
//!
//! Texture-specific config editors for **every procedural texture variant**
//! in the `bevy_symbios_texture` registry are re-exported from
//! [`bevy_symbios_texture::ui`] and dispatched live by
//! [`material_palette_editor`] (via [`texture_config_editor`]) under a
//! collapsible "Texture parameters" subsection per material slot. Generators
//! added upstream are picked up automatically — the dispatch is exhaustive
//! in the texture crate itself.

use bevy::platform::collections::HashMap;
use bevy_egui::egui;
use bevy_symbios_texture::TextureConfig;

use crate::materials::{MaterialSettings, TextureType};

pub use bevy_symbios_texture::ui::*;

/// Renders a material palette editor widget.
///
/// Shows a collapsible section per material ID with controls for base color,
/// emission, roughness, metallic, texture type, UV scale, and — for any
/// `Procedural(TextureConfig)` selection — a nested **"Texture parameters"**
/// subsection wrapping the per-variant editor for the active config (any
/// generator in the [`bevy_symbios_texture`] registry). The nested collapsible
/// keeps PBR sliders visually separate from the (sometimes lengthy)
/// generator-specific sliders.
///
/// Returns `true` if any material property was modified in a way that requires
/// texture regeneration. PBR changes (base color, emission, roughness,
/// metallic, UV scale) apply immediately; per-variant slider changes commit
/// only when the drag ends, preventing excessive re-generation while the
/// user scrubs.
///
/// Config values are always written back when any widget changes (including
/// mid-drag) to prevent visual slider snap-back; the return value indicates
/// whether texture regeneration is needed.
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
        let mut local_texture = current.texture.clone();
        let mut local_uv_scale = current.uv_scale;

        // mat_regen: caller should fire MaterialSettingsChanged so on_material_settings_changed re-applies the palette
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
                        for kind in TextureType::all_kinds() {
                            let selected = local_texture.kind() == kind.kind();
                            if ui.selectable_label(selected, kind.name()).clicked() && !selected {
                                local_texture = kind;
                                mat_regen = true;
                            }
                        }
                    });
            });

            // Per-variant config editor dispatched by
            // bevy_symbios_texture::ui::texture_config_editor, wrapped in its
            // own collapsible subsection so the (sometimes 10+) sliders for a
            // brick or thatch generator do not crowd the PBR controls.
            // Editors return (writeback, regen): writeback is true during drag
            // (prevents snap-back), regen is true only when the drag commits
            // (drag_stopped or non-drag change).
            //
            // Header captured before the mutable borrow of `local_texture` is
            // taken — `name()` would otherwise alias the inner mutable borrow.
            let header = format!("Texture parameters ({})", local_texture.name());
            if let TextureType::Procedural(cfg) = &mut local_texture
                && !matches!(**cfg, TextureConfig::None)
            {
                let id = egui::Id::new(mat_id);
                ui.collapsing(header, |ui| {
                    let (wb, regen) = texture_config_editor(ui, cfg, id);
                    mat_writeback |= wb;
                    mat_regen |= regen;
                });
            }
        });

        // Always write back when any widget changed (including mid-drag) to prevent
        // slider snap-back on the next frame.
        if (mat_regen || mat_writeback)
            && let Some(s) = settings.get_mut(&mat_id)
        {
            s.base_color = local_base_color;
            s.emission_color = local_emission_color;
            s.emission_strength = local_emission_strength;
            s.roughness = local_roughness;
            s.metallic = local_metallic;
            s.texture = local_texture;
            s.uv_scale = local_uv_scale;
        }

        if mat_regen {
            any_regen = true;
        }
    }

    any_regen
}
