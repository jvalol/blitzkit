//! The sun, the lamps, the spots, and the shading they produce.
//!
//! One sun with a direction and no place, up to eight lamps with a place and a
//! reach, and up to four spots, which are lamps with a direction. See
//! `specs/0012-lighting.md`, `0020-point-lights.md` and `0021-spot-lights.md`.
//!
//! The maths here also runs in `mesh.wgsl`, on the GPU. This copy exists so it
//! can be checked without one; the two are kept in step by hand, which is the
//! gap the spec's hand checks cover.

use glam::Vec3;

/// A light with no position, only a direction, like the sun.
#[derive(Debug, Copy, Clone)]
pub struct Light {
    /// The direction the light travels, so a light overhead points down.
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    /// Fills the side facing away, so it is not pure black.
    pub ambient: Vec3,
}

impl Light {
    pub fn new() -> Self {
        Self {
            // from above and a little to one side, so a cube's faces differ
            direction: Vec3::new(-0.3, -1.0, -0.4).normalize(),
            color: Vec3::ONE,
            intensity: 1.0,
            ambient: Vec3::splat(0.15),
        }
    }

    /// Blinn-Phong: ambient, plus diffuse from the angle to the light, plus a
    /// highlight from the half vector between the light and the viewer.
    ///
    /// `normal` and `to_viewer` are unit vectors in world space, `base` is the
    /// surface color, and `shininess` is how tight the highlight is.
    pub fn shade(&self, normal: Vec3, to_viewer: Vec3, base: Vec3, shininess: f32) -> Vec3 {
        let to_light = -self.direction.normalize_or_zero();
        let normal = normal.normalize_or_zero();
        let lambert = normal.dot(to_light).max(0.0);

        let light = self.color * self.intensity;
        let diffuse = light * lambert;

        let specular = if lambert > 0.0 {
            let half = (to_light + to_viewer.normalize_or_zero()).normalize_or_zero();
            light * normal.dot(half).max(0.0).powf(shininess.max(1.0))
        } else {
            Vec3::ZERO
        };

        base * (self.ambient + diffuse) + specular
    }
}

impl Default for Light {
    fn default() -> Self {
        Self::new()
    }
}

/// How many lamps fit in the block the shader reads. See spec 0020.
///
/// It was eight, which was never a number about hardware. A lamp is two vec4s,
/// so eight of them is 256 bytes of a block that a uniform binding has
/// kilobytes of room for, and the number came from the first scene that wanted
/// more than one light rather than from anything measured.
///
/// Sixty four, because a building is not a scene. The arcade alone has thirteen
/// sconces, thirteen lit marquees and a pendant over a sign, which is twenty
/// seven before anybody opens another room, and it was spending six of its eight
/// on one room's walls whether or not you were standing in it.
///
/// What a lamp costs is the loop in the shader, not the block it sits in, and
/// that loop now skips a lamp whose range does not reach the surface before it
/// does any work. The falloff is zero at the range rather than merely small, so
/// a lamp in another room contributes exactly nothing and is charged exactly
/// nothing for it.
pub const MAX_POINT_LIGHTS: usize = 64;

/// How many lamps may cast a shadow, out of the [`MAX_POINT_LIGHTS`] that may
/// light. Each one is six passes over the scene, which is why it is two and not
/// eight. See spec 0022.
pub const MAX_SHADOWING_POINT_LIGHTS: usize = 2;

/// Which lamps out of a list cast, by their place in it, capped at
/// [`MAX_SHADOWING_POINT_LIGHTS`].
///
/// One rule in one place, because three things ask: the scene, the uniform the
/// shader reads, and the passes that fill the maps. Any two of them disagreeing
/// would light one lamp and shadow another.
pub fn casting_lamps(lamps: &[PointLight]) -> Vec<usize> {
    lamps
        .iter()
        .enumerate()
        .filter(|(_, light)| light.casts)
        .map(|(index, _)| index)
        .take(MAX_SHADOWING_POINT_LIGHTS)
        .collect()
}

/// A light with a place, and a distance past which it stops.
///
/// By default it casts no shadow, because a lamp shines every way at once and
/// covering that takes six passes over the scene. A lamp that wants one asks
/// with [`PointLight::casting`], and at most [`MAX_SHADOWING_POINT_LIGHTS`] of
/// them are granted.
#[derive(Debug, Copy, Clone)]
pub struct PointLight {
    pub position: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    /// How far it reaches. Past this it contributes nothing at all.
    pub range: f32,
    /// Whether it asks for a shadow. Nothing about the light it gives depends
    /// on this; see spec 0022.
    pub casts: bool,
}

