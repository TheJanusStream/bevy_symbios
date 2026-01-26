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
    /// Minimum value is 3 (triangular cross-section). Default is 8.
    pub fn with_resolution(mut self, res: u32) -> Self {
        self.resolution = res.max(3);
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
        // Filter out duplicate points (zero-length segments) to prevent NaNs
        let filtered_points: Vec<&SkeletonPoint> = points
            .windows(2)
            .enumerate()
            .filter_map(|(i, window)| {
                if i == 0 {
                    return Some(&window[0]);
                }
                if window[0].position.distance_squared(window[1].position) > 0.000001 {
                    Some(&window[1])
                } else {
                    None
                }
            })
            .chain(std::iter::once(points.last().unwrap()))
            .collect();

        if filtered_points.len() < 2 {
            return;
        }

        let points = filtered_points;
        let points_count = points.len();

        let p0_pos = points[0].position;
        let p1_pos = points[1].position;
        let last_tangent = (p1_pos - p0_pos).normalize_or_zero();

        // Initial orientation
        let mut current_rotation = points[0].rotation;
        let initial_turtle_forward = current_rotation * Vec3::Y;
        let initial_correction = Self::robust_rotation_arc(initial_turtle_forward, last_tangent);
        current_rotation = initial_correction * current_rotation;

        // Track cumulative arc length for UV V coordinate
        let mut cumulative_length: f32 = 0.0;

        // Iterating Segments (i -> i+1)
        for i in 0..points_count - 1 {
            let curr = points[i];
            let next = points[i + 1];

            // 1. Determine Bucket based on the start of the segment
            let mat_id = curr.material_id;
            let bucket = self.buckets.entry(mat_id).or_default();

            // 2. Calculate Tangent & Rotation (Parallel Transport)
            // Use Miter Tangent logic for smooth corners
            let miter_tangent = if i == 0 {
                (next.position - curr.position).normalize_or_zero()
            } else {
                let prev = points[i - 1];
                let v_in = (curr.position - prev.position).normalize_or_zero();
                let v_out = (next.position - curr.position).normalize_or_zero();
                let sum = v_in + v_out;
                if sum.length_squared() < 0.001 {
                    v_in
                } else {
                    sum.normalize()
                }
            };

            let current_forward = current_rotation * Vec3::Y;
            let bend = Self::robust_rotation_arc(current_forward, miter_tangent);
            current_rotation = bend * current_rotation;

            // 3. Calculate UV V coordinates
            // V is scaled by radius to maintain aspect ratio (prevent texture stretching)
            let segment_length = curr.position.distance(next.position);
            let avg_radius = (curr.radius + next.radius) * 0.5;
            let circumference = avg_radius * std::f32::consts::TAU;
            let v_scale = if circumference > 0.0001 {
                1.0 / circumference
            } else {
                1.0
            };

            let v_start = cumulative_length * v_scale * curr.uv_scale;
            let v_end = (cumulative_length + segment_length) * v_scale * next.uv_scale;

            // 4. Generate Rings
            // We generate BOTH rings for this segment in this bucket.
            // This means vertices at boundaries are duplicated, which is necessary for split meshes.

            let bottom_idx = Self::add_ring(
                bucket,
                curr.position,
                current_rotation,
                curr.radius,
                curr.color,
                v_start,
                self.resolution,
            );

            let top_idx = Self::add_ring(
                bucket,
                next.position,
                current_rotation,
                next.radius,
                next.color,
                v_end,
                self.resolution,
            );

            // 5. Connect
            Self::connect_rings(bucket, bottom_idx, top_idx, self.resolution);

            // Update cumulative length for next segment
            cumulative_length += segment_length;
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
