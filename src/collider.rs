//! Capsule collider generation for L-System skeletons.
//!
//! This module provides efficient physics collision shapes by generating capsule
//! colliders along skeleton strands. This is significantly faster than convex
//! decomposition for branch-like structures.

use avian3d::prelude::Collider;
use bevy::prelude::*;
use symbios_turtle_3d::{Skeleton, SkeletonPoint};

/// A positioned capsule collider ready to be spawned into the world.
#[derive(Debug, Clone)]
pub struct PositionedCollider {
    /// World-space transform for the collider center.
    pub transform: Transform,
    /// The capsule collider shape.
    pub collider: Collider,
    /// Average radius of the segment (for reference).
    pub radius: f32,
    /// Length of the segment.
    pub length: f32,
}

/// Generates capsule colliders from L-System skeletons.
///
/// Iterates through skeleton strands and creates capsule colliders for each
/// segment that meets the minimum radius threshold. Thin twigs can be filtered
/// out to reduce physics overhead.
pub struct ColliderGenerator {
    min_radius: f32,
}

impl Default for ColliderGenerator {
    fn default() -> Self {
        Self { min_radius: 0.0 }
    }
}

impl ColliderGenerator {
    /// Creates a new collider generator with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the minimum radius threshold for collider generation.
    ///
    /// Segments with average radius below this threshold will be skipped.
    /// Use this to ignore thin twigs and reduce physics overhead.
    pub fn with_min_radius(mut self, min_radius: f32) -> Self {
        self.min_radius = min_radius.max(0.0);
        self
    }

    /// Generates a single compound collider for the entire skeleton.
    ///
    /// Returns `None` if no valid segments exist (empty skeleton or all segments
    /// below the minimum radius threshold). The compound collider contains one
    /// child shape per qualifying segment, avoiding ECS entity bloat.
    pub fn build(&self, skeleton: &Skeleton) -> Option<Collider> {
        let parts = self.build_parts(skeleton);
        if parts.is_empty() {
            return None;
        }
        Some(Collider::compound(
            parts
                .into_iter()
                .map(|p| (p.transform.translation, p.transform.rotation, p.collider))
                .collect::<Vec<_>>(),
        ))
    }

    /// Generates individual positioned colliders for each qualifying segment.
    ///
    /// Useful for debugging, visualization, or custom compound construction.
    /// For most use cases, prefer [`build`] which returns a single compound collider.
    pub fn build_parts(&self, skeleton: &Skeleton) -> Vec<PositionedCollider> {
        let mut colliders = Vec::new();

        for strand in &skeleton.strands {
            if strand.len() < 2 {
                continue;
            }
            self.process_strand(strand, &mut colliders);
        }

        colliders
    }

    fn process_strand(&self, points: &[SkeletonPoint], colliders: &mut Vec<PositionedCollider>) {
        if points.len() < 2 {
            return;
        }

        // Filter out duplicate adjacent points (zero-length segments)
        let filtered_points: Vec<&SkeletonPoint> = {
            let mut result = vec![&points[0]];
            for point in &points[1..] {
                let last = result.last().unwrap();
                if last.position.distance_squared(point.position) > 0.000001 {
                    result.push(point);
                }
            }
            result
        };

        if filtered_points.len() < 2 {
            return;
        }

        for i in 0..filtered_points.len() - 1 {
            let start = filtered_points[i];
            let end = filtered_points[i + 1];

            let avg_radius = (start.radius + end.radius) * 0.5;

            // Skip segments below threshold
            if avg_radius < self.min_radius {
                continue;
            }

            let segment_vec = end.position - start.position;
            let length = segment_vec.length();

            if length < 0.0001 {
                continue;
            }

            // Calculate center position and orientation
            let center = (start.position + end.position) * 0.5;
            let direction = segment_vec / length;

            // Capsule is aligned along Y axis by default in Avian
            // We need to rotate from Y to our direction
            let rotation = Quat::from_rotation_arc(Vec3::Y, direction);

            // For short segments (length < 2*radius), a capsule extends beyond the
            // segment endpoints causing ghost collisions. Use a sphere instead.
            let collider = if length < 2.0 * avg_radius {
                Collider::sphere(avg_radius)
            } else {
                let cylinder_length = length - 2.0 * avg_radius;
                Collider::capsule(avg_radius, cylinder_length)
            };

            colliders.push(PositionedCollider {
                transform: Transform::from_translation(center).with_rotation(rotation),
                collider,
                radius: avg_radius,
                length,
            });
        }
    }
}
