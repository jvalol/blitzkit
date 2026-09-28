//! wgpu: the device, the pipelines, and everything drawn through them.
//!
//! Two of them do the drawing, one for quads and one for meshes, with shadow
//! passes ahead of both. What a game pushes to be drawn in 3D is `scene`, the
//! depth buffer and its settings are `depth`, and text goes through
//! `render_text`.
//!
//! A game never sees a wgpu type. Anything uploaded comes back as a handle.

pub mod depth;
pub mod render_text;
pub mod scene;

use crate::camera::Camera;
use crate::collision::Aabb;
use crate::geometry::vertex::*;
use crate::geometry::Geometry;
use crate::lighting::Light;
use crate::mesh::MeshData;
use crate::shadow;

/// How many layers the lamp shadow stack has: six faces for each lamp that may
/// cast. See spec 0022.
const POINT_SHADOW_LAYERS: usize = crate::lighting::MAX_SHADOWING_POINT_LIGHTS * 6;

/// How many different numbers a shadow pass might be told it is for. The sun and
/// the spots need one each; a lamp's faces need one per layer, and there are
/// more of those.
const WHICH_INDEXES: usize = if POINT_SHADOW_LAYERS > crate::lighting::MAX_SPOT_LIGHTS + 1 {
    POINT_SHADOW_LAYERS
} else {
    crate::lighting::MAX_SPOT_LIGHTS + 1
};
use crate::texture::TextureData;
use render_text::*;
use scene::{Batch, Instance, MeshId, Scene, TextureId};

use std::sync::Arc;

use wgpu::util::DeviceExt;
use wgpu_text::glyph_brush::ab_glyph::FontRef;
use wgpu_text::glyph_brush::{HorizontalAlign, Layout, Section, Text};
use wgpu_text::{BrushBuilder, TextBrush};
use winit::window::Window;

const FONT_BYTES: &[u8] = include_bytes!("../../res/fonts/PressStart2P-Regular.ttf");

pub struct Renderer {
    /// Kept so a game can lock or hide the cursor, per spec 0013. The surface
    /// owns the window too, but does not hand it back.
    window: Arc<Window>,
    cursor_locked: bool,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    screen_buffer: wgpu::Buffer,
    screen_bind_group: wgpu::BindGroup,
    camera: Camera,
    camera_buffer: wgpu::Buffer,
    /// Bound by the 3D pipeline, which arrives with meshes in spec 0010.
    #[allow(dead_code)]
    camera_bind_group: wgpu::BindGroup,
    /// Kept so that pipeline can be built against the same layout.
    #[allow(dead_code)]
    camera_bind_group_layout: wgpu::BindGroupLayout,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    text_brush: TextBrush<FontRef<'static>>,
    /// Built here and resized with the surface. The pass that attaches it
    /// arrives with meshes in spec 0010; quads and text never use it.
    depth: depth::DepthTexture,
    mesh_pipeline: wgpu::RenderPipeline,
    meshes: Vec<GpuMesh>,
    textures: Vec<wgpu::BindGroup>,
    shadow_pipeline: wgpu::RenderPipeline,
    /// A second one for a lamp's six faces: it has a fragment stage, because
    /// what goes in those maps is distance rather than depth. See spec 0022.
    point_shadow_pipeline: wgpu::RenderPipeline,
    /// One per shadow pass, holding which light that pass is for.
    which_bind_groups: Vec<wgpu::BindGroup>,
    /// The far side of translucent geometry, then the near side.
    translucent_pipelines: [wgpu::RenderPipeline; 2],
    shadow_view: wgpu::TextureView,
    /// One view per layer of the spot shadow stack, to render into. See spec
    /// 0021.
    spot_shadow_layers: Vec<wgpu::TextureView>,
    /// One view per layer of the lamp shadow stack, six to a casting lamp. See
    /// spec 0022.
    point_shadow_layers: Vec<wgpu::TextureView>,
    /// The matrix and the lamp for each of those layers, rewritten every frame
    /// because lamps move.
    point_faces_buffer: wgpu::Buffer,
    point_faces_bind_group: wgpu::BindGroup,
    shadow_bind_group: wgpu::BindGroup,
    /// What the light's view covers. A game sets it to fit its world.
    scene_bounds: Aabb,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    /// Grown as needed, like the quad buffers.
    instance_buffer: wgpu::Buffer,
}

/// A mesh living on the GPU. Built once, drawn many times.
struct GpuMesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
}

impl Renderer {
    pub fn width(&self) -> f32 {
        self.config.width as f32
    }

    pub fn height(&self) -> f32 {
        self.config.height as f32
    }

