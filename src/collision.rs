//! Shapes that can touch, and the one piece of movement the engine provides.
//!
//! See `specs/0014-collision.md`. Pure maths on the CPU: no GPU, no rigid
//! bodies, no solver. A game decides what a collision means.

use glam::{Quat, Vec3};

/// How far a slide stops short of a surface, so it does not end up inside it
/// and stick there.
const SKIN: f32 = 1e-3;

/// A box aligned to the world axes.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    /// Takes the corners in any order.
    pub fn new(a: Vec3, b: Vec3) -> Self {
        Self {
            min: a.min(b),
            max: a.max(b),
        }
    }

    pub fn from_center_size(center: Vec3, size: Vec3) -> Self {
        let half = size.abs() * 0.5;
        Self {
            min: center - half,
            max: center + half,
        }
    }

    /// An empty box that grows to fit whatever is added to it.
    pub fn empty() -> Self {
        Self {
            min: Vec3::splat(f32::INFINITY),
            max: Vec3::splat(f32::NEG_INFINITY),
        }
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn size(&self) -> Vec3 {
        (self.max - self.min).max(Vec3::ZERO)
    }

    pub fn contains_point(&self, point: Vec3) -> bool {
        point.cmpge(self.min).all() && point.cmple(self.max).all()
    }

    /// Touching exactly counts as overlapping, so a ball resting on the floor
    /// is in contact with it.
    pub fn intersects(&self, other: &Aabb) -> bool {
        self.min.cmple(other.max).all() && self.max.cmpge(other.min).all()
    }

    /// The point of this box nearest to `point`, which is the point itself when
    /// it is inside.
    pub fn closest_point(&self, point: Vec3) -> Vec3 {
        point.clamp(self.min, self.max)
    }

    pub fn expanded(&self, amount: Vec3) -> Self {
        Self {
            min: self.min - amount,
            max: self.max + amount,
        }
    }

    pub fn union_point(&self, point: Vec3) -> Self {
        Self {
            min: self.min.min(point),
            max: self.max.max(point),
        }
    }

    pub fn union(&self, other: &Aabb) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
}

impl Sphere {
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self {
            center,
            radius: radius.max(0.0),
        }
    }

    pub fn contains_point(&self, point: Vec3) -> bool {
        (point - self.center).length_squared() <= self.radius * self.radius
    }

    pub fn intersects(&self, other: &Sphere) -> bool {
        let reach = self.radius + other.radius;
        (other.center - self.center).length_squared() <= reach * reach
    }

    /// True when the box's nearest point is within reach, which covers a face,
    /// an edge and a corner alike.
    pub fn intersects_aabb(&self, box_: &Aabb) -> bool {
        let closest = box_.closest_point(self.center);
        (closest - self.center).length_squared() <= self.radius * self.radius
    }
}

/// Where something was hit, how far along, and which way the surface faces.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Hit {
    pub distance: f32,
    pub point: Vec3,
    pub normal: Vec3,
}

#[derive(Debug, Copy, Clone)]
pub struct Ray {
    pub origin: Vec3,
    /// Always a unit vector.
    pub direction: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize_or_zero(),
        }
    }

    pub fn at(&self, distance: f32) -> Vec3 {
        self.origin + self.direction * distance
    }

    /// The slab method: clip the ray against each pair of parallel faces and
    /// see whether anything is left.
    pub fn hit_aabb(&self, box_: &Aabb) -> Option<Hit> {
        let mut near = 0.0f32;
        let mut far = f32::INFINITY;
        let mut axis = 0;

        for i in 0..3 {
            let direction = self.direction[i];
            let origin = self.origin[i];

            if direction.abs() < f32::EPSILON {
                // parallel to this pair of faces, so it has to start between them
                if origin < box_.min[i] || origin > box_.max[i] {
                    return None;
                }
                continue;
            }

            let mut t1 = (box_.min[i] - origin) / direction;
            let mut t2 = (box_.max[i] - origin) / direction;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
            }

            if t1 > near {
                near = t1;
                axis = i;
            }
            far = far.min(t2);

            if near > far {
                return None;
            }
        }

        let mut normal = Vec3::ZERO;
        normal[axis] = if self.direction[axis] < 0.0 {
            1.0
        } else {
            -1.0
        };

        Some(Hit {
            distance: near,
            point: self.at(near),
            normal,
        })
    }

    pub fn hit_sphere(&self, sphere: &Sphere) -> Option<Hit> {
        let to_center = self.origin - sphere.center;
        let b = to_center.dot(self.direction);
        let c = to_center.length_squared() - sphere.radius * sphere.radius;

        // starting outside and pointing away
        if c > 0.0 && b > 0.0 {
            return None;
        }

        let discriminant = b * b - c;
        if discriminant < 0.0 {
            return None;
        }

        let distance = (-b - discriminant.sqrt()).max(0.0);
        let point = self.at(distance);

        Some(Hit {
            distance,
            point,
            normal: (point - sphere.center).normalize_or_zero(),
        })
    }
}

