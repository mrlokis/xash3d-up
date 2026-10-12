//! Virtual Character movement for Xash3D players via Jolt Physics.

use std::collections::HashMap;
use oxijolt::{
    CharacterId, CharacterSettings, ExtendedUpdateSettings,
    GroundState, PhysicsWorld, QueryFilter, RVec3, Shape, Vec3,
};
use xash3d_api::{FL_DUCKING, FL_ONGROUND, IN_DUCK, IN_JUMP};
use crate::bsp_converter::{TO_GOLDSRC, TO_JOLT};

/// Standing player dimensions in GoldSrc units:
/// Width/Depth: 32 (radius 16), Height: 72 (from -36 to +36)
const STANDING_RADIUS: f32 = 16.0 * TO_JOLT;      // 0.5m
const STANDING_HEIGHT: f32 = 72.0 * TO_JOLT;      // 2.25m
const STANDING_HALF_HEIGHT: f32 = (STANDING_HEIGHT - 2.0 * STANDING_RADIUS) * 0.5;

/// Crouching player dimensions in GoldSrc units:
/// Width/Depth: 32 (radius 16), Height: 36 (from -18 to +18)
const DUCKING_RADIUS: f32 = 16.0 * TO_JOLT;       // 0.5m
const DUCKING_HEIGHT: f32 = 36.0 * TO_JOLT;       // 1.125m
const DUCKING_HALF_HEIGHT: f32 = (DUCKING_HEIGHT - 2.0 * DUCKING_RADIUS) * 0.5;

/// Jump velocity impulse in GoldSrc units.
const JUMP_VELOCITY: f32 = 270.0;

/// Default GoldSrc physics movement parameters
const FRICTION: f32 = 4.0;
const STOP_SPEED: f32 = 100.0;
const ACCELERATE: f32 = 10.0;
const AIR_ACCELERATE: f32 = 10.0;

pub struct CharacterManager {
    characters: HashMap<i32, CharacterId>,
    standing_shape: Shape,
    #[allow(dead_code)]
    ducking_shape: Shape,
}

impl CharacterManager {
    pub fn new() -> Result<Self, oxijolt::ShapeError> {
        let standing_shape = Shape::new_capsule(STANDING_HALF_HEIGHT, STANDING_RADIUS)?;
        let ducking_shape = Shape::new_capsule(DUCKING_HALF_HEIGHT, DUCKING_RADIUS)?;
        Ok(Self {
            characters: HashMap::new(),
            standing_shape,
            ducking_shape,
        })
    }

    pub fn remove_player(&mut self, world: &mut PhysicsWorld, player_index: i32) {
        if let Some(char_id) = self.characters.remove(&player_index) {
            let _ = world.remove_character(char_id);
        }
    }

    pub fn clear(&mut self, world: &mut PhysicsWorld) {
        for (_, char_id) in self.characters.drain() {
            let _ = world.remove_character(char_id);
        }
    }

    pub fn get_or_create(
        &mut self,
        world: &mut PhysicsWorld,
        player_index: i32,
        origin_gs: [f32; 3],
    ) -> Result<CharacterId, oxijolt::CharacterError> {
        if let Some(&id) = self.characters.get(&player_index) {
            if world.contains_character(id) {
                return Ok(id);
            }
        }

        let pos = RVec3::new(
            origin_gs[0] * TO_JOLT,
            origin_gs[1] * TO_JOLT,
            origin_gs[2] * TO_JOLT,
        );

        let settings = CharacterSettings::new(&self.standing_shape)
            .up(Vec3::new(0.0, 0.0, 1.0))
            .supporting_volume(Vec3::new(0.0, 0.0, 1.0), -1.0e10)
            .shape_offset(Vec3::new(0.0, 0.0, 0.0))
            .character_padding(0.02);

        let id = world.create_character(&settings, pos, oxijolt::Quat::IDENTITY)?;
        self.characters.insert(player_index, id);
        Ok(id)
    }

