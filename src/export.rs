//! Export utilities for converting L-System meshes to standard 3D file formats.
//!
//! Supports OBJ (text) and GLB (binary glTF 2.0) formats. These are pure data
//! conversion functions with no Bevy system dependencies — call them from your
//! own export systems or CLI tools.

use bevy::mesh::{Indices, VertexAttributeValues};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::materials::MaterialSettings;

/// Export format options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExportFormat {
    #[default]
    Obj,
    Glb,
}

impl ExportFormat {
    pub const ALL: &'static [ExportFormat] = &[ExportFormat::Obj, ExportFormat::Glb];

    pub fn name(&self) -> &'static str {
        match self {
            ExportFormat::Obj => "OBJ",
            ExportFormat::Glb => "GLB",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Obj => "obj",
            ExportFormat::Glb => "glb",
        }
    }
}

// ---------------------------------------------------------------------------
// OBJ Export
// ---------------------------------------------------------------------------

/// Convert mesh buckets to a combined OBJ format string.
///
/// Each material's mesh becomes a separate OBJ object named `{base_name}_mat{id}`.
/// Returns the combined OBJ text (without header comments — prepend your own).
pub fn meshes_to_obj(mesh_buckets: &HashMap<u8, Mesh>, base_name: &str) -> String {
    let mut combined = String::new();
    let mut vertex_offset = 0u32;

    for (material_id, mesh) in mesh_buckets {
        let object_name = format!("{}_mat{}", base_name, material_id);
        combined.push_str(&mesh_to_obj(mesh, &object_name, vertex_offset));
        vertex_offset += mesh.count_vertices() as u32;
    }

    combined
}

/// Convert a single Bevy [`Mesh`] to OBJ format text.
///
/// `vertex_offset` is added to all vertex indices for combining multiple meshes
/// into a single OBJ file. Pass `0` for a standalone mesh.
pub fn mesh_to_obj(mesh: &Mesh, object_name: &str, vertex_offset: u32) -> String {
    let mut obj = String::new();
    obj.push_str(&format!("o {}\n", object_name));

    let positions = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .and_then(|attr| match attr {
            VertexAttributeValues::Float32x3(v) => Some(v),
            _ => None,
        });

    let normals = mesh
        .attribute(Mesh::ATTRIBUTE_NORMAL)
        .and_then(|attr| match attr {
            VertexAttributeValues::Float32x3(v) => Some(v),
            _ => None,
        });

    if let Some(positions) = positions {
        for pos in positions {
            obj.push_str(&format!("v {} {} {}\n", pos[0], pos[1], pos[2]));
        }
    }

    if let Some(normals) = normals {
        for norm in normals {
            obj.push_str(&format!("vn {} {} {}\n", norm[0], norm[1], norm[2]));
        }
    }

    if let Some(indices) = mesh.indices() {
        let has_normals = normals.is_some();
        match indices {
            Indices::U16(idx) => {
                for tri in idx.chunks(3) {
                    if tri.len() == 3 {
                        let (a, b, c) = (
                            tri[0] as u32 + 1 + vertex_offset,
                            tri[1] as u32 + 1 + vertex_offset,
                            tri[2] as u32 + 1 + vertex_offset,
                        );
                        if has_normals {
                            obj.push_str(&format!("f {}//{} {}//{} {}//{}\n", a, a, b, b, c, c));
                        } else {
                            obj.push_str(&format!("f {} {} {}\n", a, b, c));
                        }
                    }
                }
            }
            Indices::U32(idx) => {
                for tri in idx.chunks(3) {
                    if tri.len() == 3 {
                        let (a, b, c) = (
                            tri[0] + 1 + vertex_offset,
                            tri[1] + 1 + vertex_offset,
                            tri[2] + 1 + vertex_offset,
                        );
                        if has_normals {
                            obj.push_str(&format!("f {}//{} {}//{} {}//{}\n", a, a, b, b, c, c));
                        } else {
                            obj.push_str(&format!("f {} {} {}\n", a, b, c));
                        }
                    }
                }
            }
        }
    }

    obj
}

// ---------------------------------------------------------------------------
// GLB (Binary glTF 2.0) Export
// ---------------------------------------------------------------------------

/// Convert mesh buckets and material settings to GLB (binary glTF 2.0) format.
///
/// Each mesh bucket becomes a separate glTF mesh/node/material. Materials use
/// PBR metallic-roughness with base color, metallic, roughness, and emissive
/// derived from [`MaterialSettings`].
pub fn meshes_to_glb(
    mesh_buckets: &HashMap<u8, Mesh>,
    material_settings: &HashMap<u8, MaterialSettings>,
) -> Vec<u8> {
    build_glb(mesh_buckets, material_settings)
}

