//! The 2D half: coloured quads a game pushes each frame, and their vertices.
//!
//! Positions are in physical pixels with the origin at the top left and y going
//! down. See `specs/0001-pixel-coordinates.md` and `0006-quad-color.md`.

pub mod quad;
pub mod vertex;
use crate::renderer::scene::TextureId;
use quad::Quad;

/// A stretch of consecutive quads wearing one texture, which is one draw.
///
/// `first` and `count` are into the index data, so a run draws
/// `first..first + count` of it.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Run {
    pub texture: TextureId,
    pub first: u32,
    pub count: u32,
    /// The window it is cut to, or the whole window. Spec 0113.
    pub clip: Option<(glam::Vec2, glam::Vec2)>,
}

pub struct Geometry {
    vertex_data: Vec<vertex::Vertex>,
    index_data: Vec<u32>,
    /// **Runs rather than groups.** A draw call wears one texture, so the
    /// obvious saving is to gather every quad of one image into a single
    /// draw. It is wrong here: quads have no depth buffer and a later one
    /// covers an earlier one, so the order they are drawn in is the only
    /// thing deciding what covers what. Sorting them by texture changes the
    /// picture. Spec 0024.
    runs: Vec<Run>,
    pub num_quads: u32,
}

impl Default for Geometry {
    fn default() -> Self {
        Self::new()
    }
}

impl Geometry {
    pub fn new() -> Self {
        Self {
            vertex_data: Vec::new(),
            index_data: Vec::new(),
            runs: Vec::new(),
            num_quads: 0,
        }
    }

    pub fn reset(&mut self) {
        self.vertex_data.clear();
        self.index_data.clear();
        self.runs.clear();
        self.num_quads = 0;
    }

    pub fn push_quad(&mut self, quad: &Quad) {
        // **The corners are worked out here and not in the shader.** The
        // vertex buffer already carries a position a pixel at a time, so a
        // quad that turns is four corners turned on the way in and no change
        // to the pipeline at all. Spec 0112.
        let (sin, cos) = quad.turn.sin_cos();
        let half = quad.size * 0.5;
        let corner = |x: f32, y: f32| {
            let off = glam::vec2(x * half.x, y * half.y);

            quad.position + glam::vec2(off.x * cos - off.y * sin, off.x * sin + off.y * cos)
        };

        // uv_min at the top-left corner, which is the one that is not
        // obvious: y points down in this space, so min_y is the top.
        //
        // The uv does not turn with the corners, so the image turns with the
        // rectangle rather than sliding about inside it.
        let (top_left, top_right) = (corner(-1.0, -1.0), corner(1.0, -1.0));
        let (bottom_right, bottom_left) = (corner(1.0, 1.0), corner(-1.0, 1.0));

        self.vertex_data.extend(&[
            vertex::Vertex {
                position: top_left.to_array(),
                color: quad.color.to_array(),
                uv: [quad.uv_min.x, quad.uv_min.y],
            },
            vertex::Vertex {
                position: top_right.to_array(),
                color: quad.color.to_array(),
                uv: [quad.uv_max.x, quad.uv_min.y],
            },
            vertex::Vertex {
                position: bottom_right.to_array(),
                color: quad.color.to_array(),
                uv: [quad.uv_max.x, quad.uv_max.y],
            },
            vertex::Vertex {
                position: bottom_left.to_array(),
                color: quad.color.to_array(),
                uv: [quad.uv_min.x, quad.uv_max.y],
            },
        ]);
        self.index_data.extend(&[
            (self.num_quads * 4),
            self.num_quads * 4 + 1,
            self.num_quads * 4 + 2,
            (self.num_quads * 4),
            self.num_quads * 4 + 2,
            self.num_quads * 4 + 3,
        ]);
        self.num_quads += 1;

        // A run wears one texture and one scissor, so either one changing
        // starts another draw.
        match self.runs.last_mut() {
            Some(run) if run.texture == quad.texture && run.clip == quad.clip => run.count += 6,
            _ => self.runs.push(Run {
                texture: quad.texture,
                first: self.index_data.len() as u32 - 6,
                count: 6,
                clip: quad.clip,
            }),
        }
    }

