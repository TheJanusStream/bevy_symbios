use crate::materials::MaterialPalette;
use avian3d::prelude::*;
use bevy::prelude::*;
use symbios_robot::{JointType, RobotBlueprint, SensorType, ShapePrimitive};

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

pub fn spawn_robot(
    commands: &mut Commands,
    blueprint: &RobotBlueprint,
    palette: &MaterialPalette,
    meshes: &mut Assets<Mesh>,
    spawn_location: Transform,
) -> SpawnedRobot {
    let mut entity_map = bevy::platform::collections::HashMap::new();
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

        let entity = commands
            .spawn((
                RigidBody::Dynamic,
                collider,
                Mass(module.mass),
                Mesh3d(mesh_handle),
                MeshMaterial3d(material),
                initial_transform,
            ))
            .id();

        entity_map.insert(*mod_id, entity);
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
        let parent_entity = *entity_map
            .get(&joint_def.parent_id)
            .expect("Parent module missing");
        let child_entity = *entity_map
            .get(&joint_def.child_id)
            .expect("Child module missing");

        let joint_entity = match joint_def.joint_type {
            JointType::Fixed => {
                commands.spawn(
                    FixedJoint::new(parent_entity, child_entity)
                        .with_local_anchor1(joint_def.anchor_parent)
                        .with_local_anchor2(joint_def.anchor_child),
                ).id()
            }
            JointType::Hinge => {
                let mut joint = RevoluteJoint::new(parent_entity, child_entity)
                    .with_local_anchor1(joint_def.anchor_parent)
                    .with_local_anchor2(joint_def.anchor_child)
                    .with_hinge_axis(joint_def.axis);

                if let Some(limit) = &joint_def.limits {
                    joint = joint.with_angle_limits(limit.min, limit.max);

                    let motor = AngularMotor::default()
                        .with_max_torque(limit.effort)
                        .with_target_velocity(limit.velocity);

                    joint.motor = motor;
                }

                commands.spawn(joint).id()
            }
            JointType::Ball => {
                commands.spawn(
                    SphericalJoint::new(parent_entity, child_entity)
                        .with_local_anchor1(joint_def.anchor_parent)
                        .with_local_anchor2(joint_def.anchor_child),
                ).id()
            }
            JointType::Prismatic => {
                let mut joint = PrismaticJoint::new(parent_entity, child_entity)
                    .with_local_anchor1(joint_def.anchor_parent)
                    .with_local_anchor2(joint_def.anchor_child)
                    .with_slider_axis(joint_def.axis);

                if let Some(limit) = &joint_def.limits {
                    joint = joint.with_limits(limit.min, limit.max);

                    let motor = LinearMotor::default()
                        .with_max_force(limit.effort)
                        .with_target_velocity(limit.velocity);

                    joint.motor = motor;
                }

                commands.spawn(joint).id()
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