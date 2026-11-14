# 0024 Textured quads

**Status:** implemented
**Date:** 2026-09-28, built 2026-11-14

## Goal

A picture on screen in 2D. Spec 0011 put images on meshes, so the engine can
texture a floor in 3D but cannot put one behind a menu, and every 2D game built
on it so far draws flat colored rectangles.

Nothing is waiting on this. The sliding tile puzzle that prompted it wants
tiles with thickness, which makes them meshes, and meshes have carried per
vertex uvs since spec 0010. This stays a draft until something actually needs
it, because the shape of the first real use is what should settle the questions
the Behavior section leaves open.

**And that reasoning is what gave way.** Jake, 2026-11-14: any engine that
does not support 2D textured quads is incomplete. The waiting argument looks
for a game to ask, and the engine's own claim to be an engine is the thing
asking: a splash screen, a logo, an icon and a sprite are all ordinary, and
until this was built every one of them meant giving a flat rectangle a
thickness so it could be a mesh. starry is the whole story. Its painting is
on meshes, because its tiles have thickness, and its one `push_quad` is a
flat coloured rectangle for the tray. Nothing in it wanted a picture in
screen space, so nothing in this project ever hit the gap, which is not the
same as the gap not being there.

## Behavior

**A quad carries a texture and the part of it to use.** `texture` is a
`TextureId` and `uv` is a rectangle in texture space, `uv_min` at the quad's
top-left corner and `uv_max` at its bottom-right. Defaults are `TextureId::WHITE`
and the whole 0 to 1 rectangle.

**The white texture is what keeps this one pipeline.** Spec 0011 already uploads
a one pixel white image at index 0 so an untextured mesh needs no second shader.
A quad that says nothing samples that pixel and multiplies it by its color,
which is its existing color exactly. There is no untextured path to maintain
beside the textured one.

**Color multiplies what is sampled**, the same rule meshes follow. A white quad
shows the image untouched, and a colored one tints it. Sampled alpha multiplies
the color's alpha and blends against what is already drawn, per spec 0006.

**A draw call wears one texture, and order is what batches.** Quads have no
depth buffer and a later one covers an earlier one, so they cannot be sorted
into texture groups without changing the picture. Consecutive quads sharing a
texture become one draw and a change of texture starts another. Quads that
alternate between two textures are one draw each, and a quad returning to a
texture used earlier starts a new run rather than joining the old one. Ten
tiles of one image are one draw; the same ten interleaved with something else
are twenty.

**The rectangle is not clamped.** Values outside 0 to 1 repeat, because spec
0011 opened the sampler in repeating wrap, so a `uv_max` of 4 tiles the image
four times across the quad. That is the one use for a rectangle larger than the
texture, and it is a use rather than a mistake to reject.

**Neighbouring slices of one image will bleed at their shared edge.** This is
the hazard of the feature and it is not solved here. Sampling is linear and
mipmapped, per spec 0011, so a fragment at the very edge of a sub-rectangle
draws part of its value from texels outside that rectangle, which belong to the
slice next door. Drawn at roughly one texel to the pixel it is invisible. Drawn
much smaller, where a low mip level's texel spans a wide stretch of the image,
it shows as a fringe of the neighbour along every seam.

A game slicing one image into tiles that sit side by side has three ways out,
and the engine takes none of them for it. Draw at or near the source's own
size. Pull the rectangle in by half a texel and accept that half a row is never
shown. Or cut the image into a texture per tile and spend the draw calls. The
engine samples the rectangle it is given, because a rectangle that quietly
becomes a slightly different rectangle is worse to debug than a fringe.

## Acceptance criteria

- A quad that says nothing about texture gets the white one. — `geometry::tests::quads_default_to_the_white_texture`
- A quad that says nothing about uv gets the whole image. — `geometry::tests::quads_default_to_the_whole_image`
- The uv rectangle reaches the four corners the right way round. — `geometry::tests::push_quad_puts_each_uv_corner_on_its_own_vertex`
- And the image lands on the quad that way up, on a GPU. — `pictures::tests::a_quad_wears_its_image_the_right_way_up`
- A color still reaches every vertex. — `geometry::tests::push_quad_carries_its_color_to_every_vertex`
- The vertex layout still matches the struct it claims to describe. — `geometry::vertex::tests::the_layout_matches_the_struct`
- Consecutive quads in one texture are one draw. — `geometry::tests::one_texture_in_a_row_is_one_draw`
- A change of texture starts another draw. — `geometry::tests::a_new_texture_starts_a_new_draw`
- Returning to an earlier texture does not join the earlier draw. — `geometry::tests::coming_back_to_a_texture_does_not_rejoin_it`
- The draws come out in the order the quads were pushed. — `geometry::tests::the_draws_keep_the_order_they_were_pushed_in`
- No quads is no draws. — `geometry::tests::nothing_pushed_is_nothing_drawn`

The batching ones are not decoration. Sorting quads by texture is the obvious
way to cut draw calls and it is wrong here. Without a depth buffer, the order
they are drawn in is the only thing deciding what covers what.

### What was built, and what it found

**One draw per run of one texture**, written as drafted. `Geometry` keeps a
`Vec<Run>` of `(texture, first, count)` into the index data and the overlay
pass walks it, binding the same per-image bind group the mesh pass binds, in
the same group, with the same layout. The quad pipeline is made after that
layout now rather than before it. Nothing sorts.

**The uv goes after the colour in the vertex**, so the two offsets the layout
already claimed do not move, and the struct is 32 bytes where it was 24.

**How the GPU samples can be tested after all**, which this spec said it
could not. A texture bright in one quadrant, half-lit in a second and dark in
the other two says where each quadrant landed, and sampling it transposed,
mirrored or flipped moves one of them somewhere a test can see. Three
deliberate breaks in the shader, three failures. No reference picture, so
nothing to bless and nothing to drift.

The first version of that test was wrong and passed anyway. It had one bright
quadrant at the top left, which is symmetric about the diagonal, so `uv.yx`
in the shader moved nothing at all. The third value is what tells a transpose
from the truth, and finding that took breaking the shader on purpose, which
is the only reason it was found.

**And the seam is one pixel, not a growing fringe.** The hazard section below
says a slice drawn much smaller than its source shows the neighbour along the
seam because a low mip level's texel spans a wide stretch of the image.
Measured on an Apple M5 Max, with a 64 texel image split down the middle and
each half drawn at 64, 32, 16 and 8 pixels, the dark half's first column
reads 137 of 255 at every one of those sizes and its middle reads nought. It
reads 137 with the bright half not drawn at all, so this is not the
neighbouring quad: it is the linear filter at `u` exactly 0.5, averaging the
last bright texel with the first dark one. One pixel wide, at any size.

So the warning is right about there being a seam and wrong about why, and the
three ways out it offers all still work. **No second sampler was added**,
which the Out of scope section still rules out and which a picture of one
pixel does not justify.

## Out of scope

Rotating a quad, which is a separate change to the vertex layout. Nine-slice
scaling. Texture atlases as an engine concept, with names for regions and a
packer to build them: a game that wants one keeps its own rectangles, which is
all an atlas is from here. A second sampler for 2D, which is how mipmapping
would be turned off for quads drawn one to one. Worth revisiting only if the
seam above turns out to matter.
