//! The 2D half: coloured quads a game pushes each frame, and their vertices.
//!
//! Positions are in physical pixels with the origin at the top left and y going
//! down. See `specs/0001-pixel-coordinates.md` and `0006-quad-color.md`.

pub mod quad;
pub mod vertex;
use quad::Quad;

pub struct Geometry {
    vertex_data: Vec<vertex::Vertex>,
    index_data: Vec<u32>,
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
            num_quads: 0,
        }
    }

    pub fn reset(&mut self) {
        self.vertex_data.clear();
        self.index_data.clear();
        self.num_quads = 0;
    }

    pub fn push_quad(&mut self, quad: &Quad) {
        let min_x = quad.position.x - quad.size.x * 0.5;
        let min_y = quad.position.y - quad.size.y * 0.5;
        let max_x = quad.position.x + quad.size.x * 0.5;
        let max_y = quad.position.y + quad.size.y * 0.5;

        self.vertex_data.extend(&[
            vertex::Vertex {
                position: [min_x, min_y],
                color: quad.color.to_array(),
            },
            vertex::Vertex {
                position: [max_x, min_y],
                color: quad.color.to_array(),
            },
            vertex::Vertex {
                position: [max_x, max_y],
                color: quad.color.to_array(),
            },
            vertex::Vertex {
                position: [min_x, max_y],
                color: quad.color.to_array(),
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