/// Moves a sphere along `movement` and reports the first box it touches.
///
/// A sphere sweeping past a box meets a shape with flat faces, rounded edges
/// and rounded corners: the box grown by the radius, with its corners filed
/// off. Growing the box and casting a ray at it gets the faces right and the
/// corners wrong, and the difference is not academic. A ball rolling level with
/// the top of a platform toward the next one is nowhere near touching it, but a
/// square corner says otherwise and stops it dead at the seam.
pub fn sweep_sphere(sphere: &Sphere, movement: Vec3, box_: &Aabb) -> Option<Hit> {
    let travel = movement.length();
    if travel < f32::EPSILON {
        return None;
    }

    let grown = box_.expanded(Vec3::splat(sphere.radius));

    // Already in contact, which is what resting on a floor is, and also what
    // sitting at its edge is: the sphere can be clear of the box while its
    // center is still inside the grown one. A ray from in there reports a hit
    // at no distance with whichever axis the slab test looked at first, which
    // would block rolling along a floor, and off the end of it, as surely as
    // falling through it. Decide by direction instead: into the surface is
    // blocked, away from it or along it is free.
    if grown.contains_point(sphere.center) {
        let away = nearest_face(&grown, sphere.center);

        return if movement.dot(away) >= 0.0 {
            None
        } else {
            Some(Hit {
                distance: 0.0,
                point: box_.closest_point(sphere.center),
                normal: away,
            })
        };
    }

    let ray = Ray::new(sphere.center, movement);
    let entry = ray.hit_aabb(&grown)?;
    if entry.distance > travel {
        return None;
    }

    // Which faces of the grown box the entry point sits beyond tells us what
    // the sphere is really about to meet: one axis is a flat face, two is a
    // rounded edge, three is a rounded corner.
    let point = ray.at(entry.distance);
    let mut beyond = [false; 3];
    let mut count = 0;
    for axis in 0..3 {
        if point[axis] < box_.min[axis] || point[axis] > box_.max[axis] {
            beyond[axis] = true;
            count += 1;
        }
    }

    if count <= 1 {
        return Some(Hit {
            distance: entry.distance,
            point,
            normal: entry.normal,
        });
    }

    // The corner or edge the sphere is heading for, as a point or a segment.
    let mut corner = Vec3::ZERO;
    for axis in 0..3 {
        corner[axis] = if point[axis] < box_.min[axis] {
            box_.min[axis]
        } else if point[axis] > box_.max[axis] {
            box_.max[axis]
        } else {
            point[axis]
        };
    }

    let distance = if count == 3 {
        // a corner: the rounded part is a sphere sitting on it
        ray.hit_sphere(&Sphere::new(corner, sphere.radius))
            .map(|hit| hit.distance)
    } else {
        // an edge: the rounded part is a cylinder lying along it
        let free = (0..3)
            .find(|axis| !beyond[*axis])
            .expect("two axes are beyond");
        sweep_against_edge(&ray, corner, free, box_, sphere.radius)
    }?;

    if distance > travel {
        return None;
    }

    let center = ray.at(distance);
    let normal = (center - box_.closest_point(center)).normalize_or_zero();

    Some(Hit {
        distance,
        point: box_.closest_point(center),
        normal: if normal.length_squared() < 0.5 {
            entry.normal
        } else {
            normal
        },
    })
}

/// How far along `ray` a sphere of `radius` first touches the box edge running
/// along `free` through `corner`.
fn sweep_against_edge(
    ray: &Ray,
    corner: Vec3,
    free: usize,
    box_: &Aabb,
    radius: f32,
) -> Option<f32> {
    // flatten out the axis the edge runs along: what is left is a circle
    let others: Vec<usize> = (0..3).filter(|axis| *axis != free).collect();
    let (a, b) = (others[0], others[1]);

    let ox = ray.origin[a] - corner[a];
    let oy = ray.origin[b] - corner[b];
    let dx = ray.direction[a];
    let dy = ray.direction[b];

    let quadratic_a = dx * dx + dy * dy;
    if quadratic_a < f32::EPSILON {
        // travelling straight along the edge, so it never closes in on it
        return None;
    }

    let half_b = ox * dx + oy * dy;
    let c = ox * ox + oy * oy - radius * radius;
    let discriminant = half_b * half_b - quadratic_a * c;
    if discriminant < 0.0 {
        return None;
    }

    let distance = (-half_b - discriminant.sqrt()) / quadratic_a;
    if distance < 0.0 {
        return None;
    }

    // it only counts if it lands on the edge itself rather than past its ends,
    // where a corner takes over
    let along = ray.origin[free] + ray.direction[free] * distance;
    if along < box_.min[free] || along > box_.max[free] {
        return ray
            .hit_sphere(&Sphere::new(nearest_end(corner, free, box_, along), radius))
            .map(|hit| hit.distance);
    }

    Some(distance)
}

fn nearest_end(corner: Vec3, free: usize, box_: &Aabb, along: f32) -> Vec3 {
    let mut end = corner;
    end[free] = if along < box_.min[free] {
        box_.min[free]
    } else {
        box_.max[free]
    };
    end
}

