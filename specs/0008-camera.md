# 0008 Camera

**Status:** implemented
**Date:** 2026-09-21

## Goal

A viewpoint a game can move, which is what separates a 3D scene from a picture.

## Behavior

A `Camera` holds a position, a target it looks at, an up direction, a vertical
field of view, and near and far planes. It produces a view matrix and a
projection matrix, and the engine sends their product to the GPU in a uniform,
the same way the screen size uniform already works. The matrices come from
`glam::camera::rh::proj::directx::perspective` and `look_at_mat4`, per spec
0007.

The aspect ratio comes from the surface and is updated on resize, so the picture
doesn't stretch. The uniform is written once per frame rather than per object.

A game sets the camera through the renderer with `set_camera`, and reads it back
with `camera()`. There is no camera controller in the engine: a game moves its
own camera, the way it moves everything else.

The `Game` trait does not hand a game the renderer yet, so nothing can move the
camera from a game until meshes arrive in spec 0010 and there is something 3D to
look at. The uniform is written every frame regardless, ready for the pipeline
that binds it.

**Defaults** so a game can draw something before thinking about cameras: at
`(0, 1, 5)` looking at the origin, 60 degree field of view, near 0.1, far 100.

Text and quads keep drawing in pixels and ignore the camera entirely. They are
separate pipelines and always have been.

## Acceptance criteria

- The view matrix puts the camera where it says it is. — `camera::tests::view_matrix_places_the_camera`
- A point at the near plane lands at depth 0, at the far plane at 1. — `camera::tests::projection_matches_wgpu_clip_space`
- Aspect ratio follows the window. — `camera::tests::aspect_follows_the_window`
- The default camera can see the origin. — `camera::tests::the_default_camera_sees_the_origin`
- Panning the camera slides the world across the screen. — `camera::tests::moving_the_camera_changes_the_view`
- Moving the position alone keeps the target centered, since the camera looks at it. — `camera::tests::the_target_stays_centered_when_the_camera_orbits`

### Verified by hand

- Moving the camera in a game moves the scene the expected way. — run the 3D
  example and drive the camera.

## Out of scope

Orthographic projection, several cameras at once, frustum culling, and any
built-in camera controller.

Orthographic was tried rather than merely assumed: starry is a flat board seen
from above, which is the case that wants it most, and its spec 0002 records the
answer on 2026-09-28. From far enough back at a narrow field of view the splay
is still visible and reads as a board tilted away from the viewer, which helps
the thickness rather than hurting it. Nothing has wanted orthographic since.
Written down because this line sat here for a week with no reason attached and
nobody could remember why.