fn build_glb(
    mesh_buckets: &HashMap<u8, Mesh>,
    material_settings: &HashMap<u8, MaterialSettings>,
) -> Vec<u8> {
    let mut bin_buffer: Vec<u8> = Vec::new();
    let mut buffer_views = Vec::new();
    let mut accessors = Vec::new();
    let mut gltf_meshes = Vec::new();
    let mut gltf_nodes = Vec::new();
    let mut gltf_materials = Vec::new();

    let mut mat_ids: Vec<u8> = mesh_buckets.keys().copied().collect();
    mat_ids.sort();

    // Build GLTF materials
    for &mat_id in &mat_ids {
        let defaults = MaterialSettings::default();
        let s = material_settings.get(&mat_id).unwrap_or(&defaults);
        let em_r = (s.emission_color[0] * s.emission_strength).min(1.0);
        let em_g = (s.emission_color[1] * s.emission_strength).min(1.0);
        let em_b = (s.emission_color[2] * s.emission_strength).min(1.0);

        gltf_materials.push(format!(
            concat!(
                "{{",
                "\"name\":\"Material_{}\",",
                "\"pbrMetallicRoughness\":{{",
                "\"baseColorFactor\":[{:.4},{:.4},{:.4},1.0],",
                "\"metallicFactor\":{:.4},",
                "\"roughnessFactor\":{:.4}",
                "}},",
                "\"emissiveFactor\":[{:.4},{:.4},{:.4}]",
                "}}"
            ),
            mat_id,
            s.base_color[0],
            s.base_color[1],
            s.base_color[2],
            s.metallic,
            s.roughness,
            em_r,
            em_g,
            em_b,
        ));
    }

    // Build mesh data
    for (mesh_idx, &mat_id) in mat_ids.iter().enumerate() {
        let mesh = &mesh_buckets[&mat_id];

        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|a| match a {
                VertexAttributeValues::Float32x3(v) => Some(v),
                _ => None,
            });

        let normals = mesh
            .attribute(Mesh::ATTRIBUTE_NORMAL)
            .and_then(|a| match a {
                VertexAttributeValues::Float32x3(v) => Some(v),
                _ => None,
            });

        let Some(positions) = positions else {
            continue;
        };
        let vertex_count = positions.len();
        if vertex_count == 0 {
            continue;
        }

        // Compute position bounds (required by GLTF spec for POSITION accessor)
        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];
        for pos in positions {
            for i in 0..3 {
                min[i] = min[i].min(pos[i]);
                max[i] = max[i].max(pos[i]);
            }
        }

        let mut attr_entries = Vec::new();

        // --- Positions ---
        let pos_accessor_idx = accessors.len();
        attr_entries.push(format!("\"POSITION\":{}", pos_accessor_idx));

        let pos_offset = bin_buffer.len();
        for pos in positions {
            bin_buffer.extend_from_slice(&pos[0].to_le_bytes());
            bin_buffer.extend_from_slice(&pos[1].to_le_bytes());
            bin_buffer.extend_from_slice(&pos[2].to_le_bytes());
        }
        let pos_length = bin_buffer.len() - pos_offset;

        buffer_views.push(format!(
            "{{\"buffer\":0,\"byteOffset\":{},\"byteLength\":{},\"target\":34962}}",
            pos_offset, pos_length
        ));
        accessors.push(format!(
            concat!(
                "{{\"bufferView\":{},\"componentType\":5126,\"count\":{},\"type\":\"VEC3\",",
                "\"min\":[{:.6},{:.6},{:.6}],\"max\":[{:.6},{:.6},{:.6}]}}"
            ),
            buffer_views.len() - 1,
            vertex_count,
            min[0],
            min[1],
            min[2],
            max[0],
            max[1],
            max[2],
        ));

        // --- Normals ---
        if let Some(normals) = normals {
            let norm_accessor_idx = accessors.len();
            attr_entries.push(format!("\"NORMAL\":{}", norm_accessor_idx));

            let norm_offset = bin_buffer.len();
            for norm in normals {
                bin_buffer.extend_from_slice(&norm[0].to_le_bytes());
                bin_buffer.extend_from_slice(&norm[1].to_le_bytes());
                bin_buffer.extend_from_slice(&norm[2].to_le_bytes());
            }
            let norm_length = bin_buffer.len() - norm_offset;

            buffer_views.push(format!(
                "{{\"buffer\":0,\"byteOffset\":{},\"byteLength\":{},\"target\":34962}}",
                norm_offset, norm_length
            ));
            accessors.push(format!(
                "{{\"bufferView\":{},\"componentType\":5126,\"count\":{},\"type\":\"VEC3\"}}",
                buffer_views.len() - 1,
                vertex_count,
            ));
        }

        // --- Vertex Colors ---
        let colors = mesh.attribute(Mesh::ATTRIBUTE_COLOR).and_then(|a| match a {
            VertexAttributeValues::Float32x4(v) => Some(v.as_slice()),
            _ => None,
        });
        if let Some(colors) = colors {
            let col_accessor_idx = accessors.len();
            attr_entries.push(format!("\"COLOR_0\":{}", col_accessor_idx));

            let col_offset = bin_buffer.len();
            for col in colors {
                bin_buffer.extend_from_slice(&col[0].to_le_bytes());
                bin_buffer.extend_from_slice(&col[1].to_le_bytes());
                bin_buffer.extend_from_slice(&col[2].to_le_bytes());
                bin_buffer.extend_from_slice(&col[3].to_le_bytes());
            }
            let col_length = bin_buffer.len() - col_offset;

            buffer_views.push(format!(
                "{{\"buffer\":0,\"byteOffset\":{},\"byteLength\":{},\"target\":34962}}",
                col_offset, col_length
            ));
            accessors.push(format!(
                "{{\"bufferView\":{},\"componentType\":5126,\"count\":{},\"type\":\"VEC4\"}}",
                buffer_views.len() - 1,
                vertex_count,
            ));
        }

        // --- Indices ---
        let mut indices_accessor_str = String::new();
        if let Some(indices) = mesh.indices() {
            let idx_accessor_idx = accessors.len();
            indices_accessor_str = format!(",\"indices\":{}", idx_accessor_idx);

            let idx_offset = bin_buffer.len();
            let index_count = match indices {
                Indices::U16(idx) => {
                    for &i in idx {
                        bin_buffer.extend_from_slice(&(i as u32).to_le_bytes());
                    }
                    idx.len()
                }
                Indices::U32(idx) => {
                    for &i in idx {
                        bin_buffer.extend_from_slice(&i.to_le_bytes());
                    }
                    idx.len()
                }
            };
            let idx_length = bin_buffer.len() - idx_offset;

            buffer_views.push(format!(
                "{{\"buffer\":0,\"byteOffset\":{},\"byteLength\":{},\"target\":34963}}",
                idx_offset, idx_length
            ));
            accessors.push(format!(
                "{{\"bufferView\":{},\"componentType\":5125,\"count\":{},\"type\":\"SCALAR\"}}",
                buffer_views.len() - 1,
                index_count,
            ));
        }

        let attrs_json = attr_entries.join(",");
        gltf_meshes.push(format!(
            "{{\"name\":\"mesh_mat{}\",\"primitives\":[{{\"attributes\":{{{}}}{},\"material\":{}}}]}}",
            mat_id, attrs_json, indices_accessor_str, mesh_idx
        ));

        gltf_nodes.push(format!(
            "{{\"name\":\"node_mat{}\",\"mesh\":{}}}",
            mat_id, mesh_idx
        ));
    }

    if gltf_nodes.is_empty() {
        return build_empty_glb();
    }

    let node_indices: String = (0..gltf_nodes.len())
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(",");

    let json = format!(
        concat!(
            "{{",
            "\"asset\":{{\"version\":\"2.0\",\"generator\":\"bevy_symbios\"}},",
            "\"scene\":0,",
            "\"scenes\":[{{\"name\":\"LSystem\",\"nodes\":[{}]}}],",
            "\"nodes\":[{}],",
            "\"meshes\":[{}],",
            "\"materials\":[{}],",
            "\"accessors\":[{}],",
            "\"bufferViews\":[{}],",
            "\"buffers\":[{{\"byteLength\":{}}}]",
            "}}"
        ),
        node_indices,
        gltf_nodes.join(","),
        gltf_meshes.join(","),
        gltf_materials.join(","),
        accessors.join(","),
        buffer_views.join(","),
        bin_buffer.len(),
    );

    pack_glb(&json, &bin_buffer)
}

