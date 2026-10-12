//! Central physics world management for Xash3D using Jolt.

use std::sync::{Mutex, MutexGuard};
use oxijolt::{
    BodyId, BodySettings, PhysicsWorld, RVec3, Shape, Vec3, WorldSettings,
};
use xash3d_api::{BrushFfi, PlaneFfi};
use crate::bsp_converter::{build_world_shapes, TO_JOLT};
use crate::character::CharacterManager;
use crate::bodies::EntityManager;

pub struct JoltPhysicsSystem {
    pub world: PhysicsWorld,
    pub brush_bodies: Vec<BodyId>,
    pub characters: CharacterManager,
    pub entities: EntityManager,
}

impl JoltPhysicsSystem {
    pub fn new() -> Result<Self, oxijolt::WorldError> {
        let settings = WorldSettings::default()
            .gravity(Vec3::new(0.0, 0.0, -800.0 * TO_JOLT))
            .max_bodies(32768)
            .max_body_pairs(131072)
            .max_contact_constraints(32768);

        let world = PhysicsWorld::new(settings)?;
        let characters = CharacterManager::new()
            .map_err(|_| oxijolt::WorldError::AllocationFailed("character manager"))?;
        let entities = EntityManager::new();

        Ok(Self {
            world,
            brush_bodies: Vec::new(),
            characters,
            entities,
        })
    }

    pub fn level_init(&mut self, brushes: &[BrushFfi], planes: &[PlaneFfi]) -> bool {
        self.level_free();

        let shapes = build_world_shapes(brushes, planes);
        for (shape, center) in shapes {
            let pos = RVec3::new(center.x, center.y, center.z);
            let settings = BodySettings::new_static().position(pos);
            if let Ok(body_id) = self.world.create_body(&shape, &settings) {
                self.brush_bodies.push(body_id);
            }
        }

        self.world.optimize_broad_phase();
        true
    }

    pub fn level_init_mesh(&mut self, vertices_gs: &[[f32; 3]], indices: &[[u32; 3]]) -> bool {
        self.level_free();

        let vertices_jolt: Vec<Vec3> = vertices_gs
            .iter()
            .map(|v| Vec3::new(v[0] * TO_JOLT, v[1] * TO_JOLT, v[2] * TO_JOLT))
            .collect();

        if let Ok((mesh_shape, _)) = Shape::new_mesh(&vertices_jolt, indices) {
            let pos = RVec3::ZERO;
            let settings = BodySettings::new_static().position(pos);
            if let Ok(body_id) = self.world.create_body(&mesh_shape, &settings) {
                self.brush_bodies.push(body_id);
                self.world.optimize_broad_phase();
                return true;
            }
        }

        false
    }

    pub fn level_free(&mut self) {
        self.characters.clear(&mut self.world);
        self.entities.clear(&mut self.world);

        for body_id in self.brush_bodies.drain(..) {
            let _ = self.world.remove_body(body_id);
        }
    }

    pub fn step(&mut self, dt: f32) -> bool {
        if dt <= 0.0 {
            return true;
        }

        // Clamp dt between 1ms and 100ms
        let clamped_dt = dt.clamp(0.001, 0.1);
        self.world.step(clamped_dt).is_ok()
    }
}

static SYSTEM: Mutex<Option<JoltPhysicsSystem>> = Mutex::new(None);

pub fn lock_system() -> MutexGuard<'static, Option<JoltPhysicsSystem>> {
    SYSTEM.lock().unwrap()
}

pub fn init() -> bool {
    let mut lock = lock_system();
    if lock.is_some() {
        return true;
    }

    match JoltPhysicsSystem::new() {
        Ok(sys) => {
            *lock = Some(sys);
            true
        }
        Err(_) => false,
    }
}

pub fn shutdown() {
    let mut lock = lock_system();
    if let Some(mut sys) = lock.take() {
        sys.level_free();
    }
}
