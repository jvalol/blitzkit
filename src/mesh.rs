//! 3D geometry: what a mesh is made of, the shapes that ship with the engine,
//! and the transform that places one in the world.
//!
//! See `specs/0010-meshes.md`. Everything here is plain data, so it can be built
//! and checked without a GPU. Uploading it is the renderer's job.

use glam::{Mat3, Mat4, Quat, Vec3};

/// A corner of a mesh. Arrays rather than glam types, because this goes
/// straight to the GPU and `Vec3` would pad it, per spec 0007.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

impl Vertex {
    pub const SIZE: wgpu::BufferAddress = std::mem::size_of::<Self>() as wgpu::BufferAddress;
    pub const DESC: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: Self::SIZE,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2],
    };

    pub fn new(position: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> Self {
        Self {
            position,
            normal,
            uv,
        }
    }
}

/// A mesh before it reaches the GPU.
#[derive(Debug, Clone, Default)]
pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl MeshData {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u32>) -> Self {
        Self { vertices, indices }
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// The box this mesh fits inside, so a game can build a collider from what
    /// it draws rather than typing the numbers twice. See spec 0014.
    pub fn bounds(&self) -> crate::collision::Aabb {
        self.vertices
            .iter()
            .fold(crate::collision::Aabb::empty(), |bounds, vertex| {
                bounds.union_point(Vec3::from(vertex.position))
            })
    }

    /// A unit cube centered on the origin, with a normal per face rather than
    /// per corner, so its edges stay sharp. That means four vertices per face
    /// instead of eight shared corners.
    pub fn cube() -> Self {
        let faces: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
            // normal, then the two axes spanning the face
            ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
            ([0.0, 0.0, -1.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
            ([1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]),
            ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
            ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]),
            ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        ];

        let mut vertices = Vec::with_capacity(24);
        let mut indices = Vec::with_capacity(36);

        for (normal, right, up) in faces {
            let normal = Vec3::from(normal);
            let right = Vec3::from(right);
            let up = Vec3::from(up);
            let center = normal * 0.5;
            let base = vertices.len() as u32;

            for (corner, uv) in [
                (-right - up, [0.0, 1.0]),
                (right - up, [1.0, 1.0]),
                (right + up, [1.0, 0.0]),
                (-right + up, [0.0, 0.0]),
            ] {
                let position = center + corner * 0.5;
                vertices.push(Vertex::new(position.to_array(), normal.to_array(), uv));
            }

            // counter-clockwise seen from outside, which is what the back face
            // culling in spec 0009 expects
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }

        Self::new(vertices, indices)
    }

    /// A unit sphere centered on the origin, built as rings of quads. Normals
    /// point straight out, which is what makes it read as round under the
    /// lighting in spec 0012.
    pub fn sphere(segments: u32, rings: u32) -> Self {
        let segments = segments.max(3);
        let rings = rings.max(2);

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for ring in 0..=rings {
            // from the north pole down
            let phi = std::f32::consts::PI * ring as f32 / rings as f32;
            for segment in 0..=segments {
                let theta = std::f32::consts::TAU * segment as f32 / segments as f32;
                let normal = Vec3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin());

                vertices.push(Vertex::new(
                    (normal * 0.5).to_array(),
                    normal.to_array(),
                    // 1 - the segment, not the segment. Longitude runs the other
                    // way round from the outside, and a texture laid on with
                    // the segment reads mirrored: see the uv test below.
                    [
                        1.0 - segment as f32 / segments as f32,
                        ring as f32 / rings as f32,
                    ],
                ));
            }
        }

        let stride = segments + 1;
        for ring in 0..rings {
            for segment in 0..segments {
                let a = ring * stride + segment;
                let b = a + stride;

                // counter-clockwise from outside, per spec 0009. It was not:
                // these two ran the other way, so every outward face was culled
                // as a back face and what you saw was the inside of the far
                // side. A still sphere looks the same either way, and a turning
                // one does not: the far surface moves against the near one.
                indices.extend_from_slice(&[a, b + 1, b, a, a + 1, b + 1]);
            }
        }

        Self::new(vertices, indices)
    }

    /// A unit square on the xz plane, facing up. Handy as a floor.
    pub fn plane() -> Self {
        let normal = [0.0, 1.0, 0.0];
        let vertices = vec![
            Vertex::new([-0.5, 0.0, 0.5], normal, [0.0, 1.0]),
            Vertex::new([0.5, 0.0, 0.5], normal, [1.0, 1.0]),
            Vertex::new([0.5, 0.0, -0.5], normal, [1.0, 0.0]),
            Vertex::new([-0.5, 0.0, -0.5], normal, [0.0, 0.0]),
        ];

        Self::new(vertices, vec![0, 1, 2, 0, 2, 3])
    }

    /// A mesh built from a formula rather than a list of corners. `point` is
    /// called with both parameters running from 0 to 1 inclusive, and the
    /// results are joined into quads. See spec 0016.
    pub fn surface(u_steps: u32, v_steps: u32, point: impl Fn(f32, f32) -> Vec3) -> Self {
        let u_steps = u_steps.max(1);
        let v_steps = v_steps.max(1);

        // half a cell either side: far enough to measure the surface, near
        // enough to still be measuring this part of it
        let du = 0.5 / u_steps as f32;
        let dv = 0.5 / v_steps as f32;

        let mut vertices = Vec::with_capacity(((u_steps + 1) * (v_steps + 1)) as usize);
        for i in 0..=u_steps {
            let u = i as f32 / u_steps as f32;
            for j in 0..=v_steps {
                let v = j as f32 / v_steps as f32;

                // clamped at the edges, so the formula is never asked for a
                // parameter it was not given a meaning for
                let along_u = point((u + du).min(1.0), v) - point((u - du).max(0.0), v);
                let along_v = point(u, (v + dv).min(1.0)) - point(u, (v - dv).max(0.0));
                // a direction that has collapsed, as u does at a pole, leaves
                // a cross product made of rounding error rather than a normal
                let longest = along_u.length().max(along_v.length());
                let normal = if along_u.length().min(along_v.length()) < longest * 1e-4 {
                    Vec3::ZERO
                } else {
                    along_u.cross(along_v).normalize_or_zero()
                };

                vertices.push(Vertex::new(
                    point(u, v).to_array(),
                    normal.to_array(),
                    [u, v],
                ));
            }
        }

        let stride = v_steps + 1;
        let mut indices = Vec::with_capacity((u_steps * v_steps * 6) as usize);
        for i in 0..u_steps {
            for j in 0..v_steps {
                let a = i * stride + j;
                let b = a + stride;

                // counter-clockwise around u crossed into v, which is the
                // normal above, per spec 0009
                indices.extend_from_slice(&[a, b, b + 1, a, b + 1, a + 1]);
            }
        }

        Self::new(vertices, indices)
    }

    /// The same mesh with every triangle present twice, the second wound the
    /// other way with its normal negated.
    ///
    /// Back face culling keeps exactly one of each pair, so a surface with no
    /// outside is lit correctly whichever side is facing, and the two copies
    /// never argue over a pixel. See spec 0016.
    pub fn two_sided(self) -> Self {
        let count = self.vertices.len() as u32;

        let flipped: Vec<u32> = self
            .indices
            .chunks_exact(3)
            .flat_map(|t| [t[0] + count, t[2] + count, t[1] + count])
            .collect();

        let mut vertices = self.vertices;
        vertices.reserve(count as usize);
        for index in 0..count as usize {
            let mut back = vertices[index];
            back.normal = (-Vec3::from(back.normal)).to_array();
            vertices.push(back);
        }

        let mut indices = self.indices;
        indices.extend_from_slice(&flipped);

        Self::new(vertices, indices)
    }

    /// The same surface with holes cut in it, leaving ribbons along `u_lines`
    /// lines of constant u and `v_lines` lines of constant v. `thickness` is
    /// how much of the space between two lines a ribbon covers, from 0 to 1,
    /// and a ribbon is always at least the cell either side of its line.
    ///
    /// What to reach for when the inside of a shape is the interesting part.
    /// The parameters are read back out of the texture coordinates, so this
    /// works on a mesh from `surface` and on nothing else. Corners left with no
    /// triangle on them are dropped. See spec 0016.
    pub fn lattice(self, u_lines: u32, v_lines: u32, thickness: f32) -> Self {
        let thickness = thickness.clamp(0.0, 1.0);
        let on_line = |t: f32, lines: u32| {
            if lines == 0 {
                return false;
            }
            // distance to the nearest line, measured in spacings
            let scaled = t * lines as f32;
            (scaled - scaled.round()).abs() <= thickness * 0.5
        };

        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut moved_to = vec![u32::MAX; self.vertices.len()];

        for triangle in self.indices.chunks_exact(3) {
            let uv = [triangle[0], triangle[1], triangle[2]]
                .map(|index| self.vertices[index as usize].uv);

            // one corner on a line keeps the whole triangle, so a ribbon is
            // never thinner than the cells its line runs through. Asking for
            // all three instead would leave nothing at all whenever the ribbon
            // came out narrower than a cell.
            let along_u = uv.iter().any(|uv| on_line(uv[0], u_lines));
            let along_v = uv.iter().any(|uv| on_line(uv[1], v_lines));
            if !along_u && !along_v {
                continue;
            }

            for index in triangle {
                let slot = &mut moved_to[*index as usize];
                if *slot == u32::MAX {
                    *slot = vertices.len() as u32;
                    vertices.push(self.vertices[*index as usize]);
                }
                indices.push(*slot);
            }
        }

        Self::new(vertices, indices)
    }

    /// Loads the first model out of a Wavefront OBJ file. Missing normals are
    /// filled in from the face they belong to; missing texture coordinates are
    /// zero.
    pub fn from_obj(bytes: &[u8]) -> Result<Self, tobj::LoadError> {
        let mut reader = std::io::BufReader::new(bytes);
        let (models, _) = tobj::load_obj_buf(
            &mut reader,
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
            // no .mtl files: materials are spec 0011's problem
            |_| Ok((Vec::new(), Default::default())),
        )?;

        let model = models
            .into_iter()
            .next()
            .ok_or(tobj::LoadError::InvalidObjectName)?;
        let mesh = model.mesh;
        let count = mesh.positions.len() / 3;

        let mut vertices = Vec::with_capacity(count);
        for i in 0..count {
            let position = [
                mesh.positions[i * 3],
                mesh.positions[i * 3 + 1],
                mesh.positions[i * 3 + 2],
            ];
            let normal = if mesh.normals.len() >= (i + 1) * 3 {
                [
                    mesh.normals[i * 3],
                    mesh.normals[i * 3 + 1],
                    mesh.normals[i * 3 + 2],
                ]
            } else {
                [0.0, 0.0, 0.0]
            };
            let uv = if mesh.texcoords.len() >= (i + 1) * 2 {
                [mesh.texcoords[i * 2], mesh.texcoords[i * 2 + 1]]
            } else {
                [0.0, 0.0]
            };
            vertices.push(Vertex::new(position, normal, uv));
        }

        let mut data = Self::new(vertices, mesh.indices);
        if mesh.normals.is_empty() {
            data.compute_normals();
        }

        Ok(data)
    }

    /// Adds another mesh's triangles to this one, keeping both.
    ///
    /// Nothing is welded: corners that two meshes happen to share stay
    /// separate, which is what a shape built out of separate patches wants.
    pub fn extend(&mut self, other: &Self) {
        let offset = self.vertices.len() as u32;

        self.vertices.extend_from_slice(&other.vertices);
        self.indices
            .extend(other.indices.iter().map(|index| index + offset));
    }

    /// Gives a normal to any vertex that has none, averaged from the faces it
    /// belongs to.
    ///
    /// `surface` leaves a zero normal where the surface has collapsed to a
    /// point and there was nothing to measure. The faces around such a vertex
    /// usually still know which way they face, which is enough to light it.
    /// Where they have no area either, because a whole patch edge collapsed,
    /// the vertex borrows from the corners it shares a triangle with. A vertex
    /// no triangle uses keeps its zero normal, because nothing knows better.
    pub fn fill_missing_normals(&mut self) {
        let missing: Vec<usize> = self
            .vertices
            .iter()
            .enumerate()
            .filter(|(_, vertex)| Vec3::from(vertex.normal) == Vec3::ZERO)
            .map(|(index, _)| index)
            .collect();

        if missing.is_empty() {
            return;
        }

        let mut found = vec![Vec3::ZERO; self.vertices.len()];
        for triangle in self.indices.chunks_exact(3) {
            let [a, b, c] = [
                triangle[0] as usize,
                triangle[1] as usize,
                triangle[2] as usize,
            ];
            let edge1 =
                Vec3::from(self.vertices[b].position) - Vec3::from(self.vertices[a].position);
            let edge2 =
                Vec3::from(self.vertices[c].position) - Vec3::from(self.vertices[a].position);
            let face = edge1.cross(edge2);

            for index in [a, b, c] {
                found[index] += face;
            }
        }

        let mut still_missing = Vec::new();
        for index in missing {
            let normal = found[index].normalize_or_zero();
            self.vertices[index].normal = normal.to_array();
            if normal == Vec3::ZERO {
                still_missing.push(index);
            }
        }

        // a corner where the whole patch edge has collapsed has no face with
        // any area to it, so it borrows from the corners it shares a triangle
        // with, which are the nearest thing that knows
        for index in still_missing {
            let mut borrowed = Vec3::ZERO;
            for triangle in self.indices.chunks_exact(3) {
                if !triangle.contains(&(index as u32)) {
                    continue;
                }
                for other in triangle {
                    borrowed += Vec3::from(self.vertices[*other as usize].normal);
                }
            }

            self.vertices[index].normal = borrowed.normalize_or_zero().to_array();
        }
    }

    /// Gives every vertex the normal of the faces it belongs to, averaged.
    pub fn compute_normals(&mut self) {
        let mut normals = vec![Vec3::ZERO; self.vertices.len()];

        for triangle in self.indices.chunks_exact(3) {
            let [a, b, c] = [
                triangle[0] as usize,
                triangle[1] as usize,
                triangle[2] as usize,
            ];
            let edge1 =
                Vec3::from(self.vertices[b].position) - Vec3::from(self.vertices[a].position);
            let edge2 =
                Vec3::from(self.vertices[c].position) - Vec3::from(self.vertices[a].position);
            let face = edge1.cross(edge2);

            for index in [a, b, c] {
                normals[index] += face;
            }
        }

        for (vertex, normal) in self.vertices.iter_mut().zip(normals) {
            vertex.normal = normal.normalize_or_zero().to_array();
        }
    }
}