    /// Full character controller update replacing GoldSrc pmove
    pub fn update(
        &mut self,
        world: &mut PhysicsWorld,
        player_index: i32,
        origin: &mut [f32; 3],
        velocity: &mut [f32; 3],
        angles: &[f32; 3],
        buttons: i32,
        forwardmove: f32,
        sidemove: f32,
        _upmove: f32,
        maxspeed: f32,
        flags: &mut i32,
        frametime: f32,
        gravity: f32,
    ) -> bool {
        if frametime <= 0.0 {
            return true;
        }

        let char_id = match self.get_or_create(world, player_index, *origin) {
            Ok(id) => id,
            Err(_) => return false,
        };

        // Teleport check: if engine origin differed by more than 64 units, snap Jolt character
        let current_pos = match world.character(char_id) {
            Ok(c) => c.position(),
            Err(_) => return false,
        };
        let current_pos_gs = [
            current_pos.x * TO_GOLDSRC,
            current_pos.y * TO_GOLDSRC,
            current_pos.z * TO_GOLDSRC,
        ];
        let dx = origin[0] - current_pos_gs[0];
        let dy = origin[1] - current_pos_gs[1];
        let dz = origin[2] - current_pos_gs[2];
        if (dx * dx + dy * dy + dz * dz) > 64.0 * 64.0 {
            let new_pos = RVec3::new(
                origin[0] * TO_JOLT,
                origin[1] * TO_JOLT,
                origin[2] * TO_JOLT,
            );
            if let Ok(mut c_mut) = world.character_mut(char_id) {
                let _ = c_mut.set_position(new_pos);
            }
        }

        let on_ground = (*flags & FL_ONGROUND) != 0;
        let mut vel = *velocity;

        if (buttons & IN_DUCK) != 0 {
            *flags |= FL_DUCKING;
        } else {
            *flags &= !FL_DUCKING;
        }

        // Jump impulse
        let is_jumping = (buttons & IN_JUMP) != 0 && on_ground;
        if is_jumping {
            vel[2] = JUMP_VELOCITY;
            *flags &= !FL_ONGROUND;
        }

        // Horizontal wish direction from yaw angle
        let yaw_rad = angles[1].to_radians();
        let fwd_x = yaw_rad.cos();
        let fwd_y = yaw_rad.sin();
        let right_x = fwd_y;
        let right_y = -fwd_x;

        let mut wish_x = fwd_x * forwardmove + right_x * sidemove;
        let mut wish_y = fwd_y * forwardmove + right_y * sidemove;
        let wish_speed = (wish_x * wish_x + wish_y * wish_y).sqrt();

        let max_speed = if maxspeed > 0.0 { maxspeed } else { 320.0 };
        let clamped_wish_speed = wish_speed.min(max_speed);

        if wish_speed > 0.001 {
            wish_x /= wish_speed;
            wish_y /= wish_speed;
        }

        if (*flags & FL_ONGROUND) != 0 {
            // Apply friction
            let speed = (vel[0] * vel[0] + vel[1] * vel[1]).sqrt();
            if speed > 0.1 {
                let control = speed.max(STOP_SPEED);
                let drop = control * FRICTION * frametime;
                let newspeed = (speed - drop).max(0.0);
                vel[0] = (vel[0] / speed) * newspeed;
                vel[1] = (vel[1] / speed) * newspeed;
            } else {
                vel[0] = 0.0;
                vel[1] = 0.0;
            }

            // Ground acceleration
            if clamped_wish_speed > 0.0 {
                let current_speed = vel[0] * wish_x + vel[1] * wish_y;
                let add_speed = clamped_wish_speed - current_speed;
                if add_speed > 0.0 {
                    let mut accel_speed = ACCELERATE * clamped_wish_speed * frametime;
                    if accel_speed > add_speed {
                        accel_speed = add_speed;
                    }
                    vel[0] += accel_speed * wish_x;
                    vel[1] += accel_speed * wish_y;
                }
            }
        } else {
            // Air acceleration
            if clamped_wish_speed > 0.0 {
                let wish_air_speed = clamped_wish_speed.min(30.0);
                let current_speed = vel[0] * wish_x + vel[1] * wish_y;
                let add_speed = wish_air_speed - current_speed;
                if add_speed > 0.0 {
                    let mut accel_speed = AIR_ACCELERATE * clamped_wish_speed * frametime;
                    if accel_speed > add_speed {
                        accel_speed = add_speed;
                    }
                    vel[0] += accel_speed * wish_x;
                    vel[1] += accel_speed * wish_y;
                }
            }

            // Gravity in air
            vel[2] -= gravity * frametime;
        }

        // Set Jolt linear velocity
        let jolt_vel = Vec3::new(
            vel[0] * TO_JOLT,
            vel[1] * TO_JOLT,
            vel[2] * TO_JOLT,
        );

        if let Ok(mut c_mut) = world.character_mut(char_id) {
            let _ = c_mut.set_linear_velocity(jolt_vel);
        }

        // Extended update settings (stairs and floor sticking)
        let step_height_jolt = 18.0 * TO_JOLT;
        let stick_to_floor = if is_jumping {
            Vec3::ZERO
        } else {
            Vec3::new(0.0, 0.0, -step_height_jolt)
        };

        let ext_settings = ExtendedUpdateSettings::default()
            .stick_to_floor_step_down(stick_to_floor)
            .walk_stairs_step_up(Vec3::new(0.0, 0.0, step_height_jolt));

        let jolt_gravity = Vec3::new(0.0, 0.0, -gravity * TO_JOLT);
        let _ = world.update_character(
            char_id,
            frametime,
            jolt_gravity,
            &ext_settings,
            &QueryFilter::new(),
        );

        // Read updated position, velocity, and ground state
        if let Ok(c_ref) = world.character(char_id) {
            let p = c_ref.position();
            let v = c_ref.linear_velocity();
            let g = c_ref.ground_state();

            origin[0] = p.x * TO_GOLDSRC;
            origin[1] = p.y * TO_GOLDSRC;
            origin[2] = p.z * TO_GOLDSRC;

            velocity[0] = v.x * TO_GOLDSRC;
            velocity[1] = v.y * TO_GOLDSRC;
            velocity[2] = v.z * TO_GOLDSRC;

            if g == GroundState::OnGround {
                *flags |= FL_ONGROUND;
                velocity[2] = 0.0;
            } else {
                *flags &= !FL_ONGROUND;
            }
        }

        true
    }
}