    pub async fn new(window: Arc<Window>, instance_desc: wgpu::InstanceDescriptor) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(instance_desc);
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Device"),
                ..Default::default()
            })
            .await
            .unwrap();

        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .unwrap_or(capabilities.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            format,
            present_mode: wgpu::PresentMode::Fifo,
            ..surface
                .get_default_config(&adapter, size.width.max(1), size.height.max(1))
                .unwrap()
        };
        surface.configure(&device, &config);

        let screen_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Screen Uniform"),
            contents: bytemuck::cast_slice(&screen_size(&config)),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let screen_bind_group_layout =
            uniform_bind_group_layout(&device, "Screen", wgpu::ShaderStages::VERTEX);
        let screen_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Screen Bind Group"),
            layout: &screen_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: screen_buffer.as_entire_binding(),
            }],
        });

        let mut camera = Camera::new();
        camera.set_viewport(config.width as f32, config.height as f32);
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene Uniform"),
            contents: bytemuck::cast_slice(&[SceneUniform::new(
                &camera,
                &Light::new(),
                &[],
                &[],
                &shadow::default_bounds(),
            )]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group_layout = uniform_bind_group_layout(
            &device,
            "Scene",
            wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
        );
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let pipeline = create_render_pipeline(&device, config.format, &screen_bind_group_layout);

        let vertex_buffer = create_buffer(&device, 0, wgpu::BufferUsages::VERTEX);
        let index_buffer = create_buffer(&device, 0, wgpu::BufferUsages::INDEX);

        let text_brush = BrushBuilder::using_font_bytes(FONT_BYTES).unwrap().build(
            &device,
            config.width,
            config.height,
            config.format,
        );

        let depth = depth::DepthTexture::new(&device, config.width, config.height);
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        // linear and repeating, with mips, per spec 0011
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Texture Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Shadow Map"),
            size: wgpu::Extent3d {
                width: shadow::MAP_SIZE,
                height: shadow::MAP_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: shadow::FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_view = shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // one layer per spot, per spec 0021. A separate texture from the sun's
        // because it is a different size and a different kind of projection.
        let spot_shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Spot Shadow Maps"),
            size: wgpu::Extent3d {
                width: shadow::SPOT_MAP_SIZE,
                height: shadow::SPOT_MAP_SIZE,
                depth_or_array_layers: crate::lighting::MAX_SPOT_LIGHTS as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: shadow::FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        // the whole stack for sampling, and one view per layer to render into
        let spot_shadow_view = spot_shadow_texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let spot_shadow_layers: Vec<wgpu::TextureView> = (0..crate::lighting::MAX_SPOT_LIGHTS)
            .map(|layer| {
                spot_shadow_texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some("Spot Shadow Layer"),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_array_layer: layer as u32,
                    array_layer_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();
        // six layers per casting lamp, per spec 0022. Smaller again than the
        // spots': six of these is a lot of map for one light.
        let point_shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Point Shadow Maps"),
            size: wgpu::Extent3d {
                width: shadow::POINT_MAP_SIZE,
                height: shadow::POINT_MAP_SIZE,
                depth_or_array_layers: POINT_SHADOW_LAYERS as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: shadow::POINT_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let point_shadow_view = point_shadow_texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let point_shadow_layers: Vec<wgpu::TextureView> = (0..POINT_SHADOW_LAYERS)
            .map(|layer| {
                point_shadow_texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some("Point Shadow Layer"),
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_array_layer: layer as u32,
                    array_layer_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();
        // a comparison sampler returns how much of the sample passed the depth
        // test rather than a depth, which is what makes the edges soft
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Shadow Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let shadow_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Shadow Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });
        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Shadow Bind Group"),
            layout: &shadow_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&spot_shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&point_shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
            ],
        });
        // one tiny uniform per shadow pass saying which light it belongs to,
        // per spec 0021. A bind group for one number, because the alternative
        // is immediate data and that is a capability not every backend has.
        // the fragment stage reads it too, because a lamp's pass needs to know
        // which lamp it is measuring the distance from
        let which_layout = uniform_bind_group_layout(
            &device,
            "Shadow Light Index",
            wgpu::ShaderStages::VERTEX_FRAGMENT,
        );
        let which_bind_groups: Vec<wgpu::BindGroup> = (0..WHICH_INDEXES)
            .map(|index| {
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Shadow Light Index"),
                    contents: bytemuck::cast_slice(&[index as u32, 0, 0, 0]),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Shadow Light Index"),
                    layout: &which_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }],
                })
            })
            .collect();

        // one matrix and one lamp per layer, indexed by the same number the
        // `which` uniform carries
        let point_faces_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Point Shadow Faces"),
            size: std::mem::size_of::<[GpuPointFace; POINT_SHADOW_LAYERS]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let point_faces_layout = uniform_bind_group_layout(
            &device,
            "Point Shadow Faces",
            wgpu::ShaderStages::VERTEX_FRAGMENT,
        );
        let point_faces_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Point Shadow Faces"),
            layout: &point_faces_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: point_faces_buffer.as_entire_binding(),
            }],
        });

        let shadow_pipeline =
            create_shadow_pipeline(&device, &camera_bind_group_layout, &which_layout);
        let point_shadow_pipeline = create_point_shadow_pipeline(
            &device,
            &camera_bind_group_layout,
            &which_layout,
            &point_faces_layout,
        );
        let mesh_pipeline = create_mesh_pipeline(
            &device,
            config.format,
            &camera_bind_group_layout,
            &texture_bind_group_layout,
            &shadow_bind_group_layout,
            depth::mesh_primitive_state(),
            depth::state(),
        );
        // the same shader twice more, for the far side of a translucent shape
        // and then its near side, per spec 0018
        let translucent_pipelines = [wgpu::Face::Front, wgpu::Face::Back].map(|cull| {
            create_mesh_pipeline(
                &device,
                config.format,
                &camera_bind_group_layout,
                &texture_bind_group_layout,
                &shadow_bind_group_layout,
                depth::translucent_primitive_state(cull),
                depth::translucent_state(),
            )
        });
        let instance_buffer = create_buffer(&device, 0, wgpu::BufferUsages::VERTEX);

        let mut renderer = Self {
            window,
            cursor_locked: false,
            surface,
            device,
            queue,
            config,
            pipeline,
            screen_buffer,
            screen_bind_group,
            camera,
            camera_buffer,
            camera_bind_group,
            camera_bind_group_layout,
            vertex_buffer,
            index_buffer,
            text_brush,
            depth,
            mesh_pipeline,
            translucent_pipelines,
            meshes: Vec::new(),
            textures: Vec::new(),
            shadow_pipeline,
            point_shadow_pipeline,
            which_bind_groups,
            shadow_view,
            spot_shadow_layers,
            point_shadow_layers,
            point_faces_buffer,
            point_faces_bind_group,
            shadow_bind_group,
            scene_bounds: shadow::default_bounds(),
            texture_bind_group_layout,
            sampler,
            instance_buffer,
        };

        // TextureId::WHITE, what an untextured mesh is drawn with
        renderer.add_texture(&TextureData::white());
        renderer
    }

    /// Uploads an image and its mip chain, and hands back the handle a game
    /// draws with.
    pub fn add_texture(&mut self, data: &TextureData) -> TextureId {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Texture"),
            size: wgpu::Extent3d {
                width: data.width(),
                height: data.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: data.mip_level_count(),
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: crate::texture::FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        for (level, mip) in data.levels.iter().enumerate() {
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: level as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &mip.pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(mip.width * 4),
                    rows_per_image: Some(mip.height),
                },
                wgpu::Extent3d {
                    width: mip.width,
                    height: mip.height,
                    depth_or_array_layers: 1,
                },
            );
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Bind Group"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        self.textures.push(bind_group);
        TextureId(self.textures.len() - 1)
    }

    /// Uploads a mesh once, and hands back the handle a game draws it with.
    pub fn add_mesh(&mut self, data: &MeshData) -> MeshId {
        // an empty mesh is a legal thing to build, a lattice of no lines for
        // one, and wgpu refuses to make a buffer with nothing in it. It gets
        // placeholder buffers and an index count of zero, which the passes skip.
        let vertices = mesh_buffer(
            &self.device,
            "Mesh Vertices",
            bytemuck::cast_slice(&data.vertices),
            wgpu::BufferUsages::VERTEX,
        );
        let indices = mesh_buffer(
            &self.device,
            "Mesh Indices",
            bytemuck::cast_slice(&data.indices),
            wgpu::BufferUsages::INDEX,
        );

        self.meshes.push(GpuMesh {
            vertices,
            indices,
            index_count: data.indices.len() as u32,
        });

        MeshId(self.meshes.len() - 1)
    }

    /// What the shadow map covers. A world bigger than this gets no shadows
    /// outside it, per spec 0015; smaller bounds give sharper shadows.
    pub fn set_scene_bounds(&mut self, bounds: Aabb) {
        self.scene_bounds = bounds;
    }

    pub fn scene_bounds(&self) -> Aabb {
        self.scene_bounds
    }

    /// Hides the pointer and pins it, so turning does not stop at the screen
    /// edge. If the platform refuses, this says so and leaves the cursor
    /// visible: raw mouse motion arrives either way, so looking around still
    /// works.
    pub fn set_cursor_locked(&mut self, locked: bool) -> bool {
        let mode = if locked {
            winit::window::CursorGrabMode::Locked
        } else {
            winit::window::CursorGrabMode::None
        };

        let grabbed = match self.window.set_cursor_grab(mode) {
            Ok(()) => true,
            Err(e) if locked => {
                // some platforms only confine the cursor, which is close enough
                match self
                    .window
                    .set_cursor_grab(winit::window::CursorGrabMode::Confined)
                {
                    Ok(()) => true,
                    Err(_) => {
                        log::warn!("could not lock the cursor: {}", e);
                        false
                    }
                }
            }
            Err(_) => false,
        };

        self.window.set_cursor_visible(!locked || !grabbed);
        self.cursor_locked = locked && grabbed;
        self.cursor_locked
    }

    pub fn cursor_locked(&self) -> bool {
        self.cursor_locked
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    /// Games move the camera through this. It takes effect on the next frame.
    pub fn set_camera(&mut self, camera: Camera) {
        self.camera = camera;
        self.camera
            .set_viewport(self.config.width as f32, self.config.height as f32);
    }

    pub(crate) fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        // A minimized window reports a zero size, which a surface cannot be configured with.
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
        self.queue.write_buffer(
            &self.screen_buffer,
            0,
            bytemuck::cast_slice(&screen_size(&self.config)),
        );
        self.depth = depth::DepthTexture::new(&self.device, self.config.width, self.config.height);
        self.camera.set_viewport(self.width(), self.height());
        self.text_brush
            .resize_view(self.width(), self.height(), &self.queue);
    }

    pub fn render(&mut self, scene: &Scene, geometry: &Geometry, text_renderer: &TextRenderer) {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                self.surface.configure(&self.device, &self.config);
                frame
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => return,
            other => {
                log::error!("Failed to acquire surface texture: {:?}", other);
                return;
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[SceneUniform::new(
                &self.camera,
                &scene.light,
                scene.point_lights(),
                scene.spot_lights(),
                &self.scene_bounds,
            )]),
        );
        self.queue.write_buffer(
            &self.point_faces_buffer,
            0,
            bytemuck::cast_slice(&point_faces(scene.point_lights())),
        );
        self.upload_geometry(geometry);

        let sections: Vec<Section> = text_renderer
            .render_texts
            .iter()
            .map(text_section)
            .collect();
        if let Err(e) = self.text_brush.queue(&self.device, &self.queue, sections) {
            log::error!("Failed to queue text: {}", e);
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Renderer Encoder"),
            });

        let (opaque, translucent) = self.upload_instances(scene);
        // the light sees translucent things as solid: a shadow map holds a
        // depth and has nowhere to put an alpha
        let batches: Vec<Batch> = opaque.iter().chain(translucent.iter()).copied().collect();

        // what the light can see, first of all. Skipped when nothing is drawn in
        // 3D: the instance buffer is empty then, and slicing an empty buffer is
        // a panic in wgpu.
        if !batches.is_empty() {
            // the sun first, then one pass for each spot, per spec 0021. Every
            // spot's map is cleared whether or not it is in use, so a spot that
            // was switched off this frame cannot leave last frame's shadow
            // behind on its layer.
            let spots = scene
                .spot_lights()
                .len()
                .min(crate::lighting::MAX_SPOT_LIGHTS);
            let sun = crate::lighting::MAX_SPOT_LIGHTS as u32;

            let mut maps: Vec<(u32, &wgpu::TextureView)> = vec![(sun, &self.shadow_view)];
            for (index, layer) in self.spot_shadow_layers.iter().enumerate() {
                maps.push((index as u32, layer));
            }

            for (which, view) in maps {
                let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Shadow Pass"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });

                // a spot with nothing in it gets its clear and no drawing,
                // which costs an empty pass and keeps the layer honest
                if which < sun && which as usize >= spots {
                    continue;
                }

                shadow_pass.set_pipeline(&self.shadow_pipeline);
                shadow_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                shadow_pass.set_bind_group(1, &self.which_bind_groups[which as usize], &[]);
                shadow_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

                for (mesh_id, _, first, count) in batches.iter() {
                    let mesh = &self.meshes[mesh_id.0];
                    if mesh.index_count == 0 {
                        continue;
                    }
                    shadow_pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                    shadow_pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                    shadow_pass.draw_indexed(0..mesh.index_count, 0, *first..(*first + *count));
                }
            }

            // then six passes for each lamp that asked to cast, per spec 0022.
            // Every layer is cleared whether a lamp claimed it or not, for the
            // same reason the spots' are.
            let faces = point_faces(scene.point_lights());
            for (layer, view) in self.point_shadow_layers.iter().enumerate() {
                let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Point Shadow Pass"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(shadow::POINT_CLEAR),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });

                // no lamp claimed this layer, so it gets its clear and nothing
                // else
                if faces[layer].position_range[3] <= 0.0 {
                    continue;
                }

                shadow_pass.set_pipeline(&self.point_shadow_pipeline);
                shadow_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                shadow_pass.set_bind_group(1, &self.which_bind_groups[layer], &[]);
                shadow_pass.set_bind_group(2, &self.point_faces_bind_group, &[]);
                shadow_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

                for (mesh_id, _, first, count) in batches.iter() {
                    let mesh = &self.meshes[mesh_id.0];
                    if mesh.index_count == 0 {
                        continue;
                    }
                    shadow_pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                    shadow_pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                    shadow_pass.draw_indexed(0..mesh.index_count, 0, *first..(*first + *count));
                }
            }
        }

        // the world next, depth tested, then the interface painted on top
        {
            let mut mesh_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Mesh Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            // This pass clears the frame, so it runs even with nothing 3D in
            // it. Only the drawing waits on an instance buffer that exists.
            if !batches.is_empty() {
                mesh_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                mesh_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

                // solid first, so it is in the depth buffer before anything is
                // blended over it. Then the far side of everything see-through,
                // then the near side, per spec 0018.
                let runs = [
                    (&self.mesh_pipeline, &opaque),
                    (&self.translucent_pipelines[0], &translucent),
                    (&self.translucent_pipelines[1], &translucent),
                ];

                for (pipeline, batches) in runs {
                    if batches.is_empty() {
                        continue;
                    }
                    mesh_pass.set_pipeline(pipeline);

                    for (mesh_id, texture_id, first, count) in batches.iter() {
                        let mesh = &self.meshes[mesh_id.0];
                        if mesh.index_count == 0 {
                            continue;
                        }
                        mesh_pass.set_bind_group(1, &self.textures[texture_id.0], &[]);
                        mesh_pass.set_bind_group(2, &self.shadow_bind_group, &[]);
                        mesh_pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                        mesh_pass
                            .set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                        mesh_pass.draw_indexed(0..mesh.index_count, 0, *first..(*first + *count));
                    }
                }
            }
        }

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            let num_indices = geometry.index_data().len() as u32;
            if num_indices != 0 {
                render_pass.set_pipeline(&self.pipeline);
                render_pass.set_bind_group(0, &self.screen_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..num_indices, 0, 0..1);
            }

            self.text_brush.draw(&mut render_pass);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        self.queue.present(frame);
    }

    /// Packs every instance into one buffer and says where each mesh's run
    /// starts, so each mesh is one instanced draw. The opaque runs come back
    /// separately from the translucent ones, because they are drawn in
    /// different passes, per spec 0018.
    fn upload_instances(&mut self, scene: &Scene) -> (Vec<Batch>, Vec<Batch>) {
        let mut instances: Vec<Instance> = Vec::new();
        let mut opaque = Vec::new();
        let mut translucent = Vec::new();

        for (mesh, texture, mesh_instances) in scene.batches() {
            if mesh.0 >= self.meshes.len() {
                log::warn!("a scene asked for mesh {:?}, which was never added", mesh);
                continue;
            }
            if texture.0 >= self.textures.len() {
                log::warn!(
                    "a scene asked for texture {:?}, which was never added",
                    texture
                );
                continue;
            }

            // one mesh can be drawn solid here and see-through there, so the
            // split is per instance and each half gets its own run
            for (list, wanted) in [(&mut opaque, false), (&mut translucent, true)] {
                let first = instances.len() as u32;
                instances.extend(
                    mesh_instances
                        .iter()
                        .filter(|instance| instance.is_translucent() == wanted),
                );

                let count = instances.len() as u32 - first;
                if count > 0 {
                    list.push((mesh, texture, first, count));
                }
            }
        }

        if instances.is_empty() {
            return (opaque, translucent);
        }

        let bytes: &[u8] = bytemuck::cast_slice(&instances);
        if bytes.len() as u64 > self.instance_buffer.size() {
            self.instance_buffer =
                create_buffer_init(&self.device, bytes, wgpu::BufferUsages::VERTEX);
        } else {
            self.queue.write_buffer(&self.instance_buffer, 0, bytes);
        }

        (opaque, translucent)
    }

    /// Writes this frame's quads into the vertex and index buffers, growing them when they are too small.
    fn upload_geometry(&mut self, geometry: &Geometry) {
        let vertices: &[u8] = bytemuck::cast_slice(geometry.vertex_data());
        let indices: &[u8] = bytemuck::cast_slice(geometry.index_data());

        if vertices.len() as u64 > self.vertex_buffer.size() {
            self.vertex_buffer =
                create_buffer_init(&self.device, vertices, wgpu::BufferUsages::VERTEX);
        } else if !vertices.is_empty() {
            self.queue.write_buffer(&self.vertex_buffer, 0, vertices);
        }

        if indices.len() as u64 > self.index_buffer.size() {
            self.index_buffer =
                create_buffer_init(&self.device, indices, wgpu::BufferUsages::INDEX);
        } else if !indices.is_empty() {
            self.queue.write_buffer(&self.index_buffer, 0, indices);
        }
    }
}