impl PointLight {
    pub fn new(position: Vec3, color: Vec3, intensity: f32, range: f32) -> Self {
        Self {
            position,
            color,
            intensity,
            range,
            casts: false,
        }
    }

    /// The same lamp, asking for a shadow.
    pub fn casting(mut self) -> Self {
        self.casts = true;
        self
    }

    /// How much of the light is left at `distance`: one where it sits, nothing
    /// at its range, and smooth between.
    ///
    /// Not inverse square. That never reaches zero, so every light would have
    /// to be weighed against every surface, and it goes to infinity where the
    /// light is, so anything touching a lamp turns white. The square here is
    /// what keeps the middle from reading as a flat disc. See spec 0020.
    pub fn falloff(&self, distance: f32) -> f32 {
        if self.range <= 0.0 {
            return 0.0;
        }

        let left = 1.0 - (distance / self.range).clamp(0.0, 1.0);

        left * left
    }

    /// What this light adds at a point on a surface. Diffuse and highlight, the
    /// same Blinn-Phong the sun uses, faded by distance. No ambient: the sun
    /// owns that, and eight lamps each adding their own would wash the scene
    /// out.
    ///
    /// `base` tints the diffuse and not the highlight, the same way
    /// [`Light::shade`] does it, because a highlight is the light reflected
    /// rather than the surface lit.
    ///
    /// Matches the loop in `mesh.wgsl`. Keep the two in step.
    pub fn shade(
        &self,
        at: Vec3,
        normal: Vec3,
        to_viewer: Vec3,
        base: Vec3,
        shininess: f32,
    ) -> Vec3 {
        let offset = self.position - at;
        let distance = offset.length();
        let faded = self.falloff(distance);
        if faded <= 0.0 {
            return Vec3::ZERO;
        }

        let to_light = offset.normalize_or_zero();
        let normal = normal.normalize_or_zero();
        let lambert = normal.dot(to_light).max(0.0);
        if lambert <= 0.0 {
            return Vec3::ZERO;
        }

        let light = self.color * self.intensity * faded;
        let half = (to_light + to_viewer.normalize_or_zero()).normalize_or_zero();
        let specular = light * normal.dot(half).max(0.0).powf(shininess.max(1.0));

        base * light * lambert + specular
    }
}

/// How many spots fit. Far fewer than lamps, because each one costs a whole
/// pass over the scene to fill its shadow map. See spec 0021.
pub const MAX_SPOT_LIGHTS: usize = 4;

/// A lamp with a direction: a cone, which is the cheapest light that can be
/// moved and still cast a shadow. One direction is one projection is one depth
/// map, which is the machinery spec 0015 already built for the sun.
#[derive(Debug, Copy, Clone)]
pub struct SpotLight {
    pub position: Vec3,
    /// The way the cone points. Need not be normalized.
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub range: f32,
    /// Filled to the brim inside this angle from the middle, in radians.
    pub inner: f32,
    /// Nothing at all outside this one.
    pub outer: f32,
}

impl SpotLight {
    pub fn new(
        position: Vec3,
        direction: Vec3,
        color: Vec3,
        intensity: f32,
        range: f32,
        inner: f32,
        outer: f32,
    ) -> Self {
        Self {
            position,
            direction,
            color,
            intensity,
            range,
            inner,
            outer,
        }
    }

    /// The cosines the shader compares against, outer first. Kept together
    /// because the order matters and getting it backwards turns the cone inside
    /// out: a wider angle has the smaller cosine.
    ///
    /// An inner angle wider than the outer is not an error, it is just a spot
    /// with no soft edge, so the two are put back in order rather than refused.
    pub fn cone_cosines(&self) -> (f32, f32) {
        let outer = self.outer.cos();
        let inner = self.inner.cos();

        if inner < outer {
            (inner, outer)
        } else {
            (outer, inner)
        }
    }

