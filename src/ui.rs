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
/// Returns `true` if any material property was modified in a way that requires
/// texture regeneration (PBR changes apply immediately; foliage config changes
/// commit only when the slider drag ends, preventing excessive re-generation).
///
/// The caller is responsible for writing back to `settings` only when needed.
/// Config values are always written back to prevent visual slider snap-back during
/// drag; the return value indicates whether texture regeneration is needed.
pub fn material_palette_editor(
    ui: &mut egui::Ui,
    settings: &mut HashMap<u8, MaterialSettings>,
) -> bool {
    let mut any_regen = false;

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

            // Foliage-specific parameter editors.
            // These return (writeback, regen): writeback is true during drag (prevents snap-back),
            // regen is true only when the drag commits (drag_stopped or non-drag change).
            match local_texture {
                TextureType::Leaf => {
                    let (wb, regen) = leaf_config_editor(ui, &mut local_leaf, mat_id);
                    mat_writeback |= wb;
                    mat_regen |= regen;
                }
                TextureType::Twig => {
                    let (wb, regen) = twig_config_editor(ui, &mut local_twig, mat_id);
                    mat_writeback |= wb;
                    mat_regen |= regen;
                }
                TextureType::Bark => {
                    let (wb, regen) = bark_config_editor(ui, &mut local_bark, mat_id);
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

/// Returns `(writeback, regen)`:
/// - `writeback`: a slider value changed during drag (write back to prevent snap-back)
/// - `regen`: a value committed (drag ended or non-drag change) — texture should regenerate
fn leaf_config_editor(
    ui: &mut egui::Ui,
    cfg: &mut bevy_symbios_texture::leaf::LeafConfig,
    mat_id: u8,
) -> (bool, bool) {
    let mut writeback = false;
    let mut regen = false;
    ui.collapsing(format!("Leaf Config##lc_{mat_id}"), |ui| {
        // Color pickers: instant regen.
        ui.horizontal(|ui| {
            ui.label("Base Color:");
            let r = ui.color_edit_button_rgb(&mut cfg.color_base);
            writeback |= r.changed();
            regen |= r.changed();
        });
        ui.horizontal(|ui| {
            ui.label("Edge Color:");
            let r = ui.color_edit_button_rgb(&mut cfg.color_edge);
            writeback |= r.changed();
            regen |= r.changed();
        });
        // Sliders: write back on any change, regen only when committed.
        slider_debounced(
            ui,
            egui::Slider::new(&mut cfg.serration_strength, 0.0..=0.5).text("Serration"),
            &mut writeback,
            &mut regen,
        );
        slider_debounced(
            ui,
            egui::Slider::new(&mut cfg.vein_angle, 1.0..=5.0).text("Vein Angle"),
            &mut writeback,
            &mut regen,
        );
        slider_debounced(
            ui,
            egui::Slider::new(&mut cfg.vein_count, 2.0..=12.0).text("Vein Count"),
            &mut writeback,
            &mut regen,
        );
        slider_debounced(
            ui,
            egui::Slider::new(&mut cfg.lobe_count, 0.0..=6.0).text("Lobe Count"),
            &mut writeback,
            &mut regen,
        );
        slider_debounced(
            ui,
            egui::Slider::new(&mut cfg.lobe_depth, 0.0..=1.0).text("Lobe Depth"),
            &mut writeback,
            &mut regen,
        );
        slider_debounced(
            ui,
            egui::Slider::new(&mut cfg.micro_detail, 0.0..=1.0).text("Micro Detail"),
            &mut writeback,
            &mut regen,
        );
        slider_debounced(
            ui,
            egui::Slider::new(&mut cfg.normal_strength, 0.0..=8.0).text("Normal Strength"),
            &mut writeback,
            &mut regen,
        );
        slider_debounced(
            ui,
            egui::Slider::new(&mut cfg.petiole_length, 0.0..=0.3).text("Petiole"),
            &mut writeback,
            &mut regen,
        );
    });
    (writeback, regen)
}

fn twig_config_editor(
    ui: &mut egui::Ui,
    cfg: &mut bevy_symbios_texture::twig::TwigConfig,
    mat_id: u8,
) -> (bool, bool) {
    let mut writeback = false;
    let mut regen = false;
    ui.collapsing(format!("Twig Config##tc_{mat_id}"), |ui| {
        ui.horizontal(|ui| {
            ui.label("Stem Color:");
            let r = ui.color_edit_button_rgb(&mut cfg.stem_color);
            writeback |= r.changed();
            regen |= r.changed();
        });
        slider_debounced(
            ui,
            egui::Slider::new(&mut cfg.stem_half_width, 0.005..=0.05).text("Stem Width"),
            &mut writeback,
            &mut regen,
        );
        // leaf_pairs is usize — need local copy for the slider then write back.
        let mut leaf_pairs = cfg.leaf_pairs;
        let r = ui.add(egui::Slider::new(&mut leaf_pairs, 1..=8).text("Leaf Pairs"));
        writeback |= r.changed();
        regen |= r.drag_stopped() || (r.changed() && !r.dragged());
        cfg.leaf_pairs = leaf_pairs;

        slider_debounced(
            ui,
            egui::Slider::new(&mut cfg.leaf_angle, 0.0..=std::f64::consts::PI).text("Leaf Angle"),
            &mut writeback,
            &mut regen,
        );
        slider_debounced(
            ui,
            egui::Slider::new(&mut cfg.leaf_scale, 0.1..=0.6).text("Leaf Scale"),
            &mut writeback,
            &mut regen,
        );
        slider_debounced(
            ui,
            egui::Slider::new(&mut cfg.stem_curve, 0.0..=0.15).text("Stem Curve"),
            &mut writeback,
            &mut regen,
        );
        // Checkbox: instant regen.
        let mut sympodial = cfg.sympodial;
        let r = ui.checkbox(&mut sympodial, "Sympodial");
        writeback |= r.changed();
        regen |= r.changed();
        cfg.sympodial = sympodial;

        // Inline leaf appearance sub-section.
        let (wb, rg) = leaf_config_editor(ui, &mut cfg.leaf, mat_id + 128);
        writeback |= wb;
        regen |= rg;
    });
    (writeback, regen)
}

fn bark_config_editor(
    ui: &mut egui::Ui,
    cfg: &mut bevy_symbios_texture::bark::BarkConfig,
    mat_id: u8,
) -> (bool, bool) {
    let mut writeback = false;
    let mut regen = false;
    ui.collapsing(format!("Bark Config##bc_{mat_id}"), |ui| {
        ui.horizontal(|ui| {
            ui.label("Light Color:");
            let r = ui.color_edit_button_rgb(&mut cfg.color_light);
            writeback |= r.changed();
            regen |= r.changed();
        });
        ui.horizontal(|ui| {
            ui.label("Dark Color:");
            let r = ui.color_edit_button_rgb(&mut cfg.color_dark);
            writeback |= r.changed();
            regen |= r.changed();
        });
        slider_debounced(
            ui,
            egui::Slider::new(&mut cfg.scale, 1.0..=12.0).text("Scale"),
            &mut writeback,
            &mut regen,
        );
        slider_debounced(
            ui,
            egui::Slider::new(&mut cfg.warp_u, 0.0..=0.5).text("Warp H"),
            &mut writeback,
            &mut regen,
        );
        slider_debounced(
            ui,
            egui::Slider::new(&mut cfg.warp_v, 0.0..=1.5).text("Warp V"),
            &mut writeback,
            &mut regen,
        );
        slider_debounced(
            ui,
            egui::Slider::new(&mut cfg.normal_strength, 0.0..=8.0).text("Normal Strength"),
            &mut writeback,
            &mut regen,
        );
        // octaves is usize — need local copy.
        let mut octaves = cfg.octaves;
        let r = ui.add(egui::Slider::new(&mut octaves, 1..=8).text("Octaves"));
        writeback |= r.changed();
        regen |= r.drag_stopped() || (r.changed() && !r.dragged());
        cfg.octaves = octaves;

        ui.separator();
        ui.label("Rhytidome Plates:");
        slider_debounced(
            ui,
            egui::Slider::new(&mut cfg.furrow_multiplier, 0.0..=1.0).text("Furrow Blend"),
            &mut writeback,
            &mut regen,
        );
        slider_debounced(
            ui,
            egui::Slider::new(&mut cfg.furrow_scale_u, 0.5..=6.0).text("Plate Width"),
            &mut writeback,
            &mut regen,
        );
        slider_debounced(
            ui,
            egui::Slider::new(&mut cfg.furrow_scale_v, 0.05..=1.0).text("Plate Length"),
            &mut writeback,
            &mut regen,
        );
        slider_debounced(
            ui,
            egui::Slider::new(&mut cfg.furrow_shape, 0.1..=2.0).text("Plate Shape"),
            &mut writeback,
            &mut regen,
        );
    });
    (writeback, regen)
}

/// Adds a slider and accumulates writeback/regen flags with drag-aware debouncing.
///
/// - `writeback` is set on any `changed()` (even mid-drag) so the caller can write
///   the value back to settings and prevent visual snap-back.
/// - `regen` is set only when the drag commits (`drag_stopped`) or for non-drag
///   changes (keyboard/click), avoiding unnecessary texture regeneration.
fn slider_debounced(
    ui: &mut egui::Ui,
    slider: impl egui::Widget,
    writeback: &mut bool,
    regen: &mut bool,
) {
    let r = ui.add(slider);
    *writeback |= r.changed();
    *regen |= r.drag_stopped() || (r.changed() && !r.dragged());
}
