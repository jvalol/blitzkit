# 0042 A mesh that changes

**Status:** implemented
**Date:** 2026-10-06

## Goal

Let a game move the vertices of a mesh it has already uploaded, so a surface
whose shape is worked out every frame can be drawn without leaking a new mesh
every frame.

## Behavior

**Uploading** is unchanged. `Renderer::add_mesh` takes a `MeshData` and hands
back a `MeshId`, and that handle is good for the life of the renderer.

**Changing** one is `Renderer::update_mesh(id, data)`. It replaces the vertices
and the indices of the mesh that handle names, and takes effect on the next
frame drawn. Everything already pushed with that handle draws the new shape:
a mesh is one upload shared by every instance of it, so changing it changes
all of them. A game that wants two shapes wants two meshes.

The new data need not be the same size as the old. While it fits in the buffers
already there, the bytes are written into them, which is what a surface stepped
every frame does and costs no allocation. When it does not fit, both buffers are
replaced with ones that do. Buffers are therefore never shrunk: a mesh that
grows and shrinks keeps the largest buffer it ever needed, which trades a little
memory for never allocating twice for the same shape.

An id no mesh was ever given is ignored rather than fatal. An id can only come
from `add_mesh`, so this is a game that kept a handle from a renderer it no
longer has, and losing a shape is better than losing the frame.

**An empty mesh** is legal, as it is for `add_mesh`: a mesh updated to no
triangles draws nothing and keeps its handle, which is how a surface that is
not there this frame is said. Updating it again brings it back.

**What does not change.** The material, the texture and the transform are
properties of the push, not of the mesh, so none of them is affected. Normals
are the mesh's own, so a game that moves vertices and wants the lighting to
follow computes them in the `MeshData` it passes, the way `MeshData::surface`
does from the formula it is given.

## Acceptance criteria

- A buffer that can take the new bytes is kept and written into. — `renderer::tests::a_buffer_that_fits_is_kept`
- A buffer too small for them is replaced with one big enough. — `renderer::tests::a_buffer_too_small_is_replaced`
- A buffer is never shrunk by an update that needs less room. — `renderer::tests::a_buffer_is_never_shrunk`
- Nothing to write asks for no buffer, so an empty mesh is legal to update to. — `renderer::tests::empty_data_asks_for_nothing`

An id no mesh was ever given is a miss on a `Vec`, which is the one claim here
with nothing to fail: `meshes.get_mut` returns nothing and the call ends.

### Verified by hand

Run `cargo run --example ripple` in blitzkit.

- The surface moves every frame and is lit as it moves, which is normals
  arriving with the new vertices rather than being left behind.
- Nothing grows without bound: the example runs for minutes at a steady frame
  rate, which it would not do if each frame added a mesh.

## Out of scope

**Changing part of a mesh.** The whole thing is replaced. A game that moves ten
vertices of a thousand pays for a thousand, which at the sizes this engine draws
is cheaper than the bookkeeping to do better.

**Changing a mesh from more than one place in a frame.** The last call before
the frame is drawn is the one that counts.

**Removing a mesh.** Handles are never reused, and nothing is freed until the
renderer is.