fn create_buffer(
    device: &wgpu::Device,
    size: wgpu::BufferAddress,
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: usage | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn create_buffer_init(
    device: &wgpu::Device,
    contents: &[u8],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents,
        usage: usage | wgpu::BufferUsages::COPY_DST,
    })
}

/// One lamp as the shader reads it. Two vec4s exactly: a uniform block aligns
/// every array element to sixteen bytes, so anything that is not a multiple of
/// that gets padding the Rust side does not know about. Packing the range and
/// the intensity into the spare slots is what keeps it honest.
#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
struct GpuPointLight {
    /// xyz is where it is, w is how far it reaches.
    position_range: [f32; 4],
    /// rgb is the color, a is the intensity.
    color_intensity: [f32; 4],
}

unsafe impl bytemuck::Pod for GpuPointLight {}
unsafe impl bytemuck::Zeroable for GpuPointLight {}

impl GpuPointLight {
    fn new(light: &crate::lighting::PointLight) -> Self {
        Self {
            position_range: light.position.extend(light.range.max(0.0)).to_array(),
            color_intensity: light.color.extend(light.intensity).to_array(),
        }
    }
}

/// One spot as the shader reads it: three vec4s of parameters and the matrix
/// that puts a world position on its shadow map. 128 bytes, a multiple of
/// sixteen, so an array of them is not padded apart. See spec 0021.
#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
struct GpuSpotLight {
    /// xyz is where it is, w is how far it reaches.
    position_range: [f32; 4],
    /// xyz is the way it points, w is the cosine of the outer angle.
    direction_cos_outer: [f32; 4],
    /// rgb is the color, a is the intensity.
    color_intensity: [f32; 4],
    /// x is the cosine of the inner angle. The rest is padding, because the
    /// matrix after it has to start on sixteen bytes.
    cos_inner: [f32; 4],
    /// World to this spot's clip space, for its layer of the shadow map.
    view_projection: [f32; 16],
}