/// Which way out of a box a point is nearest to, as an outward normal.
///
/// This is the contact normal for a sphere already touching a box, taken from
/// the box grown by its radius. Taking it from the nearest point on the box
/// itself looks reasonable and is wrong: a ball resting level with the top of a
/// box reads as pressed into its side, so rolling from one platform onto
/// another at the same height gets blocked at the seam.
fn nearest_face(box_: &Aabb, point: Vec3) -> Vec3 {
    let to_min = point - box_.min;
    let to_max = box_.max - point;

    let mut normal = Vec3::Y;
    let mut nearest = f32::INFINITY;

    for axis in 0..3 {
        if to_min[axis] < nearest {
            nearest = to_min[axis];
            normal = Vec3::ZERO;
            normal[axis] = -1.0;
        }
        if to_max[axis] < nearest {
            nearest = to_max[axis];
            normal = Vec3::ZERO;
            normal[axis] = 1.0;
        }
    }

    normal
}

/// Moves a sphere by `velocity` for `dt`, sliding along whatever it hits rather
/// than stopping dead, and returns where it ends up.
///
/// It gives up after four surfaces, so a corner settles instead of ringing
/// between two walls forever.
pub fn move_and_slide(sphere: Sphere, velocity: Vec3, dt: f32, colliders: &[Aabb]) -> Vec3 {
    let mut position = sphere.center;
    let mut remaining = velocity * dt;

    for _ in 0..4 {
        if remaining.length_squared() < f32::EPSILON {
            break;
        }

        let moving = Sphere::new(position, sphere.radius);
        let hit = colliders
            .iter()
            .filter_map(|box_| sweep_sphere(&moving, remaining, box_))
            .min_by(|a, b| a.distance.total_cmp(&b.distance));

        let hit = match hit {
            Some(hit) => hit,
            None => {
                position += remaining;
                break;
            }
        };

        // stop just short of the surface, then carry the rest of the movement
        // along it rather than into it
        let travelled = (hit.distance - SKIN).max(0.0);
        let direction = remaining.normalize_or_zero();
        position += direction * travelled;

        let left = remaining.length() - travelled;
        let along = direction - hit.normal * direction.dot(hit.normal);
        remaining = along * left;
    }

    position
}

/// A box that can be turned, which an `Aabb` cannot. Spec 0035.
///
/// The static world is made of `Aabb`s and they are read as these with no
/// rotation, so a block against a wall and a block against a block are the same
/// test rather than two that have to be kept agreeing.
#[derive(Debug, Clone, Copy)]
pub struct Obb {
    pub at: Vec3,
    pub turn: Quat,
    /// Half the width, height and depth, in the box's own frame.
    pub half: Vec3,
}

impl Obb {
    pub fn new(at: Vec3, turn: Quat, half: Vec3) -> Self {
        Self { at, turn, half }
    }

    pub fn from_aabb(wall: &Aabb) -> Self {
        Self {
            at: wall.center(),
            turn: Quat::IDENTITY,
            half: wall.size() * 0.5,
        }
    }

    /// Which way one of its own three axes points, in the world.
    pub fn axis(&self, n: usize) -> Vec3 {
        self.turn
            * match n {
                0 => Vec3::X,
                1 => Vec3::Y,
                _ => Vec3::Z,
            }
    }

    /// How far it reaches from its middle along a direction, which is the
    /// shadow it casts on that line.
    fn reach_along(&self, way: Vec3) -> f32 {
        (0..3)
            .map(|n| (self.half[n] * self.axis(n).dot(way)).abs())
            .sum()
    }

    /// The corner furthest along a direction.
    fn furthest(&self, way: Vec3) -> Vec3 {
        (0..3).fold(self.at, |at, n| {
            at + self.axis(n) * self.half[n] * self.axis(n).dot(way).signum()
        })
    }
}

/// Where two shapes meet, and how far in. Spec 0035.
#[derive(Debug, Clone)]
pub struct Meeting {
    /// Points from the first towards the second, so pushing the second along it
    /// takes them apart.
    pub normal: Vec3,
    pub depth: f32,
    /// Up to four places they touch. A face resting on a face gives four, a
    /// face on an edge two, and an edge crossing an edge one. A single point
    /// cannot hold a box level: resolved at one corner it see-saws onto the
    /// next.
    pub points: Vec<Vec3>,
}

/// Only take a cross-product axis over a face axis when it is clearly better.
/// Two boxes lying square on each other have nine cross axes that tie with the
/// face, and a tie broken the wrong way gives one contact point where four were
/// wanted, which is a resting box that rocks.
const PREFER_A_FACE: f32 = 1.01;