fn build_empty_glb() -> Vec<u8> {
    let json = r#"{"asset":{"version":"2.0","generator":"bevy_symbios"},"scene":0,"scenes":[{"name":"Empty"}]}"#;
    pack_glb(json, &[])
}

fn pack_glb(json: &str, bin_data: &[u8]) -> Vec<u8> {
    let json_bytes = json.as_bytes();
    let json_padded_len = (json_bytes.len() + 3) & !3;
    let bin_padded_len = (bin_data.len() + 3) & !3;

    let has_bin = !bin_data.is_empty();
    let bin_chunk_size = if has_bin { 8 + bin_padded_len } else { 0 };
    let total_length = 12 + 8 + json_padded_len + bin_chunk_size;

    let mut glb = Vec::with_capacity(total_length);

    // GLB Header
    glb.extend_from_slice(&0x46546C67u32.to_le_bytes()); // magic "glTF"
    glb.extend_from_slice(&2u32.to_le_bytes()); // version
    glb.extend_from_slice(&(total_length as u32).to_le_bytes());

    // JSON Chunk
    glb.extend_from_slice(&(json_padded_len as u32).to_le_bytes());
    glb.extend_from_slice(&0x4E4F534Au32.to_le_bytes()); // "JSON"
    glb.extend_from_slice(json_bytes);
    glb.resize(glb.len() + json_padded_len - json_bytes.len(), b' ');

    // BIN Chunk
    if has_bin {
        glb.extend_from_slice(&(bin_padded_len as u32).to_le_bytes());
        glb.extend_from_slice(&0x004E4942u32.to_le_bytes()); // "BIN\0"
        glb.extend_from_slice(bin_data);
        glb.resize(glb.len() + bin_padded_len - bin_data.len(), 0);
    }

    glb
}