unsafe impl bytemuck::Pod for GpuSpotLight {}
unsafe impl bytemuck::Zeroable for GpuSpotLight {}

impl GpuSpotLight {
    fn new(light: &crate::lighting::SpotLight) -> Self {
        let (cos_outer, cos_inner) = light.cone_cosines();
        let direction = light.direction.normalize_or_zero();
        let direction = if direction.length_squared() < 0.5 {
            glam::Vec3::NEG_Y
        } else {
            direction
        };
        let range = light.range.max(0.0);

        Self {
            position_range: light.position.extend(range).to_array(),
            direction_cos_outer: direction.extend(cos_outer).to_array(),
            color_intensity: light.color.extend(light.intensity).to_array(),
            cos_inner: [cos_inner, 0.0, 0.0, 0.0],
            view_projection: shadow::spot_view_projection(
                light.position,
                direction,
                light.outer.max(light.inner),
                range,
            )
            .to_cols_array(),
        }
    }
}

/// One face of one casting lamp, as the pass that fills it reads it: the matrix
/// onto that face, and the lamp to measure distance from. 80 bytes, a multiple
/// of sixteen, so an array of them is not padded apart. See spec 0022.
#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
struct GpuPointFace {
    view_projection: [f32; 16],
    /// xyz is the lamp, w is how far it reaches. A range of zero means this
    /// layer belongs to no lamp this frame.
    position_range: [f32; 4],
}

