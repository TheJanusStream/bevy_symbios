use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use symbios_turtle_3d::{Skeleton, SkeletonPoint};

pub struct LSystemMeshBuilder {
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    indices: Vec<u32>,
    resolution: u32,
}

impl Default for LSystemMeshBuilder {
    fn default() -> Self {
        Self {
            positions: Vec::new(),
            normals: Vec::new(),
            indices: Vec::new(),
            resolution: 8,
        }
    }
}

impl LSystemMeshBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the resolution (sides of the tube). Clamped to minimum 3.
    pub fn with_resolution(mut self, res: u32) -> Self {
        self.resolution = res.max(3);
        self
    }

    pub fn build(mut self, skeleton: &Skeleton) -> Mesh {
        for strand in &skeleton.strands {
            // Need at least 2 points to make a segment
            if strand.len() < 2 {
                continue;
            }
            self.process_strand(strand);
        }

        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }

    fn process_strand(&mut self, points: &[SkeletonPoint]) {
        // Filter out duplicate points (zero-length segments) to prevent NaNs
        let filtered_points: Vec<&SkeletonPoint> = points
            .windows(2)
            .enumerate()
            .filter_map(|(i, window)| {
                // Always keep the first point
                if i == 0 {
                    return Some(&window[0]);
                }
                // Keep point if distance to prev > epsilon
                if window[0].position.distance_squared(window[1].position) > 0.000001 {
                    Some(&window[1])
                } else {
                    None
                }
            })
            // Ensure the very last point is included if it's distinct
            .chain(std::iter::once(points.last().unwrap()))
            .collect();

        // Re-check length after filtering
        if filtered_points.len() < 2 {
            return;
        }

        let points = filtered_points;
        let points_count = points.len();
        let mut ring_start_indices = Vec::new();

        let p0_pos = points[0].position;
        let p1_pos = points[1].position;

        let last_tangent = (p1_pos - p0_pos).normalize_or_zero();

        // Initial orientation from the Turtle
        let mut current_rotation = points[0].rotation;

        // Ensure the initial rotation actually aligns with the first path segment
        let initial_turtle_forward = current_rotation * Vec3::Y;
        let initial_correction = Self::robust_rotation_arc(initial_turtle_forward, last_tangent);
        current_rotation = initial_correction * current_rotation;

        for i in 0..points_count {
            let curr = points[i];

            // Calculate Miter Tangent (Bisector)
            let miter_tangent = if i == 0 {
                (points[i + 1].position - curr.position).normalize_or_zero()
            } else if i == points_count - 1 {
                (curr.position - points[i - 1].position).normalize_or_zero()
            } else {
                let v_in = (curr.position - points[i - 1].position).normalize_or_zero();
                let v_out = (points[i + 1].position - curr.position).normalize_or_zero();
                let sum = v_in + v_out;
                if sum.length_squared() < 0.001 {
                    v_in
                } else {
                    sum.normalize()
                }
            };

            // PARALLEL TRANSPORT (Bishop Frame logic)
            let current_forward = current_rotation * Vec3::Y;
            let bend = Self::robust_rotation_arc(current_forward, miter_tangent);

            // Update the running rotation state
            current_rotation = bend * current_rotation;

            ring_start_indices.push(self.add_ring(curr.position, current_rotation, curr.radius));
        }

        // Connect rings
        for i in 0..points_count - 1 {
            self.connect_rings(ring_start_indices[i], ring_start_indices[i + 1]);
        }
    }

    /// Robust version of from_rotation_arc that handles 180 degree turns (antiparallel vectors)
    fn robust_rotation_arc(from: Vec3, to: Vec3) -> Quat {
        const DOT_THRESHOLD: f32 = 0.9999;

        let dot = from.dot(to);

        if dot < -DOT_THRESHOLD {
            // Antiparallel: 180 degree turn.
            // We need ANY axis perpendicular to 'from' to rotate around.
            let axis = if from.x.abs() < 0.8 {
                Vec3::X.cross(from).normalize()
            } else {
                Vec3::Y.cross(from).normalize()
            };
            return Quat::from_axis_angle(axis, std::f32::consts::PI);
        } else if dot > DOT_THRESHOLD {
            // Parallel: No rotation
            return Quat::IDENTITY;
        }

        Quat::from_rotation_arc(from, to)
    }

    fn add_ring(&mut self, center: Vec3, rotation: Quat, radius: f32) -> u32 {
        let start_index = self.positions.len() as u32;

        for i in 0..=self.resolution {
            let theta = (i as f32 / self.resolution as f32) * std::f32::consts::TAU;
            let (sin, cos) = theta.sin_cos();

            // Ring on XZ plane (Y is forward axis of tube)
            let local_pos = Vec3::new(cos * radius, 0.0, sin * radius);
            let local_normal = Vec3::new(cos, 0.0, sin);

            self.positions.push(center + (rotation * local_pos));
            self.normals.push(rotation * local_normal);
        }

        start_index
    }

    fn connect_rings(&mut self, bottom_start: u32, top_start: u32) {
        for i in 0..self.resolution {
            let bottom_curr = bottom_start + i;
            let bottom_next = bottom_start + i + 1;
            let top_curr = top_start + i;
            let top_next = top_start + i + 1;

            self.indices.push(bottom_curr);
            self.indices.push(top_curr);
            self.indices.push(bottom_next);

            self.indices.push(bottom_next);
            self.indices.push(top_curr);
            self.indices.push(top_next);
        }
    }
}
