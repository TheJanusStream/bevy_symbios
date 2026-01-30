//! Mesh generation for L-System skeletons.
//!
//! This module converts [`Skeleton`] data from `symbios-turtle-3d` into Bevy [`Mesh`]es.
//! Each skeleton strand becomes a smooth tube mesh using parallel transport for
//! twist-free geometry.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use symbios_turtle_3d::{Skeleton, SkeletonPoint};

// Helper struct to build a single mesh
#[derive(Default)]
struct MeshData {
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    colors: Vec<[f32; 4]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl MeshData {
    fn to_mesh(&self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs.clone());
        mesh.insert_indices(Indices::U32(self.indices.clone()));
        mesh
    }
}

/// Maximum allowed tube resolution to prevent memory exhaustion.
/// 128 vertices per ring is more than sufficient for smooth tubes.
const MAX_RESOLUTION: u32 = 128;

/// Converts L-System skeletons into Bevy meshes.
///
/// Generates smooth tube geometry from [`Skeleton`] strands using parallel transport
/// to avoid twisting artifacts. Segments are bucketed by material ID, producing
/// separate meshes for each material.
///
/// # Features
///
/// - **Multi-material support**: Segments with different `material_id` values produce
///   separate meshes, allowing different Bevy materials to be applied.
/// - **Vertex colors**: Per-vertex colors from [`SkeletonPoint::color`] are included.
/// - **UV mapping**: Arc-length parameterized UVs with aspect-ratio preservation.
///   U wraps around the tube (0.0 to 1.0), V increases along the strand.
///   V is scaled by each point's [`SkeletonPoint::uv_scale`] factor.
/// - **Smooth geometry**: Parallel transport prevents tube twisting at bends.
///
/// # Example
///
/// ```ignore
/// use bevy_symbios::LSystemMeshBuilder;
///
/// let skeleton = /* ... generate skeleton ... */;
/// let meshes = LSystemMeshBuilder::new()
///     .with_resolution(12)
///     .build(&skeleton);
///
/// for (material_id, mesh) in meshes {
///     // Spawn each mesh with appropriate material
/// }
/// ```
pub struct LSystemMeshBuilder {
    buckets: HashMap<u8, MeshData>,
    resolution: u32,
}

impl Default for LSystemMeshBuilder {
    fn default() -> Self {
        Self {
            buckets: HashMap::new(),
            resolution: 8,
        }
    }
}

impl LSystemMeshBuilder {
    /// Creates a new mesh builder with default settings (resolution = 8).
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the number of vertices around each ring of the tube.
    ///
    /// Higher values produce smoother tubes but increase vertex count.
    /// Minimum value is 3 (triangular cross-section), maximum is 128.
    /// Values outside this range are clamped with a warning. Default is 8.
    pub fn with_resolution(mut self, res: u32) -> Self {
        if res > MAX_RESOLUTION {
            warn!(
                "Mesh resolution {} exceeds maximum of {}; clamping to {}",
                res, MAX_RESOLUTION, MAX_RESOLUTION
            );
        }
        self.resolution = res.clamp(3, MAX_RESOLUTION);
        self
    }

    /// Builds meshes from the skeleton, consuming the builder.
    ///
    /// Returns a map from material ID to [`Mesh`]. Each mesh contains all segments
    /// that share the same `material_id` from their starting [`SkeletonPoint`].
    ///
    /// Empty skeletons or strands with fewer than 2 points produce no output.
    pub fn build(mut self, skeleton: &Skeleton) -> HashMap<u8, Mesh> {
        for strand in &skeleton.strands {
            if strand.len() < 2 {
                continue;
            }
            self.process_strand(strand);
        }

        self.buckets
            .into_iter()
            .map(|(k, v)| (k, v.to_mesh()))
            .collect()
    }

