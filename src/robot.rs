//! Spawn articulated rigid-body robots from `symbios-robot` blueprints.
//!
//! Requires the `robot` Cargo feature (which implies `physics`). Each
//! [`RobotBlueprint`] module becomes a dynamic [`RigidBody`] with the matching
//! collider, mesh, and material; each blueprint joint becomes an avian3d
//! constraint (`FixedJoint`, `RevoluteJoint`, `SphericalJoint`, or
//! `PrismaticJoint`). `JointType::Screw` is unsupported by avian3d and is
//! approximated as `Fixed` with a warning.
//!
//! Hinge and prismatic joints with per-axis [`AxisLimit`] entries pick the
//! limit whose `axis` is parallel to the drive axis and install an angular or
//! linear motor with that entry's effort/velocity.
//!
//! Sensors declared on a module ([`SensorType::IMU`], [`SensorType::Touch`])
//! are attached as [`ImuSensor`] / [`TouchSensor`] components. Other sensor
//! kinds are currently ignored.

use crate::materials::MaterialPalette;
use avian3d::prelude::*;
use bevy::prelude::*;
use symbios_robot::{AxisLimit, JointType, RobotBlueprint, SensorType, ShapePrimitive};

/// Pick the [`AxisLimit`] whose axis aligns with the joint's drive axis.
///
/// `limits` is per-axis: a single-axis joint (Hinge/Prismatic) takes the entry
/// whose axis is parallel (within tolerance) to the drive axis. Falls back to
/// the first entry if no axis is parallel — matches the v0.2 behavior where
/// `Option<Limit>` was implicitly the joint's single axis.
fn limit_for_axis(limits: &[AxisLimit], drive_axis: Vec3) -> Option<&AxisLimit> {
    let drive = drive_axis.normalize_or_zero();
    limits
        .iter()
        .find(|l| l.axis.normalize_or_zero().dot(drive).abs() > 0.999)
        .or_else(|| limits.first())
}

/// Marker: this module entity has an IMU sensor.
/// The IMU reads pitch and roll from the entity's `Transform`.
#[derive(Component)]
pub struct ImuSensor;

/// Marker: this module entity has a Touch sensor.
/// Touch reads ground contact from `CollidingEntities`.
#[derive(Component)]
pub struct TouchSensor;

/// Entities spawned by [`spawn_robot`], returned so callers can parent or tag them.
pub struct SpawnedRobot {
    /// Module (rigid body) entities, in blueprint iteration order.
    pub modules: Vec<Entity>,
    /// Joint constraint entities, in blueprint iteration order.
    /// Each entry is `(entity, joint_type)`.
    pub joints: Vec<(Entity, JointType)>,
    /// Module entities that carry sensors, sorted by `ModuleId`.
    /// Each entry is `(module_entity, sensor_type)`.
    pub sensors: Vec<(Entity, SensorType)>,
}