unsafe impl bytemuck::Pod for GpuPointFace {}
unsafe impl bytemuck::Zeroable for GpuPointFace {}

/// The twelve layers for a frame, in the order the passes fill them: a casting
/// lamp's six faces together, then the next lamp's.
///
/// A layer no lamp claimed is left with a zero range, which the pass reads as
/// nothing to draw. Its map is still cleared, so a lamp that stopped casting
/// cannot leave last frame's shadow behind.
fn point_faces(lamps: &[crate::lighting::PointLight]) -> [GpuPointFace; POINT_SHADOW_LAYERS] {
    let mut faces = [GpuPointFace::default(); POINT_SHADOW_LAYERS];

    for (slot, index) in crate::lighting::casting_lamps(lamps)
        .into_iter()
        .enumerate()
    {
        let lamp = &lamps[index];
        let range = lamp.range.max(0.0);
        for face in 0..6 {
            faces[slot * 6 + face] = GpuPointFace {
                view_projection: shadow::face_view_projection(lamp.position, face, range)
                    .to_cols_array(),
                position_range: lamp.position.extend(range).to_array(),
            };
        }
    }

    faces
}

/// The camera, the sun, the lamps and the spots, as the GPU sees them. Every field is
/// padded out to four floats because that is how a uniform block is laid out.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct SceneUniform {
    view_projection: [f32; 16],
    camera_position: [f32; 4],
    light_direction: [f32; 4],
    /// rgb is the color, a is the intensity.
    light_color: [f32; 4],
    ambient: [f32; 4],
    /// World to the light's clip space, for the shadow map.
    light_view_projection: [f32; 16],
    /// x is how many of the lamps below are real. The rest is padding, because
    /// the block after it has to start on sixteen bytes.
    point_light_count: [u32; 4],
    point_lights: [GpuPointLight; crate::lighting::MAX_POINT_LIGHTS],
    /// x is how many of the spots below are real, and how many layers of the
    /// spot shadow map hold anything.
    spot_light_count: [u32; 4],
    spot_lights: [GpuSpotLight; crate::lighting::MAX_SPOT_LIGHTS],
    /// x is how many lamps cast, y and z are which of the lamps above those
    /// are. w is padding. See spec 0022.
    point_shadow: [u32; 4],
}

unsafe impl bytemuck::Pod for SceneUniform {}
unsafe impl bytemuck::Zeroable for SceneUniform {}

impl SceneUniform {
    fn new(
        camera: &Camera,
        light: &Light,
        lamps: &[crate::lighting::PointLight],
        spots: &[crate::lighting::SpotLight],
        bounds: &Aabb,
    ) -> Self {
        // the scene caps this already, per spec 0020, but the array is fixed
        // and reading past it would be a different kind of bug
        let used = lamps.len().min(crate::lighting::MAX_POINT_LIGHTS);
        let mut point_lights = [GpuPointLight::default(); crate::lighting::MAX_POINT_LIGHTS];
        for (slot, lamp) in point_lights.iter_mut().zip(&lamps[..used]) {
            *slot = GpuPointLight::new(lamp);
        }

        let lit = spots.len().min(crate::lighting::MAX_SPOT_LIGHTS);
        let mut spot_lights = [GpuSpotLight::default(); crate::lighting::MAX_SPOT_LIGHTS];
        for (slot, spot) in spot_lights.iter_mut().zip(&spots[..lit]) {
            *slot = GpuSpotLight::new(spot);
        }

        // the same rule the scene and the shadow passes use, so a lamp cannot be
        // shadowed in one place and not the other
        let casting = crate::lighting::casting_lamps(&lamps[..used]);
        let mut point_shadow = [0u32; 4];
        point_shadow[0] = casting.len() as u32;
        for (slot, index) in casting.iter().enumerate() {
            point_shadow[slot + 1] = *index as u32;
        }

        Self {
            view_projection: camera.view_projection().to_cols_array(),
            camera_position: camera.position.extend(0.0).to_array(),
            light_direction: light.direction.normalize_or_zero().extend(0.0).to_array(),
            light_color: light.color.extend(light.intensity).to_array(),
            ambient: light.ambient.extend(0.0).to_array(),
            light_view_projection: shadow::light_view_projection(light.direction, bounds)
                .to_cols_array(),
            point_light_count: [used as u32, 0, 0, 0],
            point_lights,
            spot_light_count: [lit as u32, 0, 0, 0],
            spot_lights,
            point_shadow,
        }
    }
}

/// The 3D pipeline: mesh vertices, one instance per copy, the camera uniform,
/// depth testing and back face culling from spec 0009.
/// Depth only, from the light. No fragment stage and no color target.
fn create_shadow_pipeline(
    device: &wgpu::Device,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    which_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("../../res/shaders/shadow.wgsl"));

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Shadow Pipeline Layout"),
        bind_group_layouts: &[Some(camera_bind_group_layout), Some(which_layout)],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Shadow Pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Some(crate::mesh::Vertex::DESC), Some(Instance::DESC)],
            compilation_options: Default::default(),
        },
        fragment: None,
        // front faces are culled instead of back ones, which pushes the
        // recorded depth to the far side of a wall and hides most acne
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Front),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: shadow::FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

/// A lamp's face. Like the sun's pass but with a fragment stage, because what
/// goes in the map is distance from the lamp rather than the depth its
/// projection happened to produce. See spec 0022.
fn create_point_shadow_pipeline(
    device: &wgpu::Device,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    which_layout: &wgpu::BindGroupLayout,
    point_faces_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("../../res/shaders/shadow.wgsl"));

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Point Shadow Pipeline Layout"),
        bind_group_layouts: &[
            Some(camera_bind_group_layout),
            Some(which_layout),
            Some(point_faces_layout),
        ],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Point Shadow Pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_point"),
            buffers: &[Some(crate::mesh::Vertex::DESC), Some(Instance::DESC)],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_point"),
            // the depth buffer is the whole output; the fragment stage exists
            // only to write a distance into it
            targets: &[],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Front),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: shadow::POINT_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

