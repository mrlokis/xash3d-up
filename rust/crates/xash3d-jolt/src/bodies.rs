//! Rigid body management for dynamic Xash3D entities in Jolt Physics.

use std::collections::HashMap;
use oxijolt::{
    Activation, BodyId, BodySettings, PhysicsWorld, Quat, RVec3, Shape, Vec3,
};
use xash3d_api::{SHAPE_CAPSULE, SHAPE_SPHERE};
use crate::bsp_converter::{TO_GOLDSRC, TO_JOLT};

pub struct EntityManager {
    entities: HashMap<i32, BodyId>,
}

impl EntityManager {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
        }
    }

    pub fn clear(&mut self, world: &mut PhysicsWorld) {
        for (_, body_id) in self.entities.drain() {
            let _ = world.remove_body(body_id);
        }
    }

    pub fn destroy(&mut self, world: &mut PhysicsWorld, ent_index: i32) {
        if let Some(body_id) = self.entities.remove(&ent_index) {
            let _ = world.remove_body(body_id);
        }
    }

    pub fn create(
        &mut self,
        world: &mut PhysicsWorld,
        ent_index: i32,
        shape_type: i32,
        mins: [f32; 3],
        maxs: [f32; 3],
        origin: [f32; 3],
        velocity: [f32; 3],
        mass: f32,
    ) -> bool {
        self.destroy(world, ent_index);

        let shape = match shape_type {
            SHAPE_SPHERE => {
                let dx = (maxs[0] - mins[0]).abs();
                let radius = (dx * 0.5).max(1.0) * TO_JOLT;
                Shape::new_sphere(radius).ok()
            }
            SHAPE_CAPSULE => {
                let radius = ((maxs[0] - mins[0]).abs() * 0.5).max(1.0) * TO_JOLT;
                let total_height = (maxs[2] - mins[2]).abs().max(4.0) * TO_JOLT;
                let half_height = ((total_height - 2.0 * radius) * 0.5).max(0.01);
                Shape::new_capsule(half_height, radius).ok()
            }
            _ => {
                let hx = ((maxs[0] - mins[0]).abs() * 0.5).max(1.0) * TO_JOLT;
                let hy = ((maxs[1] - mins[1]).abs() * 0.5).max(1.0) * TO_JOLT;
                let hz = ((maxs[2] - mins[2]).abs() * 0.5).max(1.0) * TO_JOLT;
                Shape::new_box(Vec3::new(hx, hy, hz)).ok()
            }
        };

        let shape = match shape {
            Some(s) => s,
            None => return false,
        };

        let pos = RVec3::new(
            origin[0] * TO_JOLT,
            origin[1] * TO_JOLT,
            origin[2] * TO_JOLT,
        );
        let lin_vel = Vec3::new(
            velocity[0] * TO_JOLT,
            velocity[1] * TO_JOLT,
            velocity[2] * TO_JOLT,
        );

        let mut settings = BodySettings::new_dynamic()
            .position(pos)
            .linear_velocity(lin_vel);

        if mass > 0.0 {
            settings = settings.mass(mass);
        }

        match world.create_body(&shape, &settings) {
            Ok(body_id) => {
                self.entities.insert(ent_index, body_id);
                true
            }
            Err(_) => false,
        }
    }

    pub fn get_transform(
        &self,
        world: &PhysicsWorld,
        ent_index: i32,
        origin_out: &mut [f32; 3],
        angles_out: &mut [f32; 3],
    ) -> bool {
        let body_id = match self.entities.get(&ent_index) {
            Some(&id) => id,
            None => return false,
        };

        if let Ok(body) = world.body(body_id) {
            let p = body.position();
            origin_out[0] = p.x * TO_GOLDSRC;
            origin_out[1] = p.y * TO_GOLDSRC;
            origin_out[2] = p.z * TO_GOLDSRC;

            let q = body.rotation();
            let (pitch, yaw, roll) = quat_to_euler(q);
            angles_out[0] = pitch;
            angles_out[1] = yaw;
            angles_out[2] = roll;
            true
        } else {
            false
        }
    }

    pub fn set_transform(
        &mut self,
        world: &mut PhysicsWorld,
        ent_index: i32,
        origin: [f32; 3],
        angles: [f32; 3],
    ) -> bool {
        let body_id = match self.entities.get(&ent_index) {
            Some(&id) => id,
            None => return false,
        };

        let pos = RVec3::new(
            origin[0] * TO_JOLT,
            origin[1] * TO_JOLT,
            origin[2] * TO_JOLT,
        );
        let q = euler_to_quat(angles[0], angles[1], angles[2]);

        if let Ok(mut body) = world.body_mut(body_id) {
            let _ = body.set_position(pos, Activation::Activate);
            let _ = body.set_rotation(q, Activation::Activate);
            true
        } else {
            false
        }
    }

    pub fn get_velocity(
        &self,
        world: &PhysicsWorld,
        ent_index: i32,
        velocity_out: &mut [f32; 3],
    ) -> bool {
        let body_id = match self.entities.get(&ent_index) {
            Some(&id) => id,
            None => return false,
        };

        if let Ok(body) = world.body(body_id) {
            let v = body.linear_velocity();
            velocity_out[0] = v.x * TO_GOLDSRC;
            velocity_out[1] = v.y * TO_GOLDSRC;
            velocity_out[2] = v.z * TO_GOLDSRC;
            true
        } else {
            false
        }
    }

    pub fn set_velocity(
        &mut self,
        world: &mut PhysicsWorld,
        ent_index: i32,
        velocity: [f32; 3],
    ) -> bool {
        let body_id = match self.entities.get(&ent_index) {
            Some(&id) => id,
            None => return false,
        };

        let v = Vec3::new(
            velocity[0] * TO_JOLT,
            velocity[1] * TO_JOLT,
            velocity[2] * TO_JOLT,
        );

        if let Ok(mut body) = world.body_mut(body_id) {
            let _ = body.set_linear_velocity(v);
            true
        } else {
            false
        }
    }
}

