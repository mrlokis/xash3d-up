//! Collision queries and ray casting for Xash3D using Jolt Physics.

use oxijolt::{
    PhysicsWorld, QueryFilter, Quat, RVec3, RayCast, Shape, ShapeCast, Vec3,
};
use xash3d_api::TraceResultFfi;
use crate::bsp_converter::TO_JOLT;

/// Perform a ray cast from start to end in GoldSrc units
pub fn ray_cast(
    world: &PhysicsWorld,
    start: [f32; 3],
    end: [f32; 3],
    result: &mut TraceResultFfi,
) -> bool {
    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    let dz = end[2] - start[2];
    let dist_sq = dx * dx + dy * dy + dz * dz;

    if dist_sq < 1e-4 {
        result.fraction = 1.0;
        result.endpos = end;
        result.allsolid = 0;
        result.startsolid = 0;
        result.inopen = 1;
        result.inwater = 0;
        result.hit_ent = 0;
        return true;
    }

    let origin = RVec3::new(
        start[0] * TO_JOLT,
        start[1] * TO_JOLT,
        start[2] * TO_JOLT,
    );
    let direction = Vec3::new(
        dx * TO_JOLT,
        dy * TO_JOLT,
        dz * TO_JOLT,
    );

    let ray = RayCast::new(origin, direction);
    match world.cast_ray(&ray, &QueryFilter::new()) {
        Ok(Some(hit)) => {
            result.fraction = hit.fraction;
            result.endpos = [
                start[0] + dx * hit.fraction,
                start[1] + dy * hit.fraction,
                start[2] + dz * hit.fraction,
            ];
            result.plane_normal = [hit.normal.x, hit.normal.y, hit.normal.z];
            result.plane_dist = result.endpos[0] * hit.normal.x
                + result.endpos[1] * hit.normal.y
                + result.endpos[2] * hit.normal.z;
            result.allsolid = if hit.fraction == 0.0 { 1 } else { 0 };
            result.startsolid = if hit.fraction == 0.0 { 1 } else { 0 };
            result.inopen = 1;
            result.inwater = 0;
            result.hit_ent = hit.body.index() as i32;
            true
        }
        Ok(None) => {
            result.fraction = 1.0;
            result.endpos = end;
            result.allsolid = 0;
            result.startsolid = 0;
            result.inopen = 1;
            result.inwater = 0;
            result.hit_ent = 0;
            true
        }
        Err(_) => false,
    }
}

/// Perform a swept hull / box trace from start to end
pub fn hull_cast(
    world: &PhysicsWorld,
    start: [f32; 3],
    end: [f32; 3],
    mins: [f32; 3],
    maxs: [f32; 3],
    result: &mut TraceResultFfi,
) -> bool {
    let dx_ext = (maxs[0] - mins[0]).abs();
    let dy_ext = (maxs[1] - mins[1]).abs();
    let dz_ext = (maxs[2] - mins[2]).abs();

    if dx_ext < 0.1 && dy_ext < 0.1 && dz_ext < 0.1 {
        return ray_cast(world, start, end, result);
    }

    let hx = (dx_ext * 0.5) * TO_JOLT;
    let hy = (dy_ext * 0.5) * TO_JOLT;
    let hz = (dz_ext * 0.5) * TO_JOLT;

    let shape = match Shape::new_box(Vec3::new(hx, hy, hz)) {
        Ok(s) => s,
        Err(_) => return ray_cast(world, start, end, result),
    };

    let offset_x = (mins[0] + maxs[0]) * 0.5 * TO_JOLT;
    let offset_y = (mins[1] + maxs[1]) * 0.5 * TO_JOLT;
    let offset_z = (mins[2] + maxs[2]) * 0.5 * TO_JOLT;

    let origin = RVec3::new(
        start[0] * TO_JOLT + offset_x,
        start[1] * TO_JOLT + offset_y,
        start[2] * TO_JOLT + offset_z,
    );

    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    let dz = end[2] - start[2];

    let direction = Vec3::new(
        dx * TO_JOLT,
        dy * TO_JOLT,
        dz * TO_JOLT,
    );

    let shape_cast = ShapeCast::new(&shape, origin, Quat::IDENTITY, direction);
    match world.cast_shape(&shape_cast, &QueryFilter::new()) {
        Ok(Some(hit)) => {
            result.fraction = hit.fraction;
            result.endpos = [
                start[0] + dx * hit.fraction,
                start[1] + dy * hit.fraction,
                start[2] + dz * hit.fraction,
            ];
            result.plane_normal = [hit.normal.x, hit.normal.y, hit.normal.z];
            result.plane_dist = result.endpos[0] * hit.normal.x
                + result.endpos[1] * hit.normal.y
                + result.endpos[2] * hit.normal.z;
            result.allsolid = if hit.fraction == 0.0 { 1 } else { 0 };
            result.startsolid = if hit.fraction == 0.0 { 1 } else { 0 };
            result.inopen = 1;
            result.inwater = 0;
            result.hit_ent = hit.body.index() as i32;
            true
        }
        Ok(None) => {
            result.fraction = 1.0;
            result.endpos = end;
            result.allsolid = 0;
            result.startsolid = 0;
            result.inopen = 1;
            result.inwater = 0;
            result.hit_ent = 0;
            true
        }
        Err(_) => false,
    }
}