/// Spawn a [`RobotBlueprint`] into the world.
///
/// Modules are spawned in `ModuleId` order (so [`SpawnedRobot::sensors`] is
/// deterministic), each with `RigidBody::Dynamic`, its [`ShapePrimitive`]
/// converted to both a Bevy mesh and an avian3d collider, and a
/// [`MassPropertiesBundle`] derived from the shape and density. Each module's
/// material is looked up in `palette` by `material_id`, falling back to
/// `palette.primary_material` when missing.
///
/// Joints are then spawned with their anchors and a `local_basis2` set to the
/// child's rest-pose relative rotation so the solver preserves the intended
/// orientation between modules. Returns the spawned entities so callers can
/// parent them under a root, tag them, or wire up motor control systems.
pub fn spawn_robot(
    commands: &mut Commands,
    blueprint: &RobotBlueprint,
    palette: &MaterialPalette,
    meshes: &mut Assets<Mesh>,
    spawn_location: Transform,
) -> SpawnedRobot {
    let mut entity_map: bevy::platform::collections::HashMap<_, (Entity, Quat)> =
        bevy::platform::collections::HashMap::new();
    let mut module_entities = Vec::new();
    let mut joint_entities = Vec::new();
    let mut sensor_entries = Vec::new();

    // Sort modules by ModuleId for deterministic sensor ordering.
    let mut sorted_modules: Vec<_> = blueprint.modules.iter().collect();
    sorted_modules.sort_by_key(|(id, _)| *id);

    // 1. Spawn Modules (Rigid Bodies)
    for (mod_id, module) in &sorted_modules {
        let (pos, rot) = module.transform;

        let initial_transform =
            spawn_location * Transform::from_translation(pos).with_rotation(rot);

        let mesh_handle = match module.shape {
            ShapePrimitive::Box(e) => meshes.add(Cuboid::from_size(e * 2.0)),
            ShapePrimitive::Cylinder { radius, height } => {
                meshes.add(Cylinder::new(radius, height))
            }
            ShapePrimitive::Sphere(r) => meshes.add(Sphere::new(r)),
            ShapePrimitive::Capsule { radius, height } => {
                meshes.add(Capsule3d::new(radius, height))
            }
        };

        let collider = match module.shape {
            ShapePrimitive::Box(e) => Collider::cuboid(e.x * 2.0, e.y * 2.0, e.z * 2.0),
            ShapePrimitive::Cylinder { radius, height } => Collider::cylinder(radius, height),
            ShapePrimitive::Sphere(r) => Collider::sphere(r),
            ShapePrimitive::Capsule { radius, height } => Collider::capsule(radius, height),
        };

        let material = palette
            .materials
            .get(&module.material_id)
            .unwrap_or(&palette.primary_material)
            .clone();

        let mass_props =
            MassPropertiesBundle::from_shape(&module.shape.to_bevy_primitive(), module.density);

        let entity = commands
            .spawn((
                RigidBody::Dynamic,
                collider,
                mass_props,
                Mesh3d(mesh_handle),
                MeshMaterial3d(material),
                initial_transform,
            ))
            .id();

        entity_map.insert(*mod_id, (entity, rot));
        module_entities.push(entity);

        for sensor in &module.sensors {
            match sensor.sensor_type {
                SensorType::IMU => {
                    commands.entity(entity).insert(ImuSensor);
                    sensor_entries.push((entity, SensorType::IMU));
                }
                SensorType::Touch => {
                    commands.entity(entity).insert(TouchSensor);
                    sensor_entries.push((entity, SensorType::Touch));
                }
                _ => {}
            }
        }
    }

    // 2. Spawn Joints with Native Motors (Avian Main Branch)
    for joint_def in &blueprint.joints {
        let &(parent_entity, parent_rot) = entity_map
            .get(&joint_def.parent_id)
            .expect("Parent module missing");
        let &(child_entity, child_rot) = entity_map
            .get(&joint_def.child_id)
            .expect("Child module missing");

        // Compute the rest-pose relative rotation so the joint solver
        // preserves the intended orientation between modules.
        // Without this, all joints default to identity basis, forcing
        // connected bodies toward the same world orientation.
        let child_rest_basis = child_rot.inverse() * parent_rot;

        let joint_entity = match joint_def.joint_type {
            JointType::Fixed => commands
                .spawn(
                    FixedJoint::new(parent_entity, child_entity)
                        .with_local_anchor1(joint_def.anchor_parent)
                        .with_local_anchor2(joint_def.anchor_child)
                        .with_local_basis2(child_rest_basis),
                )
                .id(),
            JointType::Hinge { axis } => {
                let mut joint = RevoluteJoint::new(parent_entity, child_entity)
                    .with_local_anchor1(joint_def.anchor_parent)
                    .with_local_anchor2(joint_def.anchor_child)
                    .with_local_basis2(child_rest_basis)
                    .with_hinge_axis(axis);

                if let Some(limit) = limit_for_axis(&joint_def.limits, axis) {
                    joint = joint.with_angle_limits(limit.min, limit.max);

                    let motor = AngularMotor::default()
                        .with_max_torque(limit.effort)
                        .with_target_velocity(limit.velocity);

                    joint.motor = motor;
                }

                commands.spawn(joint).id()
            }
            JointType::Ball => commands
                .spawn(
                    SphericalJoint::new(parent_entity, child_entity)
                        .with_local_anchor1(joint_def.anchor_parent)
                        .with_local_anchor2(joint_def.anchor_child)
                        .with_local_basis2(child_rest_basis),
                )
                .id(),
            JointType::Prismatic { axis } => {
                let mut joint = PrismaticJoint::new(parent_entity, child_entity)
                    .with_local_anchor1(joint_def.anchor_parent)
                    .with_local_anchor2(joint_def.anchor_child)
                    .with_local_basis2(child_rest_basis)
                    .with_slider_axis(axis);

                if let Some(limit) = limit_for_axis(&joint_def.limits, axis) {
                    joint = joint.with_limits(limit.min, limit.max);

                    let motor = LinearMotor::default()
                        .with_max_force(limit.effort)
                        .with_target_velocity(limit.velocity);

                    joint.motor = motor;
                }

                commands.spawn(joint).id()
            }
            JointType::Screw { .. } => {
                // avian3d has no helical (screw) constraint. Approximate as Fixed
                // so the chain still spawns; downstream consumers can replace with
                // a custom constraint or break the rotation/translation coupling
                // into separate joints.
                warn!(
                    "Screw joints are not supported by avian3d; spawning Fixed joint between {:?} and {:?}",
                    joint_def.parent_id, joint_def.child_id
                );
                commands
                    .spawn(
                        FixedJoint::new(parent_entity, child_entity)
                            .with_local_anchor1(joint_def.anchor_parent)
                            .with_local_anchor2(joint_def.anchor_child)
                            .with_local_basis2(child_rest_basis),
                    )
                    .id()
            }
        };
        joint_entities.push((joint_entity, joint_def.joint_type));
    }

    SpawnedRobot {
        modules: module_entities,
        joints: joint_entities,
        sensors: sensor_entries,
    }
}