/// Where a mesh sits in the world.
#[derive(Debug, Copy, Clone)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub fn new() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    pub fn at(position: Vec3) -> Self {
        Self {
            position,
            ..Self::new()
        }
    }

    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    /// Scale, then rotate, then move: the order that keeps scaling along the
    /// object's own axes.
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }

    /// Normals need the inverse transpose, or a squashed object lights as if it
    /// were not squashed.
    pub fn normal_matrix(&self) -> Mat3 {
        Mat3::from_mat4(self.matrix()).inverse().transpose()
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sphere as a formula: u goes around, v goes from pole to pole, and u
    /// crossed into v points away from the center.
    fn unit_sphere(u: f32, v: f32) -> Vec3 {
        let theta = u * std::f32::consts::TAU;
        let phi = v * std::f32::consts::PI;

        Vec3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin())
    }

    #[test]
    fn extending_a_mesh_keeps_both() {
        let mut first = MeshData::plane();
        let second = MeshData::cube();
        let (corners, triangles) = (first.vertices.len(), first.triangle_count());

        first.extend(&second);

        assert_eq!(first.vertices.len(), corners + second.vertices.len());
        assert_eq!(first.triangle_count(), triangles + second.triangle_count());

        // the second mesh's corners moved, so its triangles point at the new
        // places rather than back at the first mesh's
        let moved = &first.indices[triangles * 3..];
        assert_eq!(moved[0], second.indices[0] + corners as u32);
        assert!(moved.iter().all(|index| *index as usize >= corners));
    }

    #[test]
    fn a_missing_normal_is_borrowed() {
        let mut data = MeshData::plane();
        data.vertices[0].normal = [0.0, 0.0, 0.0];

        data.fill_missing_normals();

        // the plane faces up, so the corner with nothing gets the same
        assert!(
            (Vec3::from(data.vertices[0].normal) - Vec3::Y).length() < 1e-5,
            "borrowed {:?}",
            data.vertices[0].normal
        );
        // the ones that had a normal keep it
        assert_eq!(Vec3::from(data.vertices[1].normal), Vec3::Y);
    }

    #[test]
    fn a_normal_with_nothing_to_borrow_from_stays_missing() {
        // a lone corner no triangle uses: nothing knows which way it faces
        let mut data = MeshData::plane();
        data.vertices
            .push(Vertex::new([9.0, 9.0, 9.0], [0.0, 0.0, 0.0], [0.0, 0.0]));

        data.fill_missing_normals();

        assert_eq!(data.vertices.last().unwrap().normal, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn a_surface_is_a_grid_of_quads() {
        let data = MeshData::surface(4, 3, unit_sphere);

        // one more vertex than cells along each side, for the far edge
        assert_eq!(data.vertices.len(), 5 * 4);
        assert_eq!(data.triangle_count(), 4 * 3 * 2);
    }

    #[test]
    fn a_surface_is_never_empty() {
        let data = MeshData::surface(0, 0, unit_sphere);

        assert_eq!(data.vertices.len(), 4);
        assert_eq!(data.triangle_count(), 2);
    }

    #[test]
    fn surface_normals_come_from_the_formula() {
        let data = MeshData::surface(24, 16, unit_sphere);

        for vertex in data.vertices.iter() {
            let position = Vec3::from(vertex.position);
            let normal = Vec3::from(vertex.normal);

            // the poles have no surface directions to cross, so no normal
            if normal == Vec3::ZERO {
                assert!(position.y.abs() > 0.99, "a zero normal away from a pole");
                continue;
            }

            assert!(
                normal.dot(position.normalize()) > 0.99,
                "the normal at {:?} points {:?}, not outward",
                position,
                normal
            );
        }
    }

    #[test]
    fn a_surface_is_parameterized_from_zero_to_one() {
        let seen = std::cell::RefCell::new(Vec::new());
        let data = MeshData::surface(2, 2, |u, v| {
            seen.borrow_mut().push((u, v));
            Vec3::new(u, 0.0, v)
        });

        let seen = seen.into_inner();
        let (low, high) = seen.iter().fold((1.0f32, 0.0f32), |(low, high), (u, v)| {
            (low.min(u.min(*v)), high.max(u.max(*v)))
        });
        assert_eq!((low, high), (0.0, 1.0), "the formula saw {:?}", (low, high));

        // the corners of the parameter square land in the corners of the mesh
        let uvs: Vec<[f32; 2]> = data.vertices.iter().map(|vertex| vertex.uv).collect();
        assert!(uvs.contains(&[0.0, 0.0]));
        assert!(uvs.contains(&[1.0, 1.0]));
        assert!(uvs.contains(&[0.5, 0.5]));
    }

    #[test]
    fn two_sided_adds_the_other_side() {
        let one = MeshData::surface(3, 3, unit_sphere);
        let both = one.clone().two_sided();

        assert_eq!(both.vertices.len(), one.vertices.len() * 2);
        assert_eq!(both.triangle_count(), one.triangle_count() * 2);

        let count = one.vertices.len() as u32;
        let front = &one.indices[0..3];
        let back = &both.indices[one.indices.len()..one.indices.len() + 3];

        // the same corners, in the other order
        assert_eq!(back, [front[0] + count, front[2] + count, front[1] + count]);
    }

    #[test]
    fn the_other_side_faces_the_other_way() {
        let one = MeshData::surface(3, 3, unit_sphere);
        let both = one.clone().two_sided();

        for (index, vertex) in one.vertices.iter().enumerate() {
            let back = both.vertices[one.vertices.len() + index];

            assert_eq!(back.position, vertex.position);
            assert_eq!(back.uv, vertex.uv);
            assert_eq!(Vec3::from(back.normal), -Vec3::from(vertex.normal));
        }
    }

    #[test]
    fn a_lattice_keeps_only_the_ribbons() {
        const LINES: u32 = 8;
        const THICKNESS: f32 = 0.05;

        // sixteen cells between one line and the next, so a ribbon is a small
        // part of the space it sits in
        let whole = MeshData::surface(128, 128, unit_sphere);
        let wire = whole.clone().lattice(LINES, LINES, THICKNESS);

        assert!(wire.triangle_count() > 0, "nothing survived");
        assert!(
            wire.triangle_count() < whole.triangle_count() / 2,
            "{} of {} triangles kept, which is not a lattice",
            wire.triangle_count(),
            whole.triangle_count()
        );

        // every triangle that is left has a corner on a line
        let near = |t: f32| {
            let scaled = t * LINES as f32;
            (scaled - scaled.round()).abs() <= THICKNESS * 0.5
        };
        for triangle in wire.indices.chunks_exact(3) {
            let uv = [triangle[0], triangle[1], triangle[2]]
                .map(|index| wire.vertices[index as usize].uv);

            assert!(
                uv.iter().any(|uv| near(uv[0])) || uv.iter().any(|uv| near(uv[1])),
                "a triangle at {:?} is off every line",
                uv
            );
        }
    }

    #[test]
    fn a_ribbon_thinner_than_a_cell_is_still_a_ribbon() {
        // the trap: ask for a ribbon narrower than the grid it is cut from and
        // a rule that wanted every corner on the line would leave nothing
        let wire = MeshData::surface(192, 96, unit_sphere).lattice(24, 12, 0.01);

        assert!(wire.triangle_count() > 0, "the whole lattice was cut away");
    }

    #[test]
    fn a_lattice_keeps_no_corner_it_does_not_use() {
        let wire = MeshData::surface(32, 32, unit_sphere).lattice(4, 4, 0.25);
        let used: std::collections::HashSet<u32> = wire.indices.iter().copied().collect();

        assert_eq!(
            used.len(),
            wire.vertices.len(),
            "orphaned corners are left over"
        );
    }

    #[test]
    fn a_lattice_of_no_lines_is_nothing() {
        let wire = MeshData::surface(16, 16, unit_sphere).lattice(0, 0, 1.0);

        assert_eq!(wire.triangle_count(), 0);
        assert!(wire.vertices.is_empty());
    }

    /// A triangle's position and uv together, as the two directions the
    /// texture runs in on the surface, plus the way the winding says it faces.
    ///
    /// Returns nothing for a triangle with no area or no uv area, which is
    /// what the sphere's poles are.
    fn surface(mesh: &MeshData, triangle: &[u32]) -> Option<(Vec3, Vec3, Vec3, Vec3)> {
        let at: Vec<&Vertex> = triangle
            .iter()
            .map(|index| &mesh.vertices[*index as usize])
            .collect();
        let point: Vec<Vec3> = at.iter().map(|v| Vec3::from(v.position)).collect();
        let uv: Vec<[f32; 2]> = at.iter().map(|v| v.uv).collect();

        let one = point[1] - point[0];
        let other = point[2] - point[0];
        let wound = one.cross(other);
        if wound.length() < 1e-6 {
            return None;
        }

        let du = [uv[1][0] - uv[0][0], uv[1][1] - uv[0][1]];
        let dv = [uv[2][0] - uv[0][0], uv[2][1] - uv[0][1]];
        let det = du[0] * dv[1] - dv[0] * du[1];
        if det.abs() < 1e-9 {
            return None;
        }

        let along_u = (one * dv[1] - other * du[1]) / det;
        let along_v = (other * du[0] - one * dv[0]) / det;
        let out: Vec3 = at.iter().map(|v| Vec3::from(v.normal)).sum::<Vec3>() / 3.0;

        Some((along_u, along_v, wound.normalize(), out.normalize()))
    }

    #[test]
    fn every_mesh_is_wound_the_way_it_faces() {
        // The sphere was not. Its two triangles ran clockwise seen from
        // outside, so the renderer culled every outward face and drew the
        // inside of the far side instead. Still, that looks like a sphere.
        // Turning, the far surface moves against the near one, so a rolling
        // ball rolled backwards.
        for (name, mesh) in [
            ("cube", MeshData::cube()),
            ("plane", MeshData::plane()),
            ("sphere", MeshData::sphere(16, 8)),
        ] {
            for triangle in mesh.indices.chunks(3) {
                let Some((_, _, wound, out)) = surface(&mesh, triangle) else {
                    continue;
                };
                assert!(
                    wound.dot(out) > 0.5,
                    "{} is wound inside out: {}",
                    name,
                    wound
                );
            }
        }
    }

    #[test]
    fn no_mesh_wears_its_texture_mirrored() {
        // u across and v down is how an image is read, and laying it on a
        // surface that way puts u cross v into the surface. Coming out means
        // the image is on backwards, which is what the sphere's longitude did:
        // letters would have read in a mirror, and the checker on marble's
        // ball turned the wrong way.
        for (name, mesh) in [
            ("cube", MeshData::cube()),
            ("plane", MeshData::plane()),
            ("sphere", MeshData::sphere(16, 8)),
        ] {
            for triangle in mesh.indices.chunks(3) {
                let Some((along_u, along_v, _, out)) = surface(&mesh, triangle) else {
                    continue;
                };
                let into = along_u.cross(along_v).dot(out);
                assert!(into < 0.0, "{} wears its texture mirrored: {}", name, into);
            }
        }
    }

    #[test]
    fn vertex_has_position_normal_and_uv() {
        let vertex = Vertex::new([1.0, 2.0, 3.0], [0.0, 1.0, 0.0], [0.5, 0.25]);

        assert_eq!(vertex.position, [1.0, 2.0, 3.0]);
        assert_eq!(vertex.normal, [0.0, 1.0, 0.0]);
        assert_eq!(vertex.uv, [0.5, 0.25]);

        // the same padding trap as the 2D vertex, per spec 0007
        assert_eq!(std::mem::size_of::<Vertex>(), 32);
        assert_eq!(std::mem::offset_of!(Vertex, position), 0);
        assert_eq!(std::mem::offset_of!(Vertex, normal), 12);
        assert_eq!(std::mem::offset_of!(Vertex, uv), 24);

        let attributes = Vertex::DESC.attributes;
        assert_eq!(attributes[1].offset, 12);
        assert_eq!(attributes[2].offset, 24);
        assert_eq!(Vertex::DESC.array_stride, Vertex::SIZE);
    }

    #[test]
    fn the_cube_is_a_cube() {
        let cube = MeshData::cube();

        // four vertices per face, so the faces can have their own normals
        assert_eq!(cube.vertices.len(), 24);
        assert_eq!(cube.triangle_count(), 12);

        let mut corners: Vec<[i32; 3]> = cube
            .vertices
            .iter()
            .map(|v| {
                [
                    (v.position[0] * 2.0) as i32,
                    (v.position[1] * 2.0) as i32,
                    (v.position[2] * 2.0) as i32,
                ]
            })
            .collect();
        corners.sort_unstable();
        corners.dedup();
        assert_eq!(corners.len(), 8, "a cube has eight corners");

        let mut normals: Vec<[i32; 3]> = cube
            .vertices
            .iter()
            .map(|v| [v.normal[0] as i32, v.normal[1] as i32, v.normal[2] as i32])
            .collect();
        normals.sort_unstable();
        normals.dedup();
        assert_eq!(normals.len(), 6, "one normal per face");

        // every normal points away from the middle, so the outside faces out
        for vertex in cube.vertices.iter() {
            let position = Vec3::from(vertex.position);
            let normal = Vec3::from(vertex.normal);
            assert!(position.dot(normal) > 0.0);
        }
    }

    #[test]
    fn a_mesh_knows_its_bounds() {
        let bounds = MeshData::cube().bounds();

        assert!(
            (bounds.min - Vec3::splat(-0.5)).length() < 1e-5,
            "{:?}",
            bounds
        );
        assert!(
            (bounds.max - Vec3::splat(0.5)).length() < 1e-5,
            "{:?}",
            bounds
        );

        // the plane is flat, so its box has no height
        let plane = MeshData::plane().bounds();
        assert_eq!(plane.size().y, 0.0);
        assert_eq!(plane.size().x, 1.0);
    }

    #[test]
    fn the_sphere_is_round() {
        let sphere = MeshData::sphere(16, 8);

        for vertex in sphere.vertices.iter() {
            let position = Vec3::from(vertex.position);
            let normal = Vec3::from(vertex.normal);

            // every point is the same distance from the middle
            assert!((position.length() - 0.5).abs() < 1e-4, "{:?}", vertex);
            // and its normal points straight out from there
            assert!((normal.length() - 1.0).abs() < 1e-4, "{:?}", vertex);
            assert!(position.normalize().dot(normal) > 0.999, "{:?}", vertex);
        }

        assert_eq!(sphere.triangle_count(), 16 * 8 * 2);
        assert!((sphere.bounds().size() - Vec3::ONE).length() < 1e-4);
    }

    #[test]
    fn the_plane_faces_up() {
        let plane = MeshData::plane();

        assert_eq!(plane.triangle_count(), 2);
        assert!(plane.vertices.iter().all(|v| v.normal == [0.0, 1.0, 0.0]));
        assert!(plane.vertices.iter().all(|v| v.position[1] == 0.0));
    }

    #[test]
    fn transform_applies_scale_rotation_then_position() {
        let transform = Transform::at(Vec3::new(10.0, 0.0, 0.0))
            .with_scale(Vec3::splat(2.0))
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));

        let moved = transform.matrix() * Vec3::new(1.0, 0.0, 0.0).extend(1.0);

        // scaled to (2,0,0), turned a quarter turn to (0,2,0), then moved
        assert!((moved.x - 10.0).abs() < 1e-5, "x was {}", moved.x);
        assert!((moved.y - 2.0).abs() < 1e-5, "y was {}", moved.y);
    }

    #[test]
    fn normal_matrix_survives_non_uniform_scale() {
        // squashed flat in y, which would tilt a normal if the model matrix
        // were used directly
        let transform = Transform::new().with_scale(Vec3::new(1.0, 0.1, 1.0));
        let slope = Vec3::new(0.0, 1.0, 1.0).normalize();

        let wrong = (Mat3::from_mat4(transform.matrix()) * slope).normalize();
        let right = (transform.normal_matrix() * slope).normalize();

        assert!(
            right.y > wrong.y,
            "the normal should stay steep, not flatten"
        );
        // and a straight up normal is unchanged either way
        let up = transform.normal_matrix() * Vec3::Y;
        assert!((up.normalize() - Vec3::Y).length() < 1e-5);
    }

    #[test]
    fn loads_an_obj_file() {
        // one square, two triangles, with normals and texture coordinates
        let obj = b"v -1 0 1\nv 1 0 1\nv 1 0 -1\nv -1 0 -1\n\
vn 0 1 0\n\
vt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\n\
f 1/1/1 2/2/1 3/3/1\nf 1/1/1 3/3/1 4/4/1\n";

        let mesh = MeshData::from_obj(obj).expect("the obj is valid");

        assert_eq!(mesh.triangle_count(), 2);
        assert!(mesh.vertices.iter().all(|v| v.normal == [0.0, 1.0, 0.0]));
        assert!(mesh.vertices.iter().all(|v| v.position[1] == 0.0));
    }

    #[test]
    fn computes_missing_normals() {
        let obj = b"v -1 0 1\nv 1 0 1\nv 1 0 -1\nf 1 2 3\n";

        let mesh = MeshData::from_obj(obj).expect("the obj is valid");

        // the face lies flat, so its normal points straight up or down
        for vertex in mesh.vertices.iter() {
            assert!((vertex.normal[1].abs() - 1.0).abs() < 1e-5, "{:?}", vertex);
        }
    }
}