/// Whether two boxes overlap, and where, by separating axes. Spec 0035.
///
/// Fifteen of them: three faces of each, and the nine cross products of their
/// edge directions. The cross ones are what catch an edge landing on an edge,
/// and leaving them out is what lets a box sink corner first into another one.
pub fn obbs_meet(one: &Obb, other: &Obb) -> Option<Meeting> {
    let between = other.at - one.at;

    let mut least = f32::INFINITY;
    let mut normal = Vec3::ZERO;
    let mut which = usize::MAX;

    for n in 0..15 {
        let axis = if n < 3 {
            one.axis(n)
        } else if n < 6 {
            other.axis(n - 3)
        } else {
            one.axis((n - 6) / 3).cross(other.axis((n - 6) % 3))
        };

        // two boxes square on each other have parallel edges, and the cross of
        // two parallel directions is nothing rather than an axis
        if axis.length_squared() < 1e-8 {
            continue;
        }

        let way = axis.normalize();
        let overlap = one.reach_along(way) + other.reach_along(way) - between.dot(way).abs();
        if overlap <= 0.0 {
            return None;
        }

        let judged = if n < 6 {
            overlap
        } else {
            overlap * PREFER_A_FACE
        };
        if judged < least {
            least = judged;
            which = n;
            normal = if between.dot(way) < 0.0 { -way } else { way };
        }
    }

    if which == usize::MAX {
        return None;
    }

    let depth = one.reach_along(normal) + other.reach_along(normal) - between.dot(normal).abs();

    let points = if which < 3 {
        face_points(one, other, normal)
    } else if which < 6 {
        face_points(other, one, -normal)
    } else {
        edge_point(one, other, normal, (which - 6) / 3, (which - 6) % 3)
    };

    Some(Meeting {
        normal,
        depth,
        points,
    })
}

/// The one place two crossing edges touch: the nearest points of the two edges
/// that are furthest along the normal.
fn edge_point(one: &Obb, other: &Obb, normal: Vec3, i: usize, j: usize) -> Vec<Vec3> {
    let here = one.furthest(normal) - one.axis(i) * one.half[i] * one.axis(i).dot(normal).signum();
    let there = other.furthest(-normal)
        - other.axis(j) * other.half[j] * other.axis(j).dot(-normal).signum();

    let (u, v) = (one.axis(i), other.axis(j));
    let w = here - there;
    let (b, d, e) = (u.dot(v), u.dot(w), v.dot(w));
    let denominator = 1.0 - b * b;

    if denominator.abs() < 1e-6 {
        return vec![(here + there) * 0.5];
    }

    let s = (b * e - d) / denominator;
    let t = (e - b * d) / denominator;
    let on_one = here + u * s.clamp(-one.half[i], one.half[i]);
    let on_other = there + v * t.clamp(-other.half[j], other.half[j]);

    vec![(on_one + on_other) * 0.5]
}

/// The patch where a face of `reference` is met by whatever of `incident` is
/// nearest it, found by clipping one against the sides of the other.
///
/// `normal` points from the reference towards the incident.
fn face_points(reference: &Obb, incident: &Obb, normal: Vec3) -> Vec<Vec3> {
    let out = (0..3)
        .max_by(|a, b| {
            reference
                .axis(*a)
                .dot(normal)
                .abs()
                .total_cmp(&reference.axis(*b).dot(normal).abs())
        })
        .unwrap_or(0);
    let facing = reference.axis(out) * reference.axis(out).dot(normal).signum();
    let plane = reference.at + facing * reference.half[out];

    // the face of the incident box that looks back at it most squarely
    let into = (0..3)
        .min_by(|a, b| {
            incident
                .axis(*a)
                .dot(normal)
                .abs()
                .total_cmp(&incident.axis(*b).dot(normal).abs())
                .reverse()
        })
        .unwrap_or(0);
    let back = -incident.axis(into) * incident.axis(into).dot(normal).signum();
    let (p, q) = ((into + 1) % 3, (into + 2) % 3);

    let middle = incident.at + back * incident.half[into];
    let (across, along) = (
        incident.axis(p) * incident.half[p],
        incident.axis(q) * incident.half[q],
    );
    let mut polygon = vec![
        middle - across - along,
        middle + across - along,
        middle + across + along,
        middle - across + along,
    ];

    // clipped against the four sides of the reference face, which is what keeps
    // a face hanging over an edge from reporting contact out in mid air
    for n in 0..3 {
        if n == out {
            continue;
        }

        for side in [1.0f32, -1.0] {
            let edge = reference.axis(n) * side;
            polygon = clipped(&polygon, edge, reference.at.dot(edge) + reference.half[n]);
            if polygon.is_empty() {
                return Vec::new();
            }
        }
    }

    // and then only the ones actually at or under the face
    let mut touching: Vec<Vec3> = polygon
        .into_iter()
        .filter(|point| (*point - plane).dot(facing) <= 0.0)
        .collect();

    // four is as many as a rectangle meeting a rectangle can need, and clipping
    // can leave more when corners land on edges
    while touching.len() > 4 {
        let (drop, _) = touching
            .iter()
            .enumerate()
            .map(|(n, point)| (n, (*point - plane).dot(facing)))
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap();
        touching.remove(drop);
    }

    touching
}

/// Sutherland and Hodgman, for one plane: keeps what is on the inside of
/// `point · way <= limit`, and puts a new corner where an edge crosses out.
fn clipped(polygon: &[Vec3], way: Vec3, limit: f32) -> Vec<Vec3> {
    let mut kept = Vec::with_capacity(polygon.len() + 1);

    for n in 0..polygon.len() {
        let (here, next) = (polygon[n], polygon[(n + 1) % polygon.len()]);
        let (near, far) = (here.dot(way) - limit, next.dot(way) - limit);

        if near <= 0.0 {
            kept.push(here);
        }

        if (near > 0.0) != (far > 0.0) && (near - far).abs() > 1e-9 {
            kept.push(here + (next - here) * (near / (near - far)));
        }
    }

    kept
}