    fn process_strand(&mut self, points: &[SkeletonPoint]) {
        // Filter out duplicate adjacent points (zero-length segments) to prevent NaNs.
        // Build a list by keeping only points whose position differs from the last kept point.
        let filtered: Vec<&SkeletonPoint> = {
            let mut result = vec![&points[0]];
            for point in &points[1..] {
                let last = result.last().unwrap();
                if last.position.distance_squared(point.position) > 0.000001 {
                    result.push(point);
                }
            }
            result
        };

        if filtered.len() < 2 {
            return;
        }

        let points = filtered;
        let n = points.len();

        // Phase 1: Compute per-point rotations via parallel transport.
        // Each point gets its own rotation based on its miter tangent, enabling
        // vertex sharing between consecutive same-material segments.
        let rotations = {
            let mut rots = Vec::with_capacity(n);

            // Point 0: align turtle rotation with first segment tangent
            let tangent_0 = (points[1].position - points[0].position).normalize_or_zero();
            let mut rot = points[0].rotation;
            let turtle_fwd = rot * Vec3::Y;
            rot = Self::robust_rotation_arc(turtle_fwd, tangent_0) * rot;
            rots.push(rot);

            // Points 1..N-1: use miter tangent (or endpoint tangent for last point)
            for i in 1..n {
                let tangent = if i < n - 1 {
                    let v_in = (points[i].position - points[i - 1].position).normalize_or_zero();
                    let v_out = (points[i + 1].position - points[i].position).normalize_or_zero();
                    let sum = v_in + v_out;
                    if sum.length_squared() < 0.001 {
                        v_in
                    } else {
                        sum.normalize()
                    }
                } else {
                    (points[i].position - points[i - 1].position).normalize_or_zero()
                };

                let fwd = rot * Vec3::Y;
                let bend = Self::robust_rotation_arc(fwd, tangent);
                rot = bend * rot;
                rots.push(rot);
            }

            rots
        };

        // Phase 2: Compute per-point V coordinates using incremental accumulation.
        // This ensures UV continuity across tapered segments where circumference varies.
        let v_coords = {
            let mut coords = Vec::with_capacity(n);
            let mut cumulative_v = 0.0f32;
            coords.push(0.0);

            for i in 0..n - 1 {
                let seg_len = points[i].position.distance(points[i + 1].position);
                let avg_radius = (points[i].radius + points[i + 1].radius) * 0.5;
                let circumference = avg_radius * std::f32::consts::TAU;
                let v_scale = if circumference > 0.0001 {
                    1.0 / circumference
                } else {
                    1.0
                };
                cumulative_v += seg_len * v_scale * points[i].uv_scale;
                coords.push(cumulative_v);
            }

            coords
        };

        // Phase 3: Generate rings and connect, with vertex sharing.
        // When consecutive segments share the same material ID, the top ring of
        // segment N is reused as the bottom ring of segment N+1.
        let mut ring_cache: Vec<Option<(u8, u32)>> = vec![None; n];

        for i in 0..n - 1 {
            let curr = points[i];
            let next = points[i + 1];
            let mat_id = curr.material_id;
            let bucket = self.buckets.entry(mat_id).or_default();

            // Bottom ring: reuse cached ring if same material bucket already has one
            let bottom_idx = match ring_cache[i] {
                Some((cached_mat, idx)) if cached_mat == mat_id => idx,
                _ => Self::add_ring(
                    bucket,
                    curr.position,
                    rotations[i],
                    curr.radius,
                    curr.color,
                    v_coords[i],
                    self.resolution,
                ),
            };

            // Top ring: always generate fresh
            let top_idx = Self::add_ring(
                bucket,
                next.position,
                rotations[i + 1],
                next.radius,
                next.color,
                v_coords[i + 1],
                self.resolution,
            );

            Self::connect_rings(bucket, bottom_idx, top_idx, self.resolution);

            // Cache the top ring for potential reuse by the next segment
            ring_cache[i + 1] = Some((mat_id, top_idx));
        }
    }

    fn robust_rotation_arc(from: Vec3, to: Vec3) -> Quat {
        const DOT_THRESHOLD: f32 = 0.9999;
        let dot = from.dot(to);
        if dot < -DOT_THRESHOLD {
            let axis = if from.x.abs() < 0.8 {
                Vec3::X.cross(from).normalize()
            } else {
                Vec3::Y.cross(from).normalize()
            };
            return Quat::from_axis_angle(axis, std::f32::consts::PI);
        } else if dot > DOT_THRESHOLD {
            return Quat::IDENTITY;
        }
        Quat::from_rotation_arc(from, to)
    }

    fn add_ring(
        data: &mut MeshData,
        center: Vec3,
        rotation: Quat,
        radius: f32,
        color: Vec4,
        v_coord: f32,
        res: u32,
    ) -> u32 {
        let start_index = data.positions.len() as u32;
        let color_array = color.to_array();

        for i in 0..=res {
            let u = i as f32 / res as f32;
            let theta = u * std::f32::consts::TAU;
            let (sin, cos) = theta.sin_cos();

            let local_pos = Vec3::new(cos * radius, 0.0, sin * radius);
            let local_normal = Vec3::new(cos, 0.0, sin);

            data.positions.push(center + (rotation * local_pos));
            data.normals.push(rotation * local_normal);
            data.colors.push(color_array);
            data.uvs.push([u, v_coord]);
        }
        start_index
    }

    fn connect_rings(data: &mut MeshData, bottom_start: u32, top_start: u32, res: u32) {
        for i in 0..res {
            let bottom_curr = bottom_start + i;
            let bottom_next = bottom_start + i + 1;
            let top_curr = top_start + i;
            let top_next = top_start + i + 1;

            data.indices.push(bottom_curr);
            data.indices.push(top_curr);
            data.indices.push(bottom_next);

            data.indices.push(bottom_next);
            data.indices.push(top_curr);
            data.indices.push(top_next);
        }
    }
}