    /// The draws, in the order the quads were pushed.
    pub fn runs(&self) -> &[Run] {
        &self.runs
    }

    pub(crate) fn vertex_data(&self) -> &[vertex::Vertex] {
        &self.vertex_data
    }

    pub(crate) fn index_data(&self) -> &[u32] {
        &self.index_data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corners(geometry: &Geometry) -> Vec<(f32, f32)> {
        geometry
            .vertex_data()
            .iter()
            .map(|vertex| (vertex.position[0], vertex.position[1]))
            .collect()
    }

    #[test]
    fn push_quad_makes_corner_vertices() {
        let mut geometry = Geometry::new();
        geometry.push_quad(&Quad::new((100.0, 50.0).into(), (20.0, 10.0).into()));

        assert_eq!(
            corners(&geometry),
            vec![(90.0, 45.0), (110.0, 45.0), (110.0, 55.0), (90.0, 55.0)]
        );
        assert_eq!(geometry.num_quads, 1);
    }

    #[test]
    fn push_quad_carries_its_color_to_every_vertex() {
        let mut geometry = Geometry::new();
        let red = glam::vec4(1.0, 0.0, 0.0, 1.0);
        geometry.push_quad(&Quad::colored((0.0, 0.0).into(), (2.0, 2.0).into(), red));

        assert!(geometry
            .vertex_data()
            .iter()
            .all(|vertex| vertex.color == red.to_array()));
    }

    #[test]
    fn quads_use_glam_types() {
        let quad = Quad::colored(
            glam::vec2(10.0, 20.0),
            glam::vec2(4.0, 4.0),
            glam::vec4(0.0, 1.0, 0.0, 1.0),
        );

        assert_eq!(quad.position, glam::Vec2::new(10.0, 20.0));
        assert_eq!(quad.size.x, 4.0);
        assert_eq!(quad.color.y, 1.0);
        assert_eq!(quad::WHITE, glam::Vec4::ONE);
    }

    #[test]
    fn quads_default_to_white() {
        let mut geometry = Geometry::new();
        geometry.push_quad(&Quad::new((0.0, 0.0).into(), (2.0, 2.0).into()));

        assert!(geometry
            .vertex_data()
            .iter()
            .all(|vertex| vertex.color == quad::WHITE.to_array()));
    }

    /// Spec 0112: a quarter turn puts each corner where the one before it
    /// was, and the middle does not move.
    ///
    /// **Clockwise, which is what y pointing down makes it.** Written as the
    /// corners it actually lands on rather than as a rotation applied twice,
    /// so a sign flip in `push_quad` cannot agree with a sign flip here.
    #[test]
    fn a_turned_quad_puts_its_corners_round_its_middle() {
        let mut geometry = Geometry::new();
        geometry.push_quad(
            &Quad::new((100.0, 50.0).into(), (20.0, 10.0).into())
                .turned(std::f32::consts::FRAC_PI_2),
        );

        let got = corners(&geometry);
        let want = [(105.0, 40.0), (105.0, 60.0), (95.0, 60.0), (95.0, 40.0)];

        for (n, (at, should)) in got.iter().zip(want.iter()).enumerate() {
            assert!(
                (at.0 - should.0).abs() < 1e-3 && (at.1 - should.1).abs() < 1e-3,
                "corner {} of a quarter turn is at {:?} and belongs at {:?}",
                n,
                at,
                should
            );
        }
    }

    /// Spec 0112: no turn is the rectangle it always was.
    #[test]
    fn no_turn_is_the_square_rectangle() {
        let mut geometry = Geometry::new();
        geometry.push_quad(&Quad::new((100.0, 50.0).into(), (20.0, 10.0).into()).turned(0.0));

        assert_eq!(
            corners(&geometry),
            vec![(90.0, 45.0), (110.0, 45.0), (110.0, 55.0), (90.0, 55.0)]
        );
    }

    /// Spec 0112: the image turns with the rectangle rather than sliding
    /// about inside it, which is what leaving the uv alone buys.
    #[test]
    fn a_turn_leaves_the_image_on_the_face_it_was_on() {
        let mut turned = Geometry::new();
        let mut square = Geometry::new();
        let quad = Quad::new((100.0, 50.0).into(), (20.0, 10.0).into());

        turned.push_quad(&quad.turned(0.9));
        square.push_quad(&quad);

        let uvs = |geometry: &Geometry| -> Vec<[f32; 2]> {
            geometry.vertex_data().iter().map(|one| one.uv).collect()
        };

        assert_eq!(uvs(&turned), uvs(&square));
    }

    /// Spec 0113: one run wears one scissor, so changing it starts another
    /// draw even where the texture is the same.
    #[test]
    fn a_run_breaks_when_the_window_it_is_cut_to_changes() {
        let quad = Quad::new((10.0, 10.0).into(), (4.0, 4.0).into());
        let panel = (glam::vec2(0.0, 0.0), glam::vec2(50.0, 50.0));

        let mut same = Geometry::new();
        same.push_quad(&quad.clipped_to(panel.0, panel.1));
        same.push_quad(&quad.clipped_to(panel.0, panel.1));

        let mut apart = Geometry::new();
        apart.push_quad(&quad.clipped_to(panel.0, panel.1));
        apart.push_quad(&quad);

        assert_eq!(same.runs().len(), 1, "one window, and it drew twice");
        assert_eq!(apart.runs().len(), 2, "two windows, and it drew once");
        assert_eq!(apart.runs()[0].clip, Some(panel));
        assert_eq!(apart.runs()[1].clip, None);
    }

    #[test]
    fn push_quad_makes_two_triangles() {
        let mut geometry = Geometry::new();
        geometry.push_quad(&Quad::new((0.0, 0.0).into(), (2.0, 2.0).into()));

        assert_eq!(geometry.index_data(), &[0, 1, 2, 0, 2, 3]);
    }

    #[test]
    fn second_quad_indices_are_offset() {
        let mut geometry = Geometry::new();
        geometry.push_quad(&Quad::new((0.0, 0.0).into(), (2.0, 2.0).into()));
        geometry.push_quad(&Quad::new((8.0, 8.0).into(), (2.0, 2.0).into()));

        assert_eq!(geometry.vertex_data().len(), 8);
        assert_eq!(geometry.index_data()[6..], [4, 5, 6, 4, 6, 7]);
        assert_eq!(geometry.num_quads, 2);
    }

    /// Spec 0024. An image this crate's tests can name without a device:
    /// `TextureId`'s index is crate-visible on purpose.
    fn image(n: usize) -> TextureId {
        TextureId(n)
    }

    #[test]
    fn quads_default_to_the_white_texture() {
        let quad = Quad::new((0.0, 0.0).into(), (2.0, 2.0).into());

        assert_eq!(quad.texture, TextureId::WHITE);
        assert_eq!(
            Quad::colored((0.0, 0.0).into(), (2.0, 2.0).into(), glam::Vec4::ONE).texture,
            TextureId::WHITE
        );
    }

    #[test]
    fn quads_default_to_the_whole_image() {
        let quad = Quad::textured((0.0, 0.0).into(), (2.0, 2.0).into(), image(3));

        assert_eq!((quad.uv_min, quad.uv_max), quad::WHOLE);
    }

    /// The one the corner arithmetic cannot get right by accident: y points
    /// down here, so `uv_min` belongs to the vertex at `min_y`, which is the
    /// top of the quad and not the bottom.
    #[test]
    fn push_quad_puts_each_uv_corner_on_its_own_vertex() {
        let mut geometry = Geometry::new();
        geometry.push_quad(&Quad::part_of(
            (100.0, 50.0).into(),
            (20.0, 10.0).into(),
            image(1),
            glam::vec2(0.25, 0.5),
            glam::vec2(0.75, 1.0),
        ));

        let corners: Vec<_> = geometry
            .vertex_data()
            .iter()
            .map(|vertex| (vertex.position, vertex.uv))
            .collect();

        assert_eq!(
            corners,
            vec![
                ([90.0, 45.0], [0.25, 0.5]),
                ([110.0, 45.0], [0.75, 0.5]),
                ([110.0, 55.0], [0.75, 1.0]),
                ([90.0, 55.0], [0.25, 1.0]),
            ]
        );
    }

    #[test]
    fn one_texture_in_a_row_is_one_draw() {
        let mut geometry = Geometry::new();

        for n in 0..10 {
            geometry.push_quad(&Quad::textured(
                (n as f32, 0.0).into(),
                (2.0, 2.0).into(),
                image(4),
            ));
        }

        assert_eq!(
            geometry.runs(),
            &[Run {
                texture: image(4),
                first: 0,
                count: 60,
                clip: None,
            }]
        );
    }

    #[test]
    fn a_new_texture_starts_a_new_draw() {
        let mut geometry = Geometry::new();
        geometry.push_quad(&Quad::textured(
            (0.0, 0.0).into(),
            (2.0, 2.0).into(),
            image(4),
        ));
        geometry.push_quad(&Quad::textured(
            (4.0, 0.0).into(),
            (2.0, 2.0).into(),
            image(5),
        ));

        assert_eq!(geometry.runs().len(), 2);
        assert_eq!(geometry.runs()[1].first, 6);
        assert_eq!(geometry.runs()[1].count, 6);
    }

    /// The saving nobody gets to have. Gathering both of image 4's quads into
    /// one draw would put the second of them over image 5's, and there is no
    /// depth buffer here to say otherwise.
    #[test]
    fn coming_back_to_a_texture_does_not_rejoin_it() {
        let mut geometry = Geometry::new();

        for which in [4, 5, 4] {
            geometry.push_quad(&Quad::textured(
                (0.0, 0.0).into(),
                (2.0, 2.0).into(),
                image(which),
            ));
        }

        assert_eq!(geometry.runs().len(), 3);
        assert_eq!(geometry.runs()[0].texture, image(4));
        assert_eq!(geometry.runs()[2].texture, image(4));
        assert_eq!(geometry.runs()[2].first, 12);
    }

    #[test]
    fn the_draws_keep_the_order_they_were_pushed_in() {
        let mut geometry = Geometry::new();
        let pushed = [7, 7, 2, 9, 9, 9, 2];

        for which in pushed {
            geometry.push_quad(&Quad::textured(
                (0.0, 0.0).into(),
                (2.0, 2.0).into(),
                image(which),
            ));
        }

        let drawn: Vec<_> = geometry.runs().iter().map(|run| run.texture).collect();

        assert_eq!(drawn, [7, 2, 9, 2].map(image).to_vec());

        // and every index is drawn once, in order, by exactly one run
        let mut at = 0;

        for run in geometry.runs() {
            assert_eq!(run.first, at);
            at += run.count;
        }

        assert_eq!(at as usize, geometry.index_data().len());
    }

    #[test]
    fn nothing_pushed_is_nothing_drawn() {
        let mut geometry = Geometry::new();

        assert!(geometry.runs().is_empty());

        geometry.push_quad(&Quad::new((0.0, 0.0).into(), (2.0, 2.0).into()));
        geometry.reset();

        assert!(geometry.runs().is_empty());
    }

    #[test]
    fn reset_clears_geometry() {
        let mut geometry = Geometry::new();
        geometry.push_quad(&Quad::new((0.0, 0.0).into(), (2.0, 2.0).into()));
        geometry.reset();

        assert!(geometry.vertex_data().is_empty());
        assert!(geometry.index_data().is_empty());
        assert_eq!(geometry.num_quads, 0);
    }
}