/// Where a sphere meets a box. Spec 0035.
///
/// The nearest point of the box to the middle of the sphere, which is the box's
/// own clamp done in the box's own frame. One contact point, because a sphere
/// has only ever touched anything at one.
pub fn sphere_meets_obb(ball: &Sphere, boxy: &Obb) -> Option<Meeting> {
    let into_box = boxy.turn.inverse();
    let middle = into_box * (ball.center - boxy.at);
    let near = middle.clamp(-boxy.half, boxy.half);
    let out = middle - near;
    let apart = out.length();

    if apart >= ball.radius {
        return None;
    }

    // the middle inside the box: push it out of whichever face is closest,
    // since there is no direction from a point to itself
    let (way, depth) = if apart < 1e-6 {
        let room = boxy.half - middle.abs();
        let n = (0..3)
            .min_by(|a, b| room[*a].total_cmp(&room[*b]))
            .unwrap_or(0);
        let mut way = Vec3::ZERO;
        way[n] = middle[n].signum();

        (way, room[n] + ball.radius)
    } else {
        (out / apart, ball.radius - apart)
    };

    Some(Meeting {
        // from the box towards the sphere, then turned the way the contact
        // convention wants: from the first named towards the second
        normal: -(boxy.turn * way),
        depth,
        points: vec![boxy.at + boxy.turn * near],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::vec3;

    fn cube(at: Vec3) -> Obb {
        Obb::new(at, Quat::IDENTITY, Vec3::ONE)
    }

    #[test]
    fn two_boxes_apart_do_not_meet() {
        assert!(obbs_meet(&cube(Vec3::ZERO), &cube(vec3(0.0, 2.1, 0.0))).is_none());
        assert!(obbs_meet(&cube(Vec3::ZERO), &cube(vec3(5.0, 0.0, 0.0))).is_none());
    }

    #[test]
    fn a_face_resting_on_a_face_gives_four_points() {
        let met = obbs_meet(&cube(Vec3::ZERO), &cube(vec3(0.0, 1.9, 0.0))).expect("they overlap");

        assert!((met.normal - Vec3::Y).length() < 1e-5, "{}", met.normal);
        assert!((met.depth - 0.1).abs() < 1e-5, "{}", met.depth);
        assert_eq!(met.points.len(), 4, "{:?}", met.points);

        // all four on the shared face, which is y = 1 give or take the overlap
        for point in &met.points {
            assert!(point.y > 0.8 && point.y < 1.0, "{}", point);
            assert!(point.x.abs() <= 1.0 + 1e-5 && point.z.abs() <= 1.0 + 1e-5);
        }
    }

    #[test]
    fn a_face_hanging_over_an_edge_only_touches_where_it_is_held() {
        // shifted so half of it is out past the edge of the one below
        let met = obbs_meet(&cube(Vec3::ZERO), &cube(vec3(1.0, 1.9, 0.0))).expect("they overlap");

        assert_eq!(met.points.len(), 4, "{:?}", met.points);
        for point in &met.points {
            assert!(point.x <= 1.0 + 1e-5, "contact out in mid air at {}", point);
        }
    }

    #[test]
    fn fewer_points_for_an_edge_or_a_corner() {
        // turned a half turn about z so it comes down on one of its long edges
        let on_edge = Obb::new(
            vec3(0.0, 2.3, 0.0),
            Quat::from_rotation_z(std::f32::consts::FRAC_PI_4),
            Vec3::ONE,
        );
        let met = obbs_meet(&cube(Vec3::ZERO), &on_edge).expect("they overlap");
        assert!(
            met.points.len() <= 2,
            "an edge gave {} points: {:?}",
            met.points.len(),
            met.points
        );

        // turned about two axes so it comes down on a corner
        let on_corner = Obb::new(
            vec3(0.0, 2.5, 0.0),
            Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)
                * Quat::from_rotation_x(std::f32::consts::FRAC_PI_4),
            Vec3::ONE,
        );
        let met = obbs_meet(&cube(Vec3::ZERO), &on_corner).expect("they overlap");
        assert!(
            met.points.len() <= 2,
            "a corner gave {} points: {:?}",
            met.points.len(),
            met.points
        );
    }

    #[test]
    fn an_edge_crossing_an_edge_is_found() {
        // one long bar along x, another along z, crossed and overlapping a
        // little. Nothing separates them on any face axis, so only a cross
        // product can find it.
        let along_x = Obb::new(Vec3::ZERO, Quat::IDENTITY, vec3(4.0, 0.5, 0.5));
        let along_z = Obb::new(
            vec3(0.0, 0.9, 0.0),
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            vec3(4.0, 0.5, 0.5),
        );

        let met = obbs_meet(&along_x, &along_z).expect("they overlap");
        assert!((met.normal - Vec3::Y).length() < 1e-4, "{}", met.normal);
        assert!((met.depth - 0.1).abs() < 1e-4, "{}", met.depth);
    }

    #[test]
    fn a_turned_box_is_separated_where_a_square_one_would_not_be() {
        // a cube turned 45 degrees about y reaches sqrt(2) along x, not 1, and
        // a test that forgot its axes would say these two overlap
        let turned = Obb::new(
            vec3(2.2, 0.0, 0.0),
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_4),
            Vec3::ONE,
        );

        assert!(
            obbs_meet(&cube(Vec3::ZERO), &turned).is_some(),
            "1 + 1.414 > 2.2"
        );
        assert!(obbs_meet(&cube(Vec3::ZERO), &cube(vec3(2.2, 0.0, 0.0))).is_none());
    }

    #[test]
    fn a_wall_is_a_box_with_no_turn() {
        let wall = Aabb::from_center_size(vec3(0.0, -1.0, 0.0), vec3(20.0, 2.0, 20.0));
        let read = Obb::from_aabb(&wall);

        assert_eq!(read.at, vec3(0.0, -1.0, 0.0));
        assert_eq!(read.half, vec3(10.0, 1.0, 10.0));
        assert_eq!(read.turn, Quat::IDENTITY);
    }

    #[test]
    fn a_sphere_meets_a_box_on_its_nearest_point() {
        let ball = Sphere::new(vec3(0.0, 1.4, 0.0), 0.5);
        let met = sphere_meets_obb(&ball, &cube(Vec3::ZERO)).expect("they overlap");

        // from the box towards the sphere is up, and the convention here is
        // from the first named towards the second, so it points down
        assert!((met.normal + Vec3::Y).length() < 1e-5, "{}", met.normal);
        assert!((met.depth - 0.1).abs() < 1e-5, "{}", met.depth);
        assert_eq!(met.points.len(), 1);
        assert!((met.points[0] - vec3(0.0, 1.0, 0.0)).length() < 1e-5);

        assert!(
            sphere_meets_obb(&Sphere::new(vec3(0.0, 1.6, 0.0), 0.5), &cube(Vec3::ZERO)).is_none()
        );
    }

    #[test]
    fn a_sphere_inside_a_box_comes_out_the_near_side() {
        // middle of the sphere inside the box, closer to +x than anything else
        let ball = Sphere::new(vec3(0.8, 0.0, 0.0), 0.5);
        let met = sphere_meets_obb(&ball, &cube(Vec3::ZERO)).expect("it is inside");

        assert!((met.normal + Vec3::X).length() < 1e-5, "{}", met.normal);
        assert!(met.depth > 0.0);
    }

    fn unit_box() -> Aabb {
        Aabb::from_center_size(Vec3::ZERO, Vec3::splat(2.0))
    }

    #[test]
    fn boxes_overlap_or_miss() {
        let box_ = unit_box();

        assert!(box_.intersects(&Aabb::from_center_size(vec3(1.5, 0.0, 0.0), Vec3::ONE)));
        assert!(!box_.intersects(&Aabb::from_center_size(vec3(3.0, 0.0, 0.0), Vec3::ONE)));
        // touching exactly counts, so a ball resting on a floor is in contact
        assert!(box_.intersects(&Aabb::from_center_size(
            vec3(2.0, 0.0, 0.0),
            Vec3::splat(2.0)
        )));
    }

    #[test]
    fn spheres_overlap_or_miss() {
        let sphere = Sphere::new(Vec3::ZERO, 1.0);

        assert!(sphere.intersects(&Sphere::new(vec3(1.5, 0.0, 0.0), 1.0)));
        assert!(!sphere.intersects(&Sphere::new(vec3(2.5, 0.0, 0.0), 1.0)));
        assert!(sphere.intersects(&Sphere::new(vec3(2.0, 0.0, 0.0), 1.0)));
    }

    #[test]
    fn a_sphere_meets_a_box() {
        let box_ = unit_box();

        // through a face
        assert!(Sphere::new(vec3(1.5, 0.0, 0.0), 0.6).intersects_aabb(&box_));
        assert!(!Sphere::new(vec3(1.5, 0.0, 0.0), 0.4).intersects_aabb(&box_));

        // at an edge, where the nearest point is on two faces at once
        assert!(Sphere::new(vec3(1.3, 1.3, 0.0), 0.5).intersects_aabb(&box_));
        assert!(!Sphere::new(vec3(1.6, 1.6, 0.0), 0.5).intersects_aabb(&box_));

        // at a corner, the hardest case for a naive test
        assert!(Sphere::new(vec3(1.2, 1.2, 1.2), 0.5).intersects_aabb(&box_));
        assert!(!Sphere::new(vec3(1.5, 1.5, 1.5), 0.5).intersects_aabb(&box_));
    }

    #[test]
    fn points_are_inside_or_outside() {
        let box_ = unit_box();
        assert!(box_.contains_point(Vec3::ZERO));
        assert!(box_.contains_point(vec3(1.0, 1.0, 1.0)));
        assert!(!box_.contains_point(vec3(1.1, 0.0, 0.0)));

        let sphere = Sphere::new(Vec3::ZERO, 1.0);
        assert!(sphere.contains_point(vec3(0.5, 0.5, 0.0)));
        assert!(!sphere.contains_point(vec3(0.9, 0.9, 0.0)));
    }

    #[test]
    fn a_ray_hits_a_box() {
        let hit = Ray::new(vec3(-5.0, 0.0, 0.0), Vec3::X)
            .hit_aabb(&unit_box())
            .expect("the ray points straight at it");

        assert!(
            (hit.distance - 4.0).abs() < 1e-5,
            "distance {}",
            hit.distance
        );
        assert!((hit.point - vec3(-1.0, 0.0, 0.0)).length() < 1e-5);
        // the face it struck looks back along the ray
        assert!((hit.normal - vec3(-1.0, 0.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn a_ray_pointing_away_misses() {
        let ray = Ray::new(vec3(-5.0, 0.0, 0.0), -Vec3::X);

        assert!(ray.hit_aabb(&unit_box()).is_none());
        assert!(ray.hit_sphere(&Sphere::new(Vec3::ZERO, 1.0)).is_none());
    }

    #[test]
    fn a_ray_hits_a_sphere() {
        let hit = Ray::new(vec3(0.0, 0.0, 5.0), -Vec3::Z)
            .hit_sphere(&Sphere::new(Vec3::ZERO, 2.0))
            .expect("the ray points straight at it");

        // the near surface, not the far one
        assert!(
            (hit.distance - 3.0).abs() < 1e-5,
            "distance {}",
            hit.distance
        );
        assert!((hit.normal - Vec3::Z).length() < 1e-5);
    }

    #[test]
    fn a_swept_sphere_does_not_tunnel() {
        let wall = Aabb::from_center_size(vec3(5.0, 0.0, 0.0), vec3(0.2, 10.0, 10.0));
        let ball = Sphere::new(Vec3::ZERO, 0.5);

        // a step that would jump clean past a thin wall
        let hit = sweep_sphere(&ball, vec3(20.0, 0.0, 0.0), &wall)
            .expect("the sweep catches what a snapshot would miss");

        assert!(hit.distance < 5.0, "stopped at {}", hit.distance);
        // and a test of where it ended up would have seen nothing
        assert!(!Sphere::new(vec3(20.0, 0.0, 0.0), 0.5).intersects_aabb(&wall));
    }

    #[test]
    fn resting_on_a_floor_rolls_but_does_not_sink() {
        let floor = Aabb::from_center_size(vec3(0.0, -0.5, 0.0), vec3(20.0, 1.0, 20.0));
        // exactly touching, which is where a ball at rest ends up
        let ball = Sphere::new(vec3(0.0, 0.5, 0.0), 0.5);

        // along the floor is free
        assert!(sweep_sphere(&ball, vec3(1.0, 0.0, 0.0), &floor).is_none());
        // up and away is free
        assert!(sweep_sphere(&ball, vec3(0.0, 1.0, 0.0), &floor).is_none());
        // down into it is not
        let hit = sweep_sphere(&ball, vec3(0.0, -1.0, 0.0), &floor)
            .expect("gravity should still be stopped by the floor");
        assert_eq!(hit.distance, 0.0);
        assert!((hit.normal - Vec3::Y).length() < 1e-5, "{:?}", hit.normal);

        // and rolling along it keeps its whole step
        let end = move_and_slide(ball, vec3(4.0, 0.0, 0.0), 1.0, &[floor]);
        assert!((end.x - 4.0).abs() < 1e-3, "rolled to {:?}", end);
    }

    #[test]
    fn a_ball_rolls_off_the_end_of_a_floor() {
        // the floor runs to x = 10, and the ball is just past its edge: clear of
        // the box, but still inside the box grown by its radius
        let floor = Aabb::from_center_size(vec3(0.0, -0.5, 0.0), vec3(20.0, 1.0, 20.0));
        let ball = Sphere::new(vec3(10.14, 0.5, 0.0), 0.5);

        assert!(!ball.intersects_aabb(&floor), "the ball is past the edge");

        // carrying on outward is free, not blocked by the floor behind it
        assert!(sweep_sphere(&ball, vec3(1.0, 0.0, 0.0), &floor).is_none());
        let end = move_and_slide(ball, vec3(4.0, 0.0, 0.0), 1.0, &[floor]);
        assert!(end.x > 13.0, "it stopped at the edge: {:?}", end);
    }

    #[test]
    fn a_ball_rolls_from_one_platform_onto_the_next() {
        // two platforms whose tops are level, meeting at z = 0
        let first = Aabb::from_center_size(vec3(0.0, -0.5, 4.0), vec3(10.0, 1.0, 10.0));
        let second = Aabb::from_center_size(vec3(0.0, -0.5, -6.0), vec3(5.0, 1.0, 12.0));
        let ball = Sphere::new(vec3(0.0, 0.5, 0.5), 0.5);

        // rolling toward the seam is not rolling into a wall
        assert!(sweep_sphere(&ball, vec3(0.0, 0.0, -2.0), &second).is_none());

        let end = move_and_slide(ball, vec3(0.0, 0.0, -6.0), 1.0, &[first, second]);
        assert!(end.z < -4.0, "it stopped at the seam: {:?}", end);
    }

    #[test]
    fn a_ball_level_with_a_platform_top_is_not_blocked_by_its_side() {
        // the exact case the marble game hit: rolling along one platform toward
        // another whose top is at the same height. The ball's center is 0.57
        // from the platform, well clear of its 0.4 radius, so nothing should
        // stop it, though a square cornered sweep says otherwise.
        let next = Aabb::from_center_size(vec3(0.0, -0.5, -6.0), vec3(5.0, 1.0, 12.0));
        let ball = Sphere::new(vec3(0.0, 0.4, 0.6), 0.4);

        assert!(
            sweep_sphere(&ball, vec3(0.0, 0.0, -0.5), &next).is_none(),
            "stopped short of a platform it is level with"
        );

        let end = move_and_slide(ball, vec3(0.0, 0.0, -6.0), 1.0, &[next]);
        assert!(end.z < -4.0, "it stopped at the seam: {:?}", end);
    }

    #[test]
    fn a_ball_still_lands_on_a_platform_from_above() {
        let platform = Aabb::from_center_size(vec3(0.0, -0.5, 0.0), vec3(10.0, 1.0, 10.0));
        let ball = Sphere::new(vec3(0.0, 4.0, 0.0), 0.4);

        let hit = sweep_sphere(&ball, vec3(0.0, -8.0, 0.0), &platform)
            .expect("falling onto a platform still stops");

        assert!(
            (hit.distance - 3.6).abs() < 1e-3,
            "distance {}",
            hit.distance
        );
        assert!((hit.normal - Vec3::Y).length() < 1e-3, "{:?}", hit.normal);
    }

    #[test]
    fn a_ball_is_still_stopped_by_a_wall_it_faces() {
        let wall = Aabb::from_center_size(vec3(3.0, 1.0, 0.0), vec3(1.0, 2.0, 10.0));
        let ball = Sphere::new(vec3(0.0, 0.5, 0.0), 0.4);

        let hit = sweep_sphere(&ball, vec3(6.0, 0.0, 0.0), &wall).expect("a wall stops it");

        // the wall's near face is at x 2.5, so contact is 0.4 short of it
        assert!(
            (hit.distance - 2.1).abs() < 1e-3,
            "distance {}",
            hit.distance
        );
        assert!(
            (hit.normal - vec3(-1.0, 0.0, 0.0)).length() < 1e-3,
            "{:?}",
            hit.normal
        );
    }

    #[test]
    fn a_corner_is_rounded_rather_than_square() {
        let box_ = Aabb::from_center_size(Vec3::ZERO, Vec3::splat(2.0));
        // heading at the corner diagonally from outside
        let ball = Sphere::new(vec3(3.0, 3.0, 0.0), 0.5);

        let hit = sweep_sphere(&ball, vec3(-4.0, -4.0, 0.0), &box_).expect("it meets the corner");

        // a square corner would stop it at the grown box, 0.5 further out than
        // the rounded one, which sits 0.5 from the corner along the diagonal
        let center = vec3(3.0, 3.0, 0.0) + vec3(-4.0, -4.0, 0.0).normalize() * hit.distance;
        let corner = vec3(1.0, 1.0, 0.0);
        assert!(
            ((center - corner).length() - 0.5).abs() < 1e-3,
            "contact was {} from the corner",
            (center - corner).length()
        );
    }

    #[test]
    fn move_and_slide_slides() {
        let wall = Aabb::from_center_size(vec3(2.0, 0.0, 0.0), vec3(1.0, 10.0, 10.0));
        let ball = Sphere::new(Vec3::ZERO, 0.5);

        // driven diagonally into a wall that blocks x but not z
        let end = move_and_slide(ball, vec3(4.0, 0.0, 4.0), 1.0, &[wall]);

        assert!(
            end.x < 1.1,
            "should be stopped by the wall, x was {}",
            end.x
        );
        assert!(end.z > 1.0, "should have slid along it, z was {}", end.z);
    }

    #[test]
    fn move_and_slide_settles_in_a_corner() {
        let walls = [
            Aabb::from_center_size(vec3(2.0, 0.0, 0.0), vec3(1.0, 10.0, 10.0)),
            Aabb::from_center_size(vec3(0.0, 0.0, 2.0), vec3(10.0, 10.0, 1.0)),
        ];
        let ball = Sphere::new(Vec3::ZERO, 0.5);

        let end = move_and_slide(ball, vec3(4.0, 0.0, 4.0), 1.0, &walls);

        // wedged, not shot through either wall and not flung away
        assert!(end.x < 1.1 && end.z < 1.1, "ended at {:?}", end);
        assert!(end.x > -0.1 && end.z > -0.1, "ended at {:?}", end);
        for wall in walls.iter() {
            assert!(
                !Sphere::new(end, 0.5).intersects_aabb(wall),
                "inside a wall at {:?}",
                end
            );
        }
    }

    #[test]
    fn move_and_slide_leaves_open_space_alone() {
        let far_away = Aabb::from_center_size(vec3(50.0, 0.0, 0.0), Vec3::ONE);
        let ball = Sphere::new(Vec3::ZERO, 0.5);

        let end = move_and_slide(ball, vec3(1.0, 2.0, 3.0), 0.5, &[far_away]);

        assert!(
            (end - vec3(0.5, 1.0, 1.5)).length() < 1e-5,
            "ended at {:?}",
            end
        );
    }
}
