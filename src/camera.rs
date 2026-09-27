//! Where the scene is looked at from, and the matrices that follow.
//!
//! Right-handed with y up, and the projection is where that becomes the
//! screen's coordinates. See `specs/0008-camera.md`.

use glam::{Mat4, Vec3};

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
    /// Width over height, kept in step with the window by the renderer.
    aspect: f32,
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
            aspect: 1.0,
        }
    }

    pub fn aspect(&self) -> f32 {
        self.aspect
    }

    /// Called by the renderer when the surface changes size. A zero-sized
    /// window would make the projection meaningless, so it is ignored.
    pub(crate) fn set_viewport(&mut self, width: f32, height: f32) {
        if width > 0.0 && height > 0.0 {
            self.aspect = width / height;
        }
    }

    pub fn view(&self) -> Mat4 {
        glam::camera::rh::view::look_at_mat4(self.position, self.target, self.up)
    }

    pub fn projection(&self) -> Mat4 {
        glam::camera::rh::proj::directx::perspective(self.fov_y, self.aspect, self.near, self.far)
    }

    /// What the GPU gets: one matrix taking a world position to clip space.
    pub fn view_projection(&self) -> Mat4 {
        self.projection() * self.view()
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
    fn aspect_follows_the_window() {
        let mut camera = Camera::new();
        camera.set_viewport(1600.0, 900.0);
        assert_eq!(camera.aspect(), 16.0 / 9.0);

        camera.set_viewport(800.0, 800.0);
        assert_eq!(camera.aspect(), 1.0);

        // a minimized window reports zero and is ignored rather than dividing by it
        camera.set_viewport(0.0, 0.0);
        assert_eq!(camera.aspect(), 1.0);
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