    /// How much of the cone reaches a point, from its angle alone. One down the
    /// middle, nothing past the outer angle, and smooth between: a hard circle
    /// of light on a floor is what gives a cheap spot away.
    ///
    /// Matches `spot_cone` in `mesh.wgsl`. Keep the two in step.
    pub fn cone(&self, toward_surface: Vec3) -> f32 {
        let facing = self.direction.normalize_or_zero();
        let toward = toward_surface.normalize_or_zero();
        if facing == Vec3::ZERO || toward == Vec3::ZERO {
            return 0.0;
        }

        let (cos_outer, cos_inner) = self.cone_cosines();
        let cosine = facing.dot(toward);
        let width = cos_inner - cos_outer;
        if width <= 0.0 {
            // no soft edge to speak of, so it is in or it is out
            return if cosine >= cos_inner { 1.0 } else { 0.0 };
        }

        let along = ((cosine - cos_outer) / width).clamp(0.0, 1.0);

        // smooth at both ends rather than a straight ramp, so the edge does not
        // show a line where the falloff starts
        along * along * (3.0 - 2.0 * along)
    }

    /// The same distance falloff a lamp uses, per spec 0020. A spot is a lamp
    /// with a direction, not a different kind of thing.
    pub fn falloff(&self, distance: f32) -> f32 {
        PointLight::new(self.position, self.color, self.intensity, self.range).falloff(distance)
    }