fn create_mesh_pipeline(
    device: &wgpu::Device,
    color_format: wgpu::TextureFormat,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    texture_bind_group_layout: &wgpu::BindGroupLayout,
    shadow_bind_group_layout: &wgpu::BindGroupLayout,
    primitive: wgpu::PrimitiveState,
    depth_stencil: wgpu::DepthStencilState,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("../../res/shaders/mesh.wgsl"));

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Mesh Pipeline Layout"),
        bind_group_layouts: &[
            Some(camera_bind_group_layout),
            Some(texture_bind_group_layout),
            Some(shadow_bind_group_layout),
        ],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Mesh Pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Some(crate::mesh::Vertex::DESC), Some(Instance::DESC)],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive,
        depth_stencil: Some(depth_stencil),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

/// A vertex or index buffer for a mesh. Empty contents get four zero bytes
/// instead, because `create_buffer_init` panics on nothing at all and an empty
/// mesh is something a game is allowed to hand over.
fn mesh_buffer(
    device: &wgpu::Device,
    label: &str,
    contents: &[u8],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: if contents.is_empty() {
            &[0; 4]
        } else {
            contents
        },
        usage,
    })
}

/// Quads are drawn in the order they are pushed, with no depth testing, per
/// spec 0001. The 3D pipeline is the one that uses `depth::state()`.
const QUAD_DEPTH_STENCIL: Option<wgpu::DepthStencilState> = None;

/// A uniform buffer binding, which is the shape both the screen size and the
/// scene use. `visibility` matters: the scene uniform carries the light, so the
/// fragment stage reads it too, and a vertex-only layout fails validation.
fn uniform_bind_group_layout(
    device: &wgpu::Device,
    label: &str,
    visibility: wgpu::ShaderStages,
) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(&format!("{} Bind Group Layout", label)),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

/// The screen size uniform the quad shader converts pixel positions with.
fn screen_size(config: &wgpu::SurfaceConfiguration) -> [f32; 4] {
    [config.width as f32, config.height as f32, 0.0, 0.0]
}

fn create_render_pipeline(
    device: &wgpu::Device,
    color_format: wgpu::TextureFormat,
    screen_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("../../res/shaders/quad.wgsl"));

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Pipeline Layout"),
        bind_group_layouts: &[Some(screen_bind_group_layout)],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Some(Vertex::DESC)],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: QUAD_DEPTH_STENCIL,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

/// What the focused item carries, and what every other selectable item is
/// indented by to match it. The same width either way, so a word keeps its
/// left edge as focus moves. See spec 0023.
const FOCUS_CARET: &str = "> ";
const FOCUS_INDENT: &str = "  ";

/// How much of its colour a selectable item keeps while it is not the focused
/// one. Colour rather than alpha, so it recedes against a light background
/// instead of disappearing.
const DIMMED: f32 = 0.6;

fn text_section(text: &RenderText) -> Section<'_> {
    let layout = Layout::default().h_align(if text.centered {
        HorizontalAlign::Center
    } else {
        HorizontalAlign::Left
    });

    let color = if text.selectable && !text.focused {
        glam::vec4(
            text.color.x * DIMMED,
            text.color.y * DIMMED,
            text.color.z * DIMMED,
            text.color.w,
        )
    } else {
        text.color
    };

    let mut section = Section::default()
        .with_screen_position((text.position.x, text.position.y))
        .with_bounds((text.bounds.x, text.bounds.y))
        .with_layout(layout);

    if text.selectable {
        let prefix = if text.focused {
            FOCUS_CARET
        } else {
            FOCUS_INDENT
        };
        section = section.add_text(Text::new(prefix).with_color(color).with_scale(text.size));
    }

    section.add_text(
        Text::new(&text.text)
            .with_color(color)
            .with_scale(text.size),
    )
}

#[cfg(test)]
mod text_tests {
    use super::*;

    fn item(selectable: bool, focused: bool) -> RenderText {
        RenderText {
            text: String::from("Play"),
            size: 32.0,
            color: glam::vec4(1.0, 1.0, 1.0, 1.0),
            selectable,
            focused,
            ..Default::default()
        }
    }

    fn colors(section: &Section<'_>) -> Vec<[f32; 4]> {
        section.text.iter().map(|t| t.extra.color).collect()
    }

