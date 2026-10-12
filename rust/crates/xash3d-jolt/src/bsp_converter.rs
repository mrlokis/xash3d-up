//! Converts BSP brushes into Jolt collision shapes.

use oxijolt::{Shape, Vec3};
use xash3d_api::{BrushFfi, PlaneFfi, Vector3};

pub const TO_JOLT: f32 = 1.0 / 32.0;
pub const TO_GOLDSRC: f32 = 32.0;

const EPSILON: f32 = 0.1;

#[inline]
fn to_jolt_vec(v: Vector3) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}

/// Intersect three planes to find a potential polytope vertex in GoldSrc coordinates.
fn intersect_three_planes(p1: &PlaneFfi, p2: &PlaneFfi, p3: &PlaneFfi) -> Option<Vector3> {
    let n1 = Vector3::new(p1.normal[0], p1.normal[1], p1.normal[2]);
    let n2 = Vector3::new(p2.normal[0], p2.normal[1], p2.normal[2]);
    let n3 = Vector3::new(p3.normal[0], p3.normal[1], p3.normal[2]);

    let c23 = n2.cross(n3);
    let denom = n1.dot(c23);

    if denom.abs() < 1e-4 {
        return None;
    }

    let c31 = n3.cross(n1);
    let c12 = n1.cross(n2);

    let p = (c23 * p1.dist + c31 * p2.dist + c12 * p3.dist) * (1.0 / denom);
    Some(p)
}

/// Check if point is inside all bounding planes of the brush.
fn point_inside_brush(pt: Vector3, planes: &[PlaneFfi], mins: &[f32; 3], maxs: &[f32; 3]) -> bool {
    if pt.x < mins[0] - 1.0 || pt.x > maxs[0] + 1.0
        || pt.y < mins[1] - 1.0 || pt.y > maxs[1] + 1.0
        || pt.z < mins[2] - 1.0 || pt.z > maxs[2] + 1.0
    {
        return false;
    }

    for p in planes {
        let n = Vector3::new(p.normal[0], p.normal[1], p.normal[2]);
        if n.dot(pt) > p.dist + EPSILON {
            return false;
        }
    }
    true
}

/// Convert a single BSP brush into a Jolt Shape and its world center in Jolt space.
pub fn build_brush_shape(brush: &BrushFfi, all_planes: &[PlaneFfi]) -> Option<(Shape, Vec3)> {
    let start = brush.first_plane as usize;
    let end = start + brush.num_planes as usize;
    if start >= all_planes.len() || end > all_planes.len() {
        return None;
    }

    let planes = &all_planes[start..end];
    let num_planes = planes.len();

    let center_gs = Vector3::new(
        (brush.mins[0] + brush.maxs[0]) * 0.5,
        (brush.mins[1] + brush.maxs[1]) * 0.5,
        (brush.mins[2] + brush.maxs[2]) * 0.5,
    );
    let center_jolt = to_jolt_vec(center_gs * TO_JOLT);

    let mut points: Vec<Vector3> = Vec::new();

    if num_planes >= 4 && num_planes <= 32 {
        for i in 0..num_planes {
            for j in (i + 1)..num_planes {
                for k in (j + 1)..num_planes {
                    if let Some(pt) = intersect_three_planes(&planes[i], &planes[j], &planes[k]) {
                        if point_inside_brush(pt, planes, &brush.mins, &brush.maxs) {
                            if !points.iter().any(|existing| (*existing - pt).length_squared() < 0.01) {
                                points.push(pt);
                            }
                        }
                    }
                }
            }
        }
    }

    if points.len() >= 4 {
        let local_points: Vec<Vec3> = points
            .iter()
            .map(|p| to_jolt_vec((*p - center_gs) * TO_JOLT))
            .collect();
        if let Ok(hull) = Shape::new_convex_hull(&local_points) {
            return Some((hull, center_jolt));
        }
    }

    // Fallback: axis-aligned box
    let dx = (brush.maxs[0] - brush.mins[0]).abs();
    let dy = (brush.maxs[1] - brush.mins[1]).abs();
    let dz = (brush.maxs[2] - brush.mins[2]).abs();

    if dx > 0.5 && dy > 0.5 && dz > 0.5 {
        let half_extents = Vec3::new(dx * 0.5 * TO_JOLT, dy * 0.5 * TO_JOLT, dz * 0.5 * TO_JOLT);
        if let Ok(box_shape) = Shape::new_box(half_extents) {
            return Some((box_shape, center_jolt));
        }
    }

    None
}

/// Convert an array of brushes into a collection of Jolt shapes and positions.
pub fn build_world_shapes(brushes: &[BrushFfi], planes: &[PlaneFfi]) -> Vec<(Shape, Vec3)> {
    let mut result = Vec::new();

    for brush in brushes {
        if brush.contents == -2 /* CONTENTS_WATER */
            || brush.contents == -3 /* CONTENTS_SLIME */
            || brush.contents == -4 /* CONTENTS_LAVA */
            || brush.contents == -5 /* CONTENTS_SKY */
        {
            continue;
        }

        if let Some((shape, center)) = build_brush_shape(brush, planes) {
            result.push((shape, center));
        }
    }

    result
}