    /// What this spot adds at a point on a surface.
    ///
    /// Matches the loop in `mesh.wgsl`. Keep the two in step.
    pub fn shade(
        &self,
        at: Vec3,
        normal: Vec3,
        to_viewer: Vec3,
        base: Vec3,
        shininess: f32,
    ) -> Vec3 {
        let offset = self.position - at;
        let distance = offset.length();
        let faded = self.falloff(distance) * self.cone(-offset);
        if faded <= 0.0 {
            return Vec3::ZERO;
        }

        let to_light = offset.normalize_or_zero();
        let normal = normal.normalize_or_zero();
        let lambert = normal.dot(to_light).max(0.0);
        if lambert <= 0.0 {
            return Vec3::ZERO;
        }

        let light = self.color * self.intensity * faded;
        let half = (to_light + to_viewer.normalize_or_zero()).normalize_or_zero();
        let specular = light * normal.dot(half).max(0.0).powf(shininess.max(1.0));

        base * light * lambert + specular
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A spot two units up, pointing straight down, a 20 degree core inside a
    /// 35 degree cone.
    fn spot() -> SpotLight {
        SpotLight::new(
            Vec3::Y * 2.0,
            -Vec3::Y,
            Vec3::ONE,
            1.0,
            6.0,
            20f32.to_radians(),
            35f32.to_radians(),
        )
    }

    #[test]
    fn a_spot_is_full_strength_down_its_middle() {
        // straight down the axis, and anywhere inside the inner angle
        assert!((spot().cone(-Vec3::Y) - 1.0).abs() < 1e-6);

        let just_inside = glam::vec3(15f32.to_radians().tan(), -1.0, 0.0);
        assert!((spot().cone(just_inside) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn a_spot_stops_at_its_outer_angle() {
        let past = glam::vec3(40f32.to_radians().tan(), -1.0, 0.0);
        assert_eq!(spot().cone(past), 0.0);

        // and straight out the back is nothing, not a second cone
        assert_eq!(spot().cone(Vec3::Y), 0.0);
    }

    #[test]
    fn a_spot_edge_is_soft() {
        let spot = spot();
        let at = |degrees: f32| spot.cone(glam::vec3(degrees.to_radians().tan(), -1.0, 0.0));

        let mut last = 1.0;
        for step in 0..=30 {
            let degrees = 20.0 + step as f32 * 0.5;
            let now = at(degrees);

            assert!(
                now <= last + 1e-6,
                "it brightened at {:.1} degrees",
                degrees
            );
            assert!((0.0..=1.0).contains(&now));
            last = now;
        }

        // partway across the soft edge it is neither on nor off, which is the
        // whole point of having one
        let middle = at(27.5);
        assert!(
            (0.05..0.95).contains(&middle),
            "the edge is a step, at {}",
            middle
        );
    }

    #[test]
    fn a_spot_with_its_angles_backwards_still_behaves() {
        let mut backwards = spot();
        std::mem::swap(&mut backwards.inner, &mut backwards.outer);

        // the wider of the two is still the outer one, so the cone does not
        // turn inside out
        assert!((backwards.cone(-Vec3::Y) - 1.0).abs() < 1e-6);
        assert_eq!(
            backwards.cone(glam::vec3(40f32.to_radians().tan(), -1.0, 0.0)),
            0.0
        );

        let (cos_outer, cos_inner) = backwards.cone_cosines();
        assert!(
            cos_outer <= cos_inner,
            "a wider angle has the smaller cosine"
        );
    }

    #[test]
    fn a_spot_fades_with_distance_like_a_lamp() {
        let spot = spot();
        let lamp = PointLight::new(spot.position, spot.color, spot.intensity, spot.range);

        for step in 0..=10 {
            let distance = spot.range * step as f32 / 10.0;
            assert!((spot.falloff(distance) - lamp.falloff(distance)).abs() < 1e-6);
        }
    }

    #[test]
    fn a_spot_does_not_light_the_back_of_a_surface() {
        let away = spot().shade(Vec3::ZERO, -Vec3::Y, -Vec3::Y, WHITE, 32.0);
        assert_eq!(away, Vec3::ZERO);

        let toward = spot().shade(Vec3::ZERO, Vec3::Y, Vec3::Y, WHITE, 32.0);
        assert!(toward.length() > 0.0, "the floor under it was dark");
    }

    #[test]
    fn a_spot_carries_its_color() {
        let mut green = spot();
        green.color = Vec3::new(0.0, 1.0, 0.0);
        let lit = green.shade(Vec3::ZERO, Vec3::Y, Vec3::Y, WHITE, 32.0);

        assert!(lit.y > 0.0);
        assert!(
            lit.x.abs() < 1e-6 && lit.z.abs() < 1e-6,
            "green turned up {:?}",
            lit
        );
    }

    #[test]
    fn a_spot_aimed_nowhere_lights_nothing() {
        let mut broken = spot();
        broken.direction = Vec3::ZERO;

        assert_eq!(broken.cone(-Vec3::Y), 0.0);
        assert_eq!(
            broken.shade(Vec3::ZERO, Vec3::Y, Vec3::Y, WHITE, 32.0),
            Vec3::ZERO
        );
    }

    const WHITE: Vec3 = Vec3::ONE;

    /// A lamp one unit above the origin, reaching four.
    fn lamp() -> PointLight {
        PointLight::new(Vec3::Y, Vec3::ONE, 1.0, 4.0)
    }

    #[test]
    fn a_point_light_is_full_strength_at_its_own_position() {
        assert!((lamp().falloff(0.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_point_light_stops_at_its_range() {
        let lamp = lamp();

        assert!(lamp.falloff(lamp.range).abs() < 1e-6);
        assert_eq!(lamp.falloff(lamp.range + 1.0), 0.0);
        assert_eq!(lamp.falloff(1_000.0), 0.0);
    }

    #[test]
    fn a_point_light_fades_the_whole_way() {
        let lamp = lamp();
        let mut last = f32::INFINITY;

        for step in 0..=40 {
            let distance = lamp.range * step as f32 / 40.0;
            let now = lamp.falloff(distance);

            assert!(now <= last + 1e-6, "it brightened at {:.2}", distance);
            assert!((0.0..=1.0).contains(&now));
            last = now;
        }
        // and it is a curve rather than a straight line, which is what stops
        // the lit patch reading as a flat disc
        assert!(lamp.falloff(lamp.range * 0.5) < 0.5 - 0.05);
    }

    #[test]
    fn a_light_with_no_range_lights_nothing() {
        let dead = PointLight::new(Vec3::ZERO, Vec3::ONE, 1.0, 0.0);

        assert_eq!(dead.falloff(0.0), 0.0);
        assert_eq!(
            dead.shade(Vec3::ZERO, Vec3::Y, Vec3::Y, WHITE, 32.0),
            Vec3::ZERO
        );
    }

    #[test]
    fn a_point_light_does_not_light_the_back_of_a_surface() {
        // a floor at the origin facing down, with the lamp above it
        let away = lamp().shade(Vec3::ZERO, -Vec3::Y, -Vec3::Y, WHITE, 32.0);

        assert_eq!(away, Vec3::ZERO);
        // and the same surface turned over is lit
        let toward = lamp().shade(Vec3::ZERO, Vec3::Y, Vec3::Y, WHITE, 32.0);
        assert!(toward.length() > 0.0);
    }

    #[test]
    fn a_point_light_carries_its_color() {
        let red = PointLight::new(Vec3::Y, Vec3::new(1.0, 0.0, 0.0), 1.0, 4.0);
        let lit = red.shade(Vec3::ZERO, Vec3::Y, Vec3::Y, WHITE, 32.0);

        assert!(lit.x > 0.0);
        assert!(lit.y.abs() < 1e-6, "red light turned up green");
        assert!(lit.z.abs() < 1e-6, "red light turned up blue");

        // and twice the intensity is brighter
        let brighter = PointLight::new(Vec3::Y, Vec3::new(1.0, 0.0, 0.0), 2.0, 4.0);
        assert!(brighter.shade(Vec3::ZERO, Vec3::Y, Vec3::Y, WHITE, 32.0).x > lit.x);
    }

    #[test]
    fn point_lights_add_together() {
        let red = PointLight::new(Vec3::Y, Vec3::new(1.0, 0.0, 0.0), 1.0, 4.0);
        let blue = PointLight::new(Vec3::Y, Vec3::new(0.0, 0.0, 1.0), 1.0, 4.0);

        let both = red.shade(Vec3::ZERO, Vec3::Y, Vec3::Y, WHITE, 32.0)
            + blue.shade(Vec3::ZERO, Vec3::Y, Vec3::Y, WHITE, 32.0);

        assert!(both.x > 0.0 && both.z > 0.0, "one of them won outright");
    }

    #[test]
    fn no_point_lights_is_the_old_shading() {
        // nothing added is nothing changed, which is what keeps every game that
        // never asks for a lamp looking the way it looked
        let sun = Light::new();
        let none: &[PointLight] = &[];

        let from_sun = sun.shade(Vec3::Y, Vec3::Y, WHITE, 32.0);
        let from_lamps = none.iter().fold(Vec3::ZERO, |total, lamp| {
            total + lamp.shade(Vec3::ZERO, Vec3::Y, Vec3::Y, WHITE, 32.0)
        });

        assert_eq!(from_lamps, Vec3::ZERO);
        assert_eq!(from_sun + from_lamps, from_sun);
    }

    fn white() -> Vec3 {
        Vec3::ONE
    }

    #[test]
    fn facing_the_light_is_brighter() {
        let light = Light::new();
        let to_light = -light.direction;
        let away = light.direction;

        let lit = light.shade(to_light, to_light, white(), 32.0);
        let dark = light.shade(away, to_light, white(), 32.0);

        assert!(lit.x > dark.x, "lit {:?} dark {:?}", lit, dark);
    }

    #[test]
    fn the_dark_side_is_ambient() {
        let light = Light::new();
        // facing directly away, so no diffuse and no highlight
        let shaded = light.shade(light.direction, -light.direction, white(), 32.0);

        assert!((shaded - light.ambient).length() < 1e-5, "{:?}", shaded);
    }

    #[test]
    fn intensity_scales_the_light() {
        let mut light = Light::new();
        let normal = -light.direction;

        let dim = light.shade(normal, normal, white(), 32.0);
        light.intensity = 2.0;
        let bright = light.shade(normal, normal, white(), 32.0);

        assert!(bright.x > dim.x);
        // ambient is not scaled by intensity, only the light itself
        light.intensity = 0.0;
        let unlit = light.shade(normal, normal, white(), 32.0);
        assert!((unlit - light.ambient).length() < 1e-5, "{:?}", unlit);
    }

    #[test]
    fn the_default_light_is_overhead() {
        let light = Light::new();

        // it travels downward, which means it comes from above
        assert!(light.direction.y < 0.0);
        assert!((light.direction.length() - 1.0).abs() < 1e-5);
        assert_eq!(light.color, Vec3::ONE);
    }

    #[test]
    fn a_tighter_highlight_is_smaller() {
        let light = Light::new();
        let normal = -light.direction;

        let broad = light.shade(normal, Vec3::Y, white(), 4.0);
        let tight = light.shade(normal, Vec3::Y, white(), 128.0);

        // same diffuse, so any difference is the highlight
        assert!(broad.x >= tight.x);
    }

    #[test]
    fn a_lamp_does_not_cast_unless_it_asks() {
        // five games and every example built before spec 0022 make lamps this
        // way, and none of them should start paying for six passes
        let plain = PointLight::new(Vec3::ZERO, Vec3::ONE, 1.0, 5.0);
        assert!(!plain.casts);

        assert!(plain.casting().casts);
    }

    #[test]
    fn casting_does_not_change_what_a_lamp_lights() {
        let plain = PointLight::new(Vec3::Y * 3.0, Vec3::new(1.0, 0.4, 0.2), 1.4, 9.0);
        let asking = plain.casting();

        let at = Vec3::new(1.0, 0.0, 0.5);
        let normal = Vec3::Y;
        let to_viewer = Vec3::new(0.0, 1.0, 1.0).normalize();

        assert_eq!(
            plain.shade(at, normal, to_viewer, Vec3::ONE, 32.0),
            asking.shade(at, normal, to_viewer, Vec3::ONE, 32.0),
            "asking for a shadow changed the light itself"
        );
        assert_eq!(plain.falloff(4.0), asking.falloff(4.0));
    }
}