/// Convert Jolt quaternion to GoldSrc Euler angles (pitch, yaw, roll in degrees)
fn quat_to_euler(q: Quat) -> (f32, f32, f32) {
    let sinr_cosp = 2.0 * (q.w * q.x + q.y * q.z);
    let cosr_cosp = 1.0 - 2.0 * (q.x * q.x + q.y * q.y);
    let roll = sinr_cosp.atan2(cosr_cosp).to_degrees();

    let sinp = 2.0 * (q.w * q.y - q.z * q.x);
    let pitch = if sinp.abs() >= 1.0 {
        sinp.signum() * 90.0
    } else {
        sinp.asin().to_degrees()
    };

    let siny_cosp = 2.0 * (q.w * q.z + q.x * q.y);
    let cosy_cosp = 1.0 - 2.0 * (q.y * q.y + q.z * q.z);
    let yaw = siny_cosp.atan2(cosy_cosp).to_degrees();

    (pitch, yaw, roll)
}

/// Convert GoldSrc Euler angles (pitch, yaw, roll in degrees) to normalized quaternion
fn euler_to_quat(pitch_deg: f32, yaw_deg: f32, roll_deg: f32) -> Quat {
    let p = pitch_deg.to_radians() * 0.5;
    let y = yaw_deg.to_radians() * 0.5;
    let r = roll_deg.to_radians() * 0.5;

    let cr = r.cos();
    let sr = r.sin();
    let cp = p.cos();
    let sp = p.sin();
    let cy = y.cos();
    let sy = y.sin();

    let mut q = Quat::from_xyzw(
        sr * cp * cy - cr * sp * sy,
        cr * sp * cy + sr * cp * sy,
        cr * cp * sy - sr * sp * cy,
        cr * cp * cy + sr * sp * sy,
    );

    let len = (q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w).sqrt();
    if len > 1e-6 {
        q.x /= len;
        q.y /= len;
        q.z /= len;
        q.w /= len;
    } else {
        q = Quat::IDENTITY;
    }

    q
}