    fn words<'a>(section: &Section<'a>) -> Vec<&'a str> {
        section.text.iter().map(|t| t.text).collect()
    }

    #[test]
    fn plain_text_is_left_alone() {
        let title = item(false, false);
        let section = text_section(&title);

        assert_eq!(words(&section), vec!["Play"]);
        assert_eq!(colors(&section), vec![[1.0, 1.0, 1.0, 1.0]]);
    }

    #[test]
    fn plain_text_gets_no_prefix() {
        // a title has nothing to line up with, so it is not indented either
        let mut title = item(false, false);
        title.focused = true;

        assert_eq!(words(&text_section(&title)), vec!["Play"]);
    }

    #[test]
    fn a_selectable_item_is_dimmed() {
        let dim = item(true, false);
        let section = text_section(&dim);

        for color in colors(&section) {
            assert_eq!(color[0], DIMMED, "red");
            assert_eq!(color[1], DIMMED, "green");
            assert_eq!(color[2], DIMMED, "blue");
        }
    }

    #[test]
    fn the_focused_item_is_not_dimmed() {
        let focused = item(true, true);
        let section = text_section(&focused);

        for color in colors(&section) {
            assert_eq!(color[..3], [1.0, 1.0, 1.0], "full colour");
        }
    }

    #[test]
    fn dimming_leaves_alpha_alone() {
        let mut half = item(true, false);
        half.color = glam::vec4(1.0, 1.0, 1.0, 0.5);

        for color in colors(&text_section(&half)) {
            assert_eq!(color[3], 0.5, "alpha is untouched");
        }
    }

    #[test]
    fn the_focused_item_carries_a_caret() {
        let focused = item(true, true);
        let section = text_section(&focused);

        assert_eq!(words(&section), vec![FOCUS_CARET, "Play"]);
    }

    #[test]
    fn an_unfocused_item_is_indented_to_match() {
        let unfocused = item(true, false);
        let section = text_section(&unfocused);

        assert_eq!(words(&section), vec![FOCUS_INDENT, "Play"]);
    }

    #[test]
    fn the_two_prefixes_are_the_same_width() {
        // the caret is drawn in the same monospaced font as the word, so equal
        // character counts is equal width, and nothing shifts as focus moves
        assert_eq!(FOCUS_CARET.chars().count(), FOCUS_INDENT.chars().count());
    }

    #[test]
    fn focus_does_not_change_the_size() {
        let sizes: Vec<f32> = [item(true, true), item(true, false), item(false, false)]
            .iter()
            .flat_map(|t| {
                text_section(t)
                    .text
                    .iter()
                    .map(|p| p.scale.x)
                    .collect::<Vec<_>>()
            })
            .collect();

        assert!(sizes.iter().all(|s| *s == 32.0), "sizes were {:?}", sizes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_2d_pipeline_ignores_depth() {
        // quads and text keep painting over each other in push order
        assert!(QUAD_DEPTH_STENCIL.is_none());
    }
}

#[cfg(test)]
mod uniform_tests {
    use super::*;
    use crate::lighting::{PointLight, SpotLight, MAX_POINT_LIGHTS, MAX_SPOT_LIGHTS};

    #[test]
    fn the_scene_uniform_is_laid_out_for_the_gpu() {
        // a uniform block wants each field on a 16 byte boundary
        assert_eq!(
            std::mem::size_of::<SceneUniform>(),
            64 + 16 * 4 + 64 + 16 + 32 * MAX_POINT_LIGHTS + 16 + 128 * MAX_SPOT_LIGHTS + 16
        );
        assert_eq!(
            std::mem::offset_of!(SceneUniform, light_view_projection),
            128
        );
        assert_eq!(std::mem::offset_of!(SceneUniform, camera_position), 64);
        assert_eq!(std::mem::offset_of!(SceneUniform, light_direction), 80);
        assert_eq!(std::mem::offset_of!(SceneUniform, light_color), 96);
        assert_eq!(std::mem::offset_of!(SceneUniform, ambient), 112);
        assert_eq!(std::mem::offset_of!(SceneUniform, point_light_count), 192);
        assert_eq!(std::mem::offset_of!(SceneUniform, point_lights), 208);
        assert_eq!(std::mem::offset_of!(SceneUniform, spot_light_count), 464);
        assert_eq!(std::mem::offset_of!(SceneUniform, spot_lights), 480);
        assert_eq!(std::mem::offset_of!(SceneUniform, point_shadow), 992);

        for offset in [0, 64, 80, 96, 112, 128, 192, 208, 464, 480, 992] {
            assert_eq!(offset % 16, 0, "{} is not on sixteen bytes", offset);
        }
    }

    #[test]
    fn the_uniform_carries_the_light() {
        let mut light = Light::new();
        light.intensity = 0.5;
        let uniform =
            SceneUniform::new(&Camera::new(), &light, &[], &[], &shadow::default_bounds());

        assert_eq!(uniform.light_color[3], 0.5);
        // pointing down, as the default light comes from above
        assert!(uniform.light_direction[1] < 0.0);
    }

    #[test]
    fn a_lamp_is_two_vec4s_and_nothing_else() {
        // an array in a uniform block puts each element on sixteen bytes, so a
        // lamp that is not a multiple of that gets padding the Rust side does
        // not know about, and every lamp after the first reads shifted
        assert_eq!(std::mem::size_of::<GpuPointLight>(), 32);
        assert_eq!(std::mem::offset_of!(GpuPointLight, position_range), 0);
        assert_eq!(std::mem::offset_of!(GpuPointLight, color_intensity), 16);
        assert!(std::mem::align_of::<GpuPointLight>() <= 16);
        assert_eq!(
            std::mem::size_of::<[GpuPointLight; MAX_POINT_LIGHTS]>(),
            32 * MAX_POINT_LIGHTS
        );
    }

    #[test]
    fn the_uniform_carries_the_lamps_it_is_given() {
        let lamps = [
            PointLight::new(
                glam::vec3(1.0, 2.0, 3.0),
                glam::vec3(1.0, 0.0, 0.0),
                2.0,
                5.0,
            ),
            PointLight::new(
                glam::vec3(-4.0, 0.0, 0.0),
                glam::vec3(0.0, 1.0, 0.0),
                0.5,
                9.0,
            ),
        ];
        let uniform = SceneUniform::new(
            &Camera::new(),
            &Light::new(),
            &lamps,
            &[],
            &shadow::default_bounds(),
        );

        assert_eq!(uniform.point_light_count[0], 2);
        assert_eq!(uniform.point_lights[0].position_range, [1.0, 2.0, 3.0, 5.0]);
        assert_eq!(
            uniform.point_lights[0].color_intensity,
            [1.0, 0.0, 0.0, 2.0]
        );
        assert_eq!(
            uniform.point_lights[1].position_range,
            [-4.0, 0.0, 0.0, 9.0]
        );
        // the rest stay at nothing, so a lamp from a past frame cannot light
        assert_eq!(uniform.point_lights[2].color_intensity, [0.0; 4]);
    }

    #[test]
    fn no_lamps_is_a_count_of_none() {
        let uniform = SceneUniform::new(
            &Camera::new(),
            &Light::new(),
            &[],
            &[],
            &shadow::default_bounds(),
        );

        assert_eq!(uniform.point_light_count[0], 0);
        for lamp in uniform.point_lights.iter() {
            assert_eq!(lamp.position_range, [0.0; 4]);
        }
    }

    #[test]
    fn more_lamps_than_fit_are_cut_rather_than_overrunning() {
        let many: Vec<PointLight> = (0..MAX_POINT_LIGHTS + 5)
            .map(|index| {
                PointLight::new(glam::Vec3::splat(index as f32), glam::Vec3::ONE, 1.0, 1.0)
            })
            .collect();
        let uniform = SceneUniform::new(
            &Camera::new(),
            &Light::new(),
            &many,
            &[],
            &shadow::default_bounds(),
        );

        assert_eq!(uniform.point_light_count[0] as usize, MAX_POINT_LIGHTS);
    }

    #[test]
    fn a_spot_is_laid_out_for_an_array() {
        // 128 bytes, a multiple of sixteen, or every spot after the first
        // reads shifted by however much padding was inserted
        assert_eq!(std::mem::size_of::<GpuSpotLight>(), 128);
        assert_eq!(std::mem::size_of::<GpuSpotLight>() % 16, 0);
        assert_eq!(std::mem::offset_of!(GpuSpotLight, position_range), 0);
        assert_eq!(std::mem::offset_of!(GpuSpotLight, direction_cos_outer), 16);
        assert_eq!(std::mem::offset_of!(GpuSpotLight, color_intensity), 32);
        assert_eq!(std::mem::offset_of!(GpuSpotLight, cos_inner), 48);
        assert_eq!(std::mem::offset_of!(GpuSpotLight, view_projection), 64);
        assert_eq!(
            std::mem::size_of::<[GpuSpotLight; MAX_SPOT_LIGHTS]>(),
            128 * MAX_SPOT_LIGHTS
        );
    }

    #[test]
    fn the_uniform_carries_the_spots_it_is_given() {
        let spots = [SpotLight::new(
            glam::vec3(0.0, 3.0, 0.0),
            -glam::Vec3::Y,
            glam::vec3(1.0, 0.0, 0.0),
            2.0,
            9.0,
            0.2,
            0.5,
        )];
        let uniform = SceneUniform::new(
            &Camera::new(),
            &Light::new(),
            &[],
            &spots,
            &shadow::default_bounds(),
        );

        assert_eq!(uniform.spot_light_count[0], 1);
        assert_eq!(uniform.spot_lights[0].position_range, [0.0, 3.0, 0.0, 9.0]);
        assert_eq!(uniform.spot_lights[0].color_intensity, [1.0, 0.0, 0.0, 2.0]);
        // the wider angle has the smaller cosine, and it is the outer one
        let cos_outer = uniform.spot_lights[0].direction_cos_outer[3];
        let cos_inner = uniform.spot_lights[0].cos_inner[0];
        assert!(cos_outer < cos_inner, "the cone is inside out");
        // and its matrix is real rather than left at zero
        assert!(uniform.spot_lights[0]
            .view_projection
            .iter()
            .any(|v| *v != 0.0));
        assert!(uniform.spot_lights[0]
            .view_projection
            .iter()
            .all(|v| v.is_finite()));

        // the rest stay dark
        assert_eq!(uniform.spot_lights[1].color_intensity, [0.0; 4]);
    }

    #[test]
    fn a_negative_range_reaches_nothing_rather_than_wrapping() {
        let bad = PointLight::new(glam::Vec3::ZERO, glam::Vec3::ONE, 1.0, -3.0);
        let uniform = SceneUniform::new(
            &Camera::new(),
            &Light::new(),
            &[bad],
            &[],
            &shadow::default_bounds(),
        );

        assert_eq!(uniform.point_lights[0].position_range[3], 0.0);
    }

    #[test]
    fn a_face_is_a_matrix_and_a_lamp_and_nothing_else() {
        assert_eq!(std::mem::size_of::<GpuPointFace>(), 80);
        assert_eq!(std::mem::offset_of!(GpuPointFace, view_projection), 0);
        assert_eq!(std::mem::offset_of!(GpuPointFace, position_range), 64);
        assert!(std::mem::align_of::<GpuPointFace>() <= 16);
        assert_eq!(
            std::mem::size_of::<[GpuPointFace; POINT_SHADOW_LAYERS]>(),
            80 * POINT_SHADOW_LAYERS
        );
    }

    #[test]
    fn the_uniform_says_where_each_casting_lamp_starts() {
        let lamps = [
            PointLight::new(glam::Vec3::ZERO, glam::Vec3::ONE, 1.0, 5.0),
            PointLight::new(glam::Vec3::X, glam::Vec3::ONE, 1.0, 5.0).casting(),
            PointLight::new(glam::Vec3::Y, glam::Vec3::ONE, 1.0, 5.0),
            PointLight::new(glam::Vec3::Z, glam::Vec3::ONE, 1.0, 5.0).casting(),
        ];
        let uniform = SceneUniform::new(
            &Camera::new(),
            &Light::new(),
            &lamps,
            &[],
            &shadow::default_bounds(),
        );

        assert_eq!(uniform.point_shadow[0], 2, "the count is wrong");
        // the lamps that cast, by their place in the array the shader loops over
        assert_eq!(uniform.point_shadow[1], 1);
        assert_eq!(uniform.point_shadow[2], 3);

        // and all four still light, per spec 0022
        assert_eq!(uniform.point_light_count[0], 4);
    }

    #[test]
    fn no_lamp_casting_is_a_count_of_none() {
        let lamps = [PointLight::new(glam::Vec3::ZERO, glam::Vec3::ONE, 1.0, 5.0)];
        let uniform = SceneUniform::new(
            &Camera::new(),
            &Light::new(),
            &lamps,
            &[],
            &shadow::default_bounds(),
        );

        assert_eq!(uniform.point_shadow, [0; 4]);
    }

    #[test]
    fn a_casting_lamp_gets_six_layers_and_the_rest_get_none() {
        let lamps =
            [PointLight::new(glam::vec3(0.0, 3.0, 0.0), glam::Vec3::ONE, 1.0, 8.0).casting()];
        let faces = point_faces(&lamps);

        for (layer, face) in faces.iter().take(6).enumerate() {
            assert_eq!(
                face.position_range,
                [0.0, 3.0, 0.0, 8.0],
                "layer {} is not the lamp's",
                layer
            );
            // and the matrix is the one that face's pass will use
            assert_eq!(
                face.view_projection,
                shadow::face_view_projection(glam::vec3(0.0, 3.0, 0.0), layer, 8.0).to_cols_array()
            );
        }

        // the second lamp's six belong to nobody, and a range of nothing is how
        // the pass knows to draw nothing into them
        for (layer, face) in faces.iter().enumerate().skip(6) {
            assert_eq!(face.position_range[3], 0.0, "layer {} was claimed", layer);
        }
    }

    #[test]
    fn the_faces_and_the_uniform_agree_on_which_lamps_cast() {
        // the shader finds a lamp's layers from its slot in the uniform's table,
        // so the table and the order the passes fill the layers in have to be
        // the same order
        let lamps = [
            PointLight::new(glam::Vec3::X * 2.0, glam::Vec3::ONE, 1.0, 4.0),
            PointLight::new(glam::Vec3::Y * 2.0, glam::Vec3::ONE, 1.0, 4.0).casting(),
            PointLight::new(glam::Vec3::Z * 2.0, glam::Vec3::ONE, 1.0, 4.0).casting(),
        ];
        let uniform = SceneUniform::new(
            &Camera::new(),
            &Light::new(),
            &lamps,
            &[],
            &shadow::default_bounds(),
        );
        let faces = point_faces(&lamps);

        for slot in 0..uniform.point_shadow[0] as usize {
            let lamp = uniform.point_lights[uniform.point_shadow[slot + 1] as usize];
            assert_eq!(
                faces[slot * 6].position_range,
                lamp.position_range,
                "slot {} points at a different lamp than its layers hold",
                slot
            );
        }
    }
}
