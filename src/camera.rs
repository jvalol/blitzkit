//! Where the scene is looked at from, and the matrices that follow.
//!
//! Right-handed with y up, and the projection is where that becomes the
//! screen's coordinates. See `specs/0008-camera.md`.

use crate::collision::Ray;
use glam::{Mat4, Vec2, Vec3, Vec4};

/// Where the scene is looked at from.
///
/// World space is right-handed with y up, per `specs/0007-math-types.md`, and
/// the projection targets wgpu's clip space: depth 0 at the near plane, 1 at the
/// far plane.
#[derive(Debug, Copy, Clone)]
pub struct Camera {
    pub position: Vec3,
    /// The point the camera looks at.
    pub target: Vec3,
    pub up: Vec3,
    /// Vertical field of view, in radians.
    pub fov_y: f32,
    pub near: f32,
    pub far: f32,
    /// The window, in physical pixels, kept in step by the renderer.
    ///
    /// The size and not only its ratio: a cursor cannot be turned back into a
    /// position in the picture without knowing how big the picture is.
    viewport: Vec2,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            position: Vec3::new(0.0, 1.0, 5.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            fov_y: 60f32.to_radians(),
            near: 0.1,
            far: 100.0,
            viewport: Vec2::ONE,
        }
    }

    pub fn aspect(&self) -> f32 {
        self.viewport.x / self.viewport.y
    }

    /// The window the camera is drawing into, in physical pixels.
    pub fn viewport(&self) -> Vec2 {
        self.viewport
    }

    /// Called by the renderer when the surface changes size. A zero-sized
    /// window would make the projection meaningless, so it is ignored.
    pub(crate) fn set_viewport(&mut self, width: f32, height: f32) {
        if width > 0.0 && height > 0.0 {
            self.viewport = Vec2::new(width, height);
        }
    }

    pub fn view(&self) -> Mat4 {
        glam::camera::rh::view::look_at_mat4(self.position, self.target, self.up)
    }

    pub fn projection(&self) -> Mat4 {
        glam::camera::rh::proj::directx::perspective(self.fov_y, self.aspect(), self.near, self.far)
    }

    /// What the GPU gets: one matrix taking a world position to clip space.
    pub fn view_projection(&self) -> Mat4 {
        self.projection() * self.view()
    }

    /// The ray a cursor points along, per spec 0025.
    ///
    /// `cursor` is in physical pixels from the window's top left with y going
    /// down, which is where spec 0001 puts everything and where spec 0013
    /// reports it.
    ///
    /// Built by unprojecting twice rather than from the camera's position.
    /// Taking the eye as the origin is the same picture today and only today:
    /// under a perspective projection every ray starts there, and under the
    /// orthographic one spec 0008 leaves out they are parallel and start
    /// nowhere near it. Two unprojections cost one more matrix multiply and
    /// stay right either way.
    ///
    /// A cursor outside the window still gives a ray, pointing where the
    /// window would have been looking. Whether that means anything is the
    /// game's question.
    pub fn ray_through(&self, cursor: Vec2) -> Ray {
        let picture = Vec2::new(
            cursor.x / self.viewport.x * 2.0 - 1.0,
            1.0 - cursor.y / self.viewport.y * 2.0,
        );

        let back = self.view_projection().inverse();
        // depth zero is the near plane and one the far plane, per spec 0008,
        // and each point is divided by its own w
        let at = |depth: f32| {
            let out = back * Vec4::new(picture.x, picture.y, depth, 1.0);

            out.truncate() / out.w
        };

        let near = at(0.0);

        Ray::new(near, at(1.0) - near)
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{vec2, vec3};

    /// True when a world point lands inside the box the GPU keeps: x and y
    /// within -w to w, and depth within 0 to w.
    fn is_on_screen(camera: &Camera, point: Vec3) -> bool {
        let clip = camera.view_projection() * point.extend(1.0);

        clip.w > 0.0
            && clip.x.abs() <= clip.w
            && clip.y.abs() <= clip.w
            && (0.0..=clip.w).contains(&clip.z)
    }

    #[test]
    fn view_matrix_places_the_camera() {
        let camera = Camera::new();
        // the camera's own position is the origin of view space
        let eye = camera.view() * camera.position.extend(1.0);

        assert!(eye.truncate().length() < 1e-5, "eye was {:?}", eye);

        // and it looks down -z, so the target is in front of it
        let target = camera.view() * camera.target.extend(1.0);
        assert!(target.z < 0.0, "target was {:?}", target);
    }

    #[test]
    fn projection_matches_wgpu_clip_space() {
        let camera = Camera::new();
        let projection = camera.projection();

        let near = projection * glam::vec4(0.0, 0.0, -camera.near, 1.0);
        let far = projection * glam::vec4(0.0, 0.0, -camera.far, 1.0);

        assert!(
            (near.z / near.w).abs() < 1e-5,
            "near was {}",
            near.z / near.w
        );
        assert!(
            (far.z / far.w - 1.0).abs() < 1e-5,
            "far was {}",
            far.z / far.w
        );
    }

    #[test]
    fn the_viewport_is_kept_and_aspect_still_follows_it() {
        // the size and not only its ratio: a cursor cannot be turned back into
        // a position in the picture without knowing how big the picture is
        let mut camera = Camera::new();
        camera.set_viewport(1600.0, 900.0);

        assert_eq!(camera.viewport(), vec2(1600.0, 900.0));
        assert_eq!(camera.aspect(), 16.0 / 9.0);

        camera.set_viewport(800.0, 800.0);

        assert_eq!(camera.viewport(), vec2(800.0, 800.0));
        assert_eq!(camera.aspect(), 1.0);
    }

    #[test]
    fn a_zero_viewport_is_ignored() {
        // a minimized window reports zero, and a projection through it is
        // meaningless rather than merely wrong
        let mut camera = Camera::new();
        camera.set_viewport(800.0, 800.0);
        camera.set_viewport(0.0, 0.0);

        assert_eq!(camera.viewport(), vec2(800.0, 800.0));
        assert_eq!(camera.aspect(), 1.0);
    }

    /// A camera looking at the origin from along +z, with a window to point at.
    fn looking() -> Camera {
        let mut camera = Camera::new();
        camera.position = vec3(0.0, 0.0, 10.0);
        camera.target = Vec3::ZERO;
        camera.set_viewport(1600.0, 900.0);

        camera
    }

    #[test]
    fn the_middle_of_the_window_looks_at_the_target() {
        let camera = looking();
        let ray = camera.ray_through(camera.viewport() * 0.5);

        let toward = (camera.target - camera.position).normalize();

        assert!(
            ray.direction.dot(toward) > 0.9999,
            "the middle points {:?}, not {:?}",
            ray.direction,
            toward
        );
    }

    #[test]
    fn a_ray_goes_back_through_the_point_that_made_it() {
        // the whole of what this is for: a world point projected to a pixel,
        // and that pixel's ray passing back through the point
        let camera = looking();
        let point = vec3(1.5, -0.8, 2.0);

        let clip = camera.view_projection() * point.extend(1.0);
        let ndc = clip.truncate() / clip.w;
        let cursor = vec2(
            (ndc.x * 0.5 + 0.5) * camera.viewport().x,
            (1.0 - (ndc.y * 0.5 + 0.5)) * camera.viewport().y,
        );

        let ray = camera.ray_through(cursor);
        let along = (point - ray.origin).dot(ray.direction);
        let nearest = ray.at(along);

        assert!(
            (nearest - point).length() < 1e-3,
            "the ray passes {:?}, not {:?}",
            nearest,
            point
        );
    }

    #[test]
    fn a_cursor_ray_starts_at_the_near_plane() {
        let camera = looking();
        let ray = camera.ray_through(camera.viewport() * 0.5);

        let off = (ray.origin - camera.position).length();

        assert!(
            (off - camera.near).abs() < 1e-3,
            "it starts {} from the eye, not {}",
            off,
            camera.near
        );
    }

    #[test]
    fn a_cursor_ray_is_normalised() {
        // so a distance a ray cast reports is a distance in world units rather
        // than a multiple of the near to far span
        let camera = looking();

        for at in [vec2(0.0, 0.0), vec2(1600.0, 900.0), vec2(300.0, 700.0)] {
            let ray = camera.ray_through(at);

            assert!(
                (ray.direction.length() - 1.0).abs() < 1e-5,
                "{:?} gave a direction of {}",
                at,
                ray.direction.length()
            );
        }
    }

    #[test]
    fn the_two_sides_of_the_window_point_to_their_own_sides() {
        let camera = looking();
        let middle = camera.viewport().y * 0.5;

        let left = camera.ray_through(vec2(10.0, middle));
        let right = camera.ray_through(vec2(1590.0, middle));

        assert!(
            left.direction.x < 0.0,
            "the left points {:?}",
            left.direction
        );
        assert!(
            right.direction.x > 0.0,
            "the right points {:?}",
            right.direction
        );
    }

    #[test]
    fn the_corners_spread_outwards() {
        let camera = looking();
        let middle = camera.ray_through(camera.viewport() * 0.5);

        let corners = [
            vec2(0.0, 0.0),
            vec2(1600.0, 0.0),
            vec2(0.0, 900.0),
            vec2(1600.0, 900.0),
        ];

        for corner in corners {
            let ray = camera.ray_through(corner);

            assert!(
                ray.direction.dot(middle.direction) < 0.99,
                "{:?} points the same way as the middle",
                corner
            );
        }

        // and away from each other rather than converging
        let top_left = camera.ray_through(corners[0]).direction;
        let bottom_right = camera.ray_through(corners[3]).direction;

        assert!(top_left.x < 0.0 && top_left.y > 0.0, "{:?}", top_left);
        assert!(
            bottom_right.x > 0.0 && bottom_right.y < 0.0,
            "{:?}",
            bottom_right
        );
    }

    #[test]
    fn a_ray_through_a_box_hits_it() {
        // what this is for: the ray goes straight into the cast spec 0014
        // already does, so pointing at a thing is this and then that
        let camera = looking();
        let box_ = crate::collision::Aabb::from_center_size(Vec3::ZERO, Vec3::splat(2.0));

        let middle = camera.ray_through(camera.viewport() * 0.5);
        assert!(middle.hit_aabb(&box_).is_some(), "the middle missed it");

        let corner = camera.ray_through(Vec2::ZERO);
        assert!(corner.hit_aabb(&box_).is_none(), "a corner hit it");
    }

    #[test]
    fn a_cursor_off_the_edge_still_gives_a_ray() {
        // the arithmetic holds past the edges, and whether a cursor that has
        // left the window means anything is the game's question
        let camera = looking();
        let ray = camera.ray_through(vec2(-400.0, 1400.0));

        assert!((ray.direction.length() - 1.0).abs() < 1e-5);
        assert!(ray.direction.x < 0.0, "{:?}", ray.direction);
        assert!(ray.direction.y < 0.0, "{:?}", ray.direction);
    }

    #[test]
    fn the_default_camera_sees_the_origin() {
        let camera = Camera::new();

        assert!(is_on_screen(&camera, Vec3::ZERO));
        // and a point behind the camera is not on screen
        assert!(!is_on_screen(&camera, Vec3::new(0.0, 1.0, 10.0)));
    }

    #[test]
    fn moving_the_camera_changes_the_view() {
        let camera = Camera::new();
        let before = camera.view();

        // a pan: position and target move together, so the camera keeps facing
        // the same way and the world slides across the screen
        let mut panned = camera;
        panned.position += Vec3::new(2.0, 0.0, 0.0);
        panned.target += Vec3::new(2.0, 0.0, 0.0);

        assert_ne!(before, panned.view());

        let was = camera.view_projection() * Vec3::ZERO.extend(1.0);
        let now = panned.view_projection() * Vec3::ZERO.extend(1.0);
        assert!(
            now.x / now.w < was.x / was.w,
            "the origin should sit further left after panning right"
        );
    }

    #[test]
    fn the_target_stays_centered_when_the_camera_orbits() {
        // moving the position alone swings the camera around, since it keeps
        // looking at its target
        let mut orbited = Camera::new();
        orbited.position = Vec3::new(4.0, 2.0, -3.0);

        let clip = orbited.view_projection() * orbited.target.extend(1.0);

        assert!((clip.x / clip.w).abs() < 1e-5);
        assert!((clip.y / clip.w).abs() < 1e-5);
    }
}
