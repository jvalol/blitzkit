# 0010 Meshes

**Status:** implemented
**Date:** 2026-09-21

## Goal

Something to draw in 3D, placed where the game wants it.

## Behavior

A 3D vertex is a position, a normal, and a texture coordinate: `Vec3`, `Vec3`,
`Vec2`. That is a separate type from the 2D vertex, which stays as it is.

A `Mesh` owns a vertex buffer and an index buffer on the GPU, built once from
vertex and index data rather than rebuilt every frame the way quads are. Meshes
are long lived: a game builds one and draws it many times.

The engine ships a cube, a plane and a sphere, so a game can see something
without supplying geometry. The sphere takes a segment and ring count, since how
round it needs to be is the game's call.

**Drawing.** A game uploads meshes once in `Game::load` and pushes a mesh and a
transform into the `Scene` each frame in `Game::draw`, the same shape as
`push_quad`. Both have empty defaults, so a 2D game ignores them. The transform is a `Mat4`, and a
`Transform` helper builds one from position, rotation and scale. Drawing the
same mesh many times is instanced, one draw call per mesh with the transforms in
an instance buffer.

Normals are transformed by the inverse transpose of the model matrix, so
non-uniform scaling does not bend the lighting.

**Every built-in mesh is wound counter-clockwise seen from outside**, which is
what spec 0009's culling keeps, and wears its texture the right way round. Those
are two things a mesh can get wrong on its own and both are invisible on a still
shape, so each has a test that reads every triangle of every built-in.

The sphere had both wrong, and they hid each other. Its triangles ran clockwise
from outside, so the renderer culled every outward face and drew the inside of
the far side; its longitude ran the other way, so the texture was mirrored to
match. A still ball looked right. A turning one did not: the far surface moves
against the near one, so marble's ball rolled backwards when it rolled towards
or away from the camera, and correctly when it rolled across. That is the shape
of the bug, and it is why the tests check the invariants rather than the
pictures.

The texture test is the handedness: an image is read with u across and v down,
so laying it on a surface puts u cross v into that surface. Coming out means the
image is on backwards, and letters on it would read in a mirror.

**Loading from a file** is `Mesh::from_obj`, using `tobj`. Wavefront OBJ is
enough to get real models on screen, and it is one dependency rather than the
several glTF wants.

## Acceptance criteria

- A 3D vertex carries a position, a normal and a texture coordinate. — `mesh::tests::vertex_has_position_normal_and_uv`
- The built-in cube has 8 corners, 12 triangles, and normals per face. — `mesh::tests::the_cube_is_a_cube`
- A transform built from position, rotation and scale applies them in that order. — `mesh::tests::transform_applies_scale_rotation_then_position`
- The normal matrix is the inverse transpose of the model matrix. — `mesh::tests::normal_matrix_survives_non_uniform_scale`
- Pushing the same mesh twice makes one draw call with two instances. — `renderer::scene::tests::repeated_meshes_are_instanced`
- An instance carries its transform and color. — `renderer::scene::tests::an_instance_carries_its_transform_and_color`
- The instance layout's offsets match the struct's real ones. — `renderer::scene::tests::the_instance_layout_matches_the_struct`
- The vertex layout's offsets match too. — `mesh::tests::vertex_has_position_normal_and_uv`
- A mesh loaded without normals gets them computed. — `mesh::tests::computes_missing_normals`
- The built-in plane faces up. — `mesh::tests::the_plane_faces_up`
- The built-in sphere is round, with normals pointing out. — `mesh::tests::the_sphere_is_round`
- Every built-in mesh is wound the way its normals face. — `mesh::tests::every_mesh_is_wound_the_way_it_faces`
- No built-in mesh wears its texture mirrored. — `mesh::tests::no_mesh_wears_its_texture_mirrored`
- An OBJ file loads into vertices and indices. — `mesh::tests::loads_an_obj_file`

### Verified by hand

Run `cargo run --example cubes` in blitzkit. The camera orbits on its own.

- The cube looks like a cube from every angle, with no glimpses of its inside.
- The near cube covers the far one although it is pushed later, which is depth
  testing rather than draw order, per spec 0009.
- The text stays on top of the world.

Until lighting lands in spec 0012 the shader is flat colored, so a cube's faces
do not differ in brightness. The example spins one and gives it a floor for that
reason.

## Out of scope

Skeletal animation, glTF, materials per submesh, level of detail, and any scene
graph. A game keeps its own list of what to draw.
