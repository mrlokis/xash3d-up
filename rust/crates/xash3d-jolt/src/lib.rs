//! Xash3D Jolt Physics Engine - Rust Integration & C FFI Library.

pub mod bsp_converter;
pub mod character;
pub mod bodies;
pub mod trace;
pub mod world;

use std::slice;
use xash3d_api::{BrushFfi, PlaneFfi, TraceResultFfi};
use world::lock_system;

#[no_mangle]
pub unsafe extern "C" fn xash_rust_jolt_init() -> i32 {
    if world::init() { 1 } else { 0 }
}

#[no_mangle]
pub unsafe extern "C" fn xash_rust_jolt_shutdown() {
    world::shutdown();
}

#[no_mangle]
pub unsafe extern "C" fn xash_rust_jolt_level_init(
    brushes: *const BrushFfi,
    num_brushes: u32,
    planes: *const PlaneFfi,
    num_planes: u32,
) -> i32 {
    if brushes.is_null() || planes.is_null() {
        return 0;
    }

    let brush_slice = slice::from_raw_parts(brushes, num_brushes as usize);
    let plane_slice = slice::from_raw_parts(planes, num_planes as usize);

    let mut lock = lock_system();
    if let Some(sys) = lock.as_mut() {
        if sys.level_init(brush_slice, plane_slice) { 1 } else { 0 }
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn xash_rust_jolt_level_init_mesh(
    vertices: *const f32,
    num_vertices: u32,
    indices: *const u32,
    num_triangles: u32,
) -> i32 {
    if vertices.is_null() || indices.is_null() || num_vertices == 0 || num_triangles == 0 {
        return 0;
    }

    let vert_slice = slice::from_raw_parts(vertices as *const [f32; 3], num_vertices as usize);
    let ind_slice = slice::from_raw_parts(indices as *const [u32; 3], num_triangles as usize);

    let mut lock = lock_system();
    if let Some(sys) = lock.as_mut() {
        if sys.level_init_mesh(vert_slice, ind_slice) { 1 } else { 0 }
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn xash_rust_jolt_level_free() {
    let mut lock = lock_system();
    if let Some(sys) = lock.as_mut() {
        sys.level_free();
    }
}

#[no_mangle]
pub unsafe extern "C" fn xash_rust_jolt_step(dt: f32) -> i32 {
    let mut lock = lock_system();
    if let Some(sys) = lock.as_mut() {
        if sys.step(dt) { 1 } else { 0 }
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn xash_rust_jolt_character_update(
    player_index: i32,
    origin: *mut f32,
    velocity: *mut f32,
    angles: *const f32,
    buttons: i32,
    forwardmove: f32,
    sidemove: f32,
    upmove: f32,
    maxspeed: f32,
    flags: *mut i32,
    frametime: f32,
    gravity: f32,
) -> i32 {
    if origin.is_null() || velocity.is_null() || angles.is_null() || flags.is_null() {
        return 0;
    }

    let origin_arr = slice::from_raw_parts_mut(origin, 3);
    let velocity_arr = slice::from_raw_parts_mut(velocity, 3);
    let angles_arr = slice::from_raw_parts(angles, 3);

    let mut origin_buf = [origin_arr[0], origin_arr[1], origin_arr[2]];
    let mut velocity_buf = [velocity_arr[0], velocity_arr[1], velocity_arr[2]];
    let angles_buf = [angles_arr[0], angles_arr[1], angles_arr[2]];
    let mut flags_val = *flags;

    let mut lock = lock_system();
    if let Some(sys) = lock.as_mut() {
        let res = sys.characters.update(
            &mut sys.world,
            player_index,
            &mut origin_buf,
            &mut velocity_buf,
            &angles_buf,
            buttons,
            forwardmove,
            sidemove,
            upmove,
            maxspeed,
            &mut flags_val,
            frametime,
            gravity,
        );

        if res {
            origin_arr[0] = origin_buf[0];
            origin_arr[1] = origin_buf[1];
            origin_arr[2] = origin_buf[2];

            velocity_arr[0] = velocity_buf[0];
            velocity_arr[1] = velocity_buf[1];
            velocity_arr[2] = velocity_buf[2];

            *flags = flags_val;
            1
        } else {
            0
        }
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn xash_rust_jolt_character_remove(player_index: i32) {
    let mut lock = lock_system();
    if let Some(sys) = lock.as_mut() {
        sys.characters.remove_player(&mut sys.world, player_index);
    }
}

#[no_mangle]
pub unsafe extern "C" fn xash_rust_jolt_entity_create(
    ent_index: i32,
    shape_type: i32,
    mins: *const f32,
    maxs: *const f32,
    origin: *const f32,
    velocity: *const f32,
    mass: f32,
) -> i32 {
    if mins.is_null() || maxs.is_null() || origin.is_null() || velocity.is_null() {
        return 0;
    }

    let mins_slice = slice::from_raw_parts(mins, 3);
    let maxs_slice = slice::from_raw_parts(maxs, 3);
    let orig_slice = slice::from_raw_parts(origin, 3);
    let vel_slice = slice::from_raw_parts(velocity, 3);

    let mins_buf = [mins_slice[0], mins_slice[1], mins_slice[2]];
    let maxs_buf = [maxs_slice[0], maxs_slice[1], maxs_slice[2]];
    let orig_buf = [orig_slice[0], orig_slice[1], orig_slice[2]];
    let vel_buf = [vel_slice[0], vel_slice[1], vel_slice[2]];

    let mut lock = lock_system();
    if let Some(sys) = lock.as_mut() {
        if sys.entities.create(
            &mut sys.world,
            ent_index,
            shape_type,
            mins_buf,
            maxs_buf,
            orig_buf,
            vel_buf,
            mass,
        ) {
            1
        } else {
            0
        }
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn xash_rust_jolt_entity_destroy(ent_index: i32) {
    let mut lock = lock_system();
    if let Some(sys) = lock.as_mut() {
        sys.entities.destroy(&mut sys.world, ent_index);
    }
}

#[no_mangle]
pub unsafe extern "C" fn xash_rust_jolt_entity_get_transform(
    ent_index: i32,
    origin: *mut f32,
    angles: *mut f32,
) -> i32 {
    if origin.is_null() || angles.is_null() {
        return 0;
    }

    let origin_arr = slice::from_raw_parts_mut(origin, 3);
    let angles_arr = slice::from_raw_parts_mut(angles, 3);

    let mut orig_buf = [0.0f32; 3];
    let mut ang_buf = [0.0f32; 3];

    let lock = lock_system();
    if let Some(sys) = lock.as_ref() {
        if sys.entities.get_transform(&sys.world, ent_index, &mut orig_buf, &mut ang_buf) {
            origin_arr.copy_from_slice(&orig_buf);
            angles_arr.copy_from_slice(&ang_buf);
            1
        } else {
            0
        }
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn xash_rust_jolt_entity_set_transform(
    ent_index: i32,
    origin: *const f32,
    angles: *const f32,
) -> i32 {
    if origin.is_null() || angles.is_null() {
        return 0;
    }

    let orig_slice = slice::from_raw_parts(origin, 3);
    let ang_slice = slice::from_raw_parts(angles, 3);

    let orig_buf = [orig_slice[0], orig_slice[1], orig_slice[2]];
    let ang_buf = [ang_slice[0], ang_slice[1], ang_slice[2]];

    let mut lock = lock_system();
    if let Some(sys) = lock.as_mut() {
        if sys.entities.set_transform(&mut sys.world, ent_index, orig_buf, ang_buf) {
            1
        } else {
            0
        }
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn xash_rust_jolt_entity_get_velocity(
    ent_index: i32,
    velocity: *mut f32,
) -> i32 {
    if velocity.is_null() {
        return 0;
    }

    let vel_arr = slice::from_raw_parts_mut(velocity, 3);
    let mut vel_buf = [0.0f32; 3];

    let lock = lock_system();
    if let Some(sys) = lock.as_ref() {
        if sys.entities.get_velocity(&sys.world, ent_index, &mut vel_buf) {
            vel_arr.copy_from_slice(&vel_buf);
            1
        } else {
            0
        }
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn xash_rust_jolt_entity_set_velocity(
    ent_index: i32,
    velocity: *const f32,
) -> i32 {
    if velocity.is_null() {
        return 0;
    }

    let vel_slice = slice::from_raw_parts(velocity, 3);
    let vel_buf = [vel_slice[0], vel_slice[1], vel_slice[2]];

    let mut lock = lock_system();
    if let Some(sys) = lock.as_mut() {
        if sys.entities.set_velocity(&mut sys.world, ent_index, vel_buf) {
            1
        } else {
            0
        }
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn xash_rust_jolt_trace_line(
    start: *const f32,
    end: *const f32,
    result: *mut TraceResultFfi,
) -> i32 {
    if start.is_null() || end.is_null() || result.is_null() {
        return 0;
    }

    let start_s = slice::from_raw_parts(start, 3);
    let end_s = slice::from_raw_parts(end, 3);

    let start_buf = [start_s[0], start_s[1], start_s[2]];
    let end_buf = [end_s[0], end_s[1], end_s[2]];

    let lock = lock_system();
    if let Some(sys) = lock.as_ref() {
        if trace::ray_cast(&sys.world, start_buf, end_buf, &mut *result) {
            1
        } else {
            0
        }
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn xash_rust_jolt_trace_hull(
    start: *const f32,
    end: *const f32,
    mins: *const f32,
    maxs: *const f32,
    result: *mut TraceResultFfi,
) -> i32 {
    if start.is_null() || end.is_null() || mins.is_null() || maxs.is_null() || result.is_null() {
        return 0;
    }

    let start_s = slice::from_raw_parts(start, 3);
    let end_s = slice::from_raw_parts(end, 3);
    let mins_s = slice::from_raw_parts(mins, 3);
    let maxs_s = slice::from_raw_parts(maxs, 3);

    let start_buf = [start_s[0], start_s[1], start_s[2]];
    let end_buf = [end_s[0], end_s[1], end_s[2]];
    let mins_buf = [mins_s[0], mins_s[1], mins_s[2]];
    let maxs_buf = [maxs_s[0], maxs_s[1], maxs_s[2]];

    let lock = lock_system();
    if let Some(sys) = lock.as_ref() {
        if trace::hull_cast(&sys.world, start_buf, end_buf, mins_buf, maxs_buf, &mut *result) {
            1
        } else {
            0
        }
    } else {
        0
    }
}
