//! wgpu: the device, the pipelines, and everything drawn through them.
//!
//! Two of them do the drawing, one for quads and one for meshes, with shadow
//! passes ahead of both. What a game pushes to be drawn in 3D is `scene`, the
//! depth buffer and its settings are `depth`, and text goes through
//! `render_text`.
//!
//! A game never sees a wgpu type. Anything uploaded comes back as a handle.

pub mod coplanar;
mod culling;
pub mod depth;
mod hidden;
pub mod outline;
pub mod render_text;
pub mod scene;

use std::sync::atomic::{AtomicBool, Ordering};

use web_time::Instant;

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
use culling::{Drawn, Seen, FIRST_LAMP, FIRST_SPOT, PICTURE, SUN, VIEWS};
use render_text::*;
use scene::{Instance, MeshId, Scene, TextureId};

use std::sync::Arc;

use wgpu::util::DeviceExt;
use wgpu_text::glyph_brush::ab_glyph::FontRef;
use wgpu_text::glyph_brush::{HorizontalAlign, Layout, Section, Text};
use wgpu_text::{BrushBuilder, TextBrush};
use winit::window::Window;

use crate::text::FONT_BYTES;

/// How many fighting pairs a coplanar check writes into the log, biggest
/// first, per spec 0048. The rest are in [`Renderer::fights`] to be outlined.
const SAID: usize = 20;

pub struct Renderer {
    /// Kept so a game can lock or hide the cursor, per spec 0013. The surface
    /// owns the window too, but does not hand it back.
    ///
    /// Nothing when this renderer has no window, which is what a picture
    /// test uses: the same device, the same passes and the same pipelines,
    /// with a texture where the surface would be. Spec 0062.
    window: Option<Arc<Window>>,
    cursor_locked: bool,
    surface: Option<wgpu::Surface<'static>>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    /// What everything is drawn through, which is `config.format` with sRGB
    /// on it. See where it is worked out, in `new`.
    drawn: wgpu::TextureFormat,
    /// What the last frame's drawing came to. Spec 0056.
    counted: Counted,
    /// And what the GPU said it took, where the device will say. Spec 0056.
    timing: Option<Timing>,
    /// What the last frame's work came to on this side, in seconds, and how
    /// much of that was spent waiting for the display to hand back a surface.
    worked: f32,
    waited: f32,
    /// And how long it was from the previous frame to this one, uncapped.
    took: f32,
    pipeline: wgpu::RenderPipeline,
    screen_buffer: wgpu::Buffer,
    screen_bind_group: wgpu::BindGroup,
    camera: Camera,
    camera_buffer: wgpu::Buffer,
    /// Bound by the 3D pipeline, which arrives with meshes in spec 0010.
    #[allow(dead_code)]
    camera_bind_group: wgpu::BindGroup,
    /// Every kept run's instances, written when the run was kept and left
    /// where they were put. Spec 0063.
    kept_buffer: wgpu::Buffer,
    /// Where each kept run lies in that buffer, by the run's own name.
    kept_homes: std::collections::HashMap<u64, culling::Home>,
    /// What the kept buffer holds: each run and the version of it that was
    /// written. A run that moved, or a run that came or went, is what makes
    /// this disagree with the scene and the buffer be written again.
    kept_built: Vec<(u64, u64)>,
    /// One entry a kept run, holding the run's middle with the eye taken
    /// off, which the vertex stage adds to every instance of it. Entry
    /// nought adds nothing and is what a run written this frame uses.
    standing_buffer: wgpu::Buffer,
    /// How many kept instances would not fit in a buffer on this device,
    /// and so were not drawn. Spec 0069.
    turned_away: usize,
    /// Whether that has been said. Once, like every other cap here.
    said_what_did_not_fit: bool,
    /// How far apart those entries are, which the device decides: a
    /// dynamic offset has to be a multiple of its alignment, and sixteen
    /// bytes of use costs two hundred and fifty six of room.
    standing_stride: u64,
    /// Kept so that pipeline can be built against the same layout.
    #[allow(dead_code)]
    camera_bind_group_layout: wgpu::BindGroupLayout,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    text_brush: TextBrush<FontRef<'static>>,
    /// Built here and resized with the surface. The pass that attaches it
    /// arrives with meshes in spec 0010; quads and text never use it.
    depth: depth::DepthTexture,
    /// What the world is drawn into when the device multisamples, and which is
    /// resolved onto the surface at the end of the mesh pass. Spec 0052.
    ///
    /// `None` on a device that will not have it, where the world goes straight
    /// onto the surface as it always did.
    colour: Option<wgpu::TextureView>,
    mesh_pipeline: wgpu::RenderPipeline,
    /// The prepass again for anything whose texture has holes in it, with the
    /// texture bound so the holes can be cut before the depth is written.
    /// Spec 0047.
    cutout_prepass_pipeline: wgpu::RenderPipeline,
    /// Collision boxes drawn over the scene as lines, per spec 0046.
    outline_pipeline: wgpu::RenderPipeline,
    outline_buffer: wgpu::Buffer,
    /// Depth for the whole scene, settled before anything is shaded, so the
    /// mesh pass can read it while it tests against it. See spec 0029.
    prepass_pipeline: wgpu::RenderPipeline,
    /// Which textures have something cut out of them, by `TextureId`. Spec
    /// 0047.
    holed: Vec<bool>,
    scene_depth_layout: wgpu::BindGroupLayout,
    scene_depth_bind_group: wgpu::BindGroup,
    /// Whether the march happens at all. A cost, and a game with nothing
    /// resting on anything does not need it.
    contact_shadows: bool,
    /// What a wall hides, per spec 0061: what is known, what is being
    /// asked, and the pipeline that asks it.
    hidden: hidden::Hidden,
    asking: Option<hidden::Asking>,
    hidden_pipeline: wgpu::RenderPipeline,
    /// A unit cube, for drawing a run's box with. Added once, with the
    /// renderer's own meshes.
    box_mesh: Option<MeshId>,
    /// How many boxes this frame asked about.
    asked_count: u32,
    /// Which frame this is, so an answer can be known to be stale.
    frame: u64,
    /// Whether a frame asks what is hidden at all. Off, because it costs a
    /// pass and a read and saves nothing in a scene with nothing in front of
    /// anything. Spec 0061.
    occluding: bool,
    /// Whether each pass leaves out what it cannot see. Spec 0057.
    ///
    /// On, because a game should not have to ask not to draw the room behind
    /// it. Off is for measuring what it saves, which is what `load` does
    /// with it: a comparison made in one binary on one machine in one minute
    /// is the only kind this project has found it can trust.
    culling: bool,
    meshes: Vec<GpuMesh>,
    /// Whether a coplanar check is owed, and what the last one found. A game
    /// asks for one on a key press and reads the answer back the frame after.
    /// See spec 0048.
    asked_fights: bool,
    fights: coplanar::Fights,
    /// The body to fill out from the camera with, if the game said. See spec
    /// 0049.
    body: Option<crate::reach::Body>,
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
    /// The box the mesh fits inside, kept so the coplanar check has something
    /// to measure without reading the vertices back off the GPU. See spec
    /// 0048.
    bounds: crate::collision::Aabb,
    /// Whether that box is a statement about this mesh's faces or only about
    /// how much room it takes up. See `MeshData::is_box`.
    boxy: bool,
}

/// What asking the surface for a frame came back with, as what to do about it.
///
/// Out of `render` so the rule can be checked without a window. Whether an
/// answer draws, reconfigures or is worth a line in the log is arithmetic, and
/// the one that matters is that every answer which does not draw still hands
/// its staging back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Asked {
    /// Whether there is a frame to draw into.
    pub draws: bool,
    /// Whether the surface wants setting up again.
    pub reconfigure: bool,
    /// Whether this is worth a line in the log.
    pub complain: bool,
}

impl Asked {
    pub(crate) fn at(got: &wgpu::CurrentSurfaceTexture) -> Self {
        match got {
            wgpu::CurrentSurfaceTexture::Success(_) => Self {
                draws: true,
                reconfigure: false,
                complain: false,
            },
            wgpu::CurrentSurfaceTexture::Suboptimal(_) => Self {
                draws: true,
                reconfigure: true,
                complain: false,
            },
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => Self {
                draws: false,
                reconfigure: true,
                complain: false,
            },
            // Not faults, and not worth a line each at a thousand frames a
            // second. A window behind another window is occluded and a machine
            // under load times out, and both of them come back on their own.
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => Self {
                draws: false,
                reconfigure: false,
                complain: false,
            },
            _ => Self {
                draws: false,
                reconfigure: false,
                complain: true,
            },
        }
    }

    /// Whether this frame has to hand its staging back before it gives up.
    ///
    /// Every answer that does not draw. `write_buffer` stages into memory that
    /// only comes back on a submit, so a frame that uploads and then returns
    /// keeps what it uploaded until the process ends.
    pub(crate) fn must_flush(&self) -> bool {
        !self.draws
    }
}

impl Renderer {
    pub fn width(&self) -> f32 {
        self.config.width as f32
    }

    pub fn height(&self) -> f32 {
        self.config.height as f32
    }

    pub async fn new(
        window: Arc<Window>,
        instance_desc: wgpu::InstanceDescriptor,
        presenting: Presenting,
        timing: bool,
    ) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(instance_desc);
        let surface = instance.create_surface(window.clone()).unwrap();

        Self::made(
            instance,
            Some(surface),
            Some(window),
            size,
            presenting,
            timing,
        )
        .await
    }

    /// A renderer with no window, drawing into a texture. Spec 0062.
    ///
    /// **The same engine, not a copy of it.** Every pipeline, every pass and
    /// every uniform is the one a game gets; what differs is that there is
    /// no surface to present to, so a frame is read back instead. That is
    /// what makes a picture drawn here worth comparing against one a game
    /// drew.
    ///
    /// Nothing at all on a machine with no adapter, which is what a plain
    /// container is: a picture test skips there rather than failing.
    pub async fn offscreen(width: u32, height: u32) -> Option<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());

        instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .ok()?;

        Some(
            Self::made(
                instance,
                None,
                None,
                winit::dpi::PhysicalSize::new(width.max(1), height.max(1)),
                Presenting::AsFastAsItWill,
                // and it times its frames, so that the one thing in this
                // engine that can only be checked against a real device
                // has a test. Spec 0065.
                true,
            )
            .await,
        )
    }

    async fn made(
        instance: wgpu::Instance,
        surface: Option<wgpu::Surface<'static>>,
        window: Option<Arc<Window>>,
        size: winit::dpi::PhysicalSize<u32>,
        presenting: Presenting,
        timing: bool,
    ) -> Self {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: surface.as_ref(),
                ..Default::default()
            })
            .await
            .unwrap();

        // Spec 0052: a device holds every format to what the WebGPU
        // specification guarantees unless it is asked for the rest, and eight
        // samples on a depth format is past the guarantee. The adapter saying
        // the hardware has eight is not the device agreeing to use it.
        let beyond_the_spec = adapter
            .features()
            .contains(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES);
        // Spec 0056: and the GPU's own clock, for the one game that measures.
        // Asked for only when it is wanted, so no other game pays two
        // timestamps and a buffer read a frame for a number it never shows.
        let clocked = timing && adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY);

        if timing && !clocked {
            log::warn!("this device will not time a frame, so only the wall clock is measured");
        }

        let wanted = if beyond_the_spec {
            wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
        } else {
            wgpu::Features::empty()
        } | if clocked {
            wgpu::Features::TIMESTAMP_QUERY
        } else {
            wgpu::Features::empty()
        };

        // **The room the adapter has rather than the room the default asks
        // for.** `Limits::default()` holds a buffer to 256 MB, which at a
        // hundred and twenty bytes an instance is two million of them, and
        // this engine never asked for anything else: on the machine this was
        // written on the adapter gives four gigabytes, which is thirty five
        // million. Asked for and not assumed, with the default as the way
        // back, because a browser may well say no and that is the one place
        // this cannot be worked around. Spec 0060.
        let (device, queue) = match adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Device"),
                required_features: wanted,
                required_limits: adapter.limits(),
                ..Default::default()
            })
            .await
        {
            Ok(both) => both,
            Err(why) => {
                log::warn!(
                    "this device would not give the adapter's own limits ({}), so the frame has \
                     the default room in it",
                    why
                );

                adapter
                    .request_device(&wgpu::DeviceDescriptor {
                        label: Some("Device"),
                        required_features: wanted,
                        ..Default::default()
                    })
                    .await
                    .unwrap()
            }
        };

        log::info!(
            "a buffer may hold {} instances",
            device.limits().max_buffer_size / Instance::SIZE
        );
        let timing = clocked.then(|| Timing::new(&device, &queue));

        // a surface says what it can do; a texture is told. The formats a
        // window offers are the ones a picture test asks for by hand, so
        // both go through `formats` and come out drawing the same way.
        let offered = match &surface {
            Some(surface) => surface.get_capabilities(&adapter).formats,
            None => vec![wgpu::TextureFormat::Rgba8Unorm],
        };
        let (format, drawn) = formats(&offered);
        let present_mode = match &surface {
            Some(surface) => presenting.out_of(&surface.get_capabilities(&adapter).present_modes),
            None => wgpu::PresentMode::Fifo,
        };

        if presenting != Presenting::WaitForTheDisplay {
            log::info!("presenting {:?}, asked for {:?}", present_mode, presenting);
        }

        let config = match &surface {
            Some(surface) => wgpu::SurfaceConfiguration {
                format,
                present_mode,
                view_formats: if drawn == format {
                    Vec::new()
                } else {
                    vec![drawn]
                },
                ..surface
                    .get_default_config(&adapter, size.width.max(1), size.height.max(1))
                    .unwrap()
            },
            None => wgpu::SurfaceConfiguration {
                color_space: wgpu::SurfaceColorSpace::Srgb,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width: size.width.max(1),
                height: size.height.max(1),
                present_mode,
                desired_maximum_frame_latency: 2,
                alpha_mode: wgpu::CompositeAlphaMode::Opaque,
                view_formats: if drawn == format {
                    Vec::new()
                } else {
                    vec![drawn]
                },
            },
        };

        if let Some(surface) = &surface {
            surface.configure(&device, &config);
        }

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
                true,
            )]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        // the scene's own uniform, and beside it the one entry of the
        // standing table this draw is for. Spec 0063: the second is read at
        // a dynamic offset, which is the one way a value can differ from
        // draw to draw without being in the instance itself.
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Scene Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: wgpu::BufferSize::new(STANDING),
                        },
                        count: None,
                    },
                ],
            });
        let standing_stride = device
            .limits()
            .min_uniform_buffer_offset_alignment
            .max(STANDING as u32) as u64;
        let standing_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Standing"),
            size: standing_stride,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind_group = standing_bind_group(
            &device,
            &camera_bind_group_layout,
            &camera_buffer,
            &standing_buffer,
        );
        let kept_buffer = create_buffer(&device, 0, wgpu::BufferUsages::VERTEX);

        let pipeline = create_render_pipeline(&device, drawn, &screen_bind_group_layout);

        let vertex_buffer = create_buffer(&device, 0, wgpu::BufferUsages::VERTEX);
        let index_buffer = create_buffer(&device, 0, wgpu::BufferUsages::INDEX);

        let text_brush = BrushBuilder::using_font_bytes(FONT_BYTES).unwrap().build(
            &device,
            config.width,
            config.height,
            drawn,
        );

        // How many samples a pixel is worked out from, asked of the device
        // rather than assumed. Spec 0052: four is what everything this has run
        // on supports, and a device that will not have it gets one and a
        // staircase, which is what every device got before.
        //
        // Both formats have to agree, because they are attached to one pass.
        let samples = {
            let got = depth::samples(
                adapter.get_texture_format_features(drawn).flags,
                adapter.get_texture_format_features(depth::FORMAT).flags,
                beyond_the_spec,
            );

            if got <= 1 {
                log::warn!("this device will not multisample, so edges are drawn hard");
            } else {
                log::info!("drawing with {} samples a pixel", got);
            }

            // and how big a surface, which is the other thing worth knowing
            // from a machine somebody else is sitting at. A window has a size
            // because something gave it one; a canvas has whatever winit was
            // told, and a surface one pixel across is a canvas of one flat
            // colour stretched over however wide the page lays it out.
            log::info!("drawing {} by {}", config.width, config.height);

            got
        };
        let depth = depth::DepthTexture::new(&device, config.width, config.height, samples);
        let colour = multisampled(&device, &config, drawn, samples);
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
        let scene_depth_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Scene Depth Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        // the depth it reads is the one the world is drawn
                        // into, so when that is multisampled this is too, and
                        // the march in spec 0029 loads one sample of it
                        multisampled: samples > 1,
                    },
                    count: None,
                }],
            });
        let scene_depth_bind_group = scene_depth_group(&device, &scene_depth_layout, &depth);
        let prepass_pipeline = create_prepass_pipeline(&device, &camera_bind_group_layout, samples);
        let hidden_pipeline = create_hidden_pipeline(&device, &camera_bind_group_layout, samples);

        let mesh_pipeline = create_mesh_pipeline(
            &device,
            drawn,
            MeshLayouts {
                camera: &camera_bind_group_layout,
                texture: &texture_bind_group_layout,
                shadow: &shadow_bind_group_layout,
                scene_depth: &scene_depth_layout,
            },
            depth::mesh_primitive_state(),
            depth::read_only_state(),
            samples,
        );
        let cutout_prepass_pipeline = create_cutout_prepass_pipeline(
            &device,
            &camera_bind_group_layout,
            &texture_bind_group_layout,
            samples,
        );

        // the same shader twice more, for the far side of a translucent shape
        // and then its near side, per spec 0018
        let translucent_pipelines = [wgpu::Face::Front, wgpu::Face::Back].map(|cull| {
            create_mesh_pipeline(
                &device,
                drawn,
                MeshLayouts {
                    camera: &camera_bind_group_layout,
                    texture: &texture_bind_group_layout,
                    shadow: &shadow_bind_group_layout,
                    scene_depth: &scene_depth_layout,
                },
                depth::translucent_primitive_state(cull),
                depth::translucent_state(),
                samples,
            )
        });
        let instance_buffer = create_buffer(&device, 0, wgpu::BufferUsages::VERTEX);
        let outline_pipeline =
            create_outline_pipeline(&device, &camera_bind_group_layout, drawn, samples);
        let outline_buffer = create_buffer(&device, 0, wgpu::BufferUsages::VERTEX);

        let mut renderer = Self {
            window,
            cursor_locked: false,
            surface,
            device,
            queue,
            config,
            drawn,
            counted: Counted::default(),
            timing,
            worked: 0.0,
            waited: 0.0,
            took: 0.0,
            pipeline,
            screen_buffer,
            screen_bind_group,
            camera,
            camera_buffer,
            camera_bind_group,
            camera_bind_group_layout,
            kept_buffer,
            kept_homes: std::collections::HashMap::new(),
            kept_built: Vec::new(),
            turned_away: 0,
            said_what_did_not_fit: false,
            standing_buffer,
            standing_stride,
            vertex_buffer,
            index_buffer,
            text_brush,
            depth,
            colour,
            mesh_pipeline,
            cutout_prepass_pipeline,
            outline_pipeline,
            outline_buffer,
            prepass_pipeline,
            holed: Vec::new(),
            scene_depth_layout,
            scene_depth_bind_group,
            contact_shadows: true,
            hidden: hidden::Hidden::default(),
            asking: None,
            hidden_pipeline,
            box_mesh: None,
            asked_count: 0,
            frame: 0,
            occluding: false,
            culling: true,
            translucent_pipelines,
            meshes: Vec::new(),
            asked_fights: false,
            fights: coplanar::Fights::default(),
            body: None,
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
        // whether anything drawn with this belongs in the passes that write
        // depth without a texture bound. See spec 0047.
        self.holed.push(data.has_holes());
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
            VERTICES,
        );
        let indices = mesh_buffer(
            &self.device,
            "Mesh Indices",
            bytemuck::cast_slice(&data.indices),
            INDICES,
        );

        self.meshes.push(GpuMesh {
            vertices,
            indices,
            index_count: data.indices.len() as u32,
            bounds: data.bounds(),
            boxy: data.is_box(),
        });

        MeshId(self.meshes.len() - 1)
    }

    /// Moves the vertices of a mesh already uploaded, per spec 0042.
    ///
    /// A mesh is one upload shared by every instance of it, so this changes
    /// every push of that handle. A game wanting two shapes wants two meshes.
    ///
    /// An id no mesh was ever given is ignored. It can only come from
    /// `add_mesh`, so it is a handle kept from a renderer that is gone, and
    /// losing a shape is better than losing the frame.
    pub fn update_mesh(&mut self, id: MeshId, data: &MeshData) {
        // the device and the queue are read while a mesh is held mutably, which
        // is three fields of one struct rather than the whole of it
        let Renderer {
            device,
            queue,
            meshes,
            ..
        } = self;
        let Some(mesh) = meshes.get_mut(id.0) else {
            return;
        };

        refill(
            device,
            queue,
            &mut mesh.vertices,
            "Mesh Vertices",
            bytemuck::cast_slice(&data.vertices),
            VERTICES,
        );
        refill(
            device,
            queue,
            &mut mesh.indices,
            "Mesh Indices",
            bytemuck::cast_slice(&data.indices),
            INDICES,
        );

        mesh.index_count = data.indices.len() as u32;
        mesh.bounds = data.bounds();
        mesh.boxy = data.is_box();
    }

    /// Every pair of drawn faces in this frame that share a plane and are both
    /// on show, per spec 0048.
    ///
    /// This is the one fault in this building that a list of colliders cannot
    /// find, because fighting is a drawing fault and the colliders are not the
    /// drawing. So it reads the frame: each instance's mesh box put through its
    /// transform, every pair compared, and the pairs some third box covers
    /// thrown away.
    ///
    /// It compares every instance with every other, which is a key press and
    /// not a frame. Instances turned off the axes are counted and left out
    /// rather than measured loose; see [`coplanar::world_box`].
    pub fn fighting_faces(&self, scene: &Scene) -> coplanar::Fights {
        let mut boxes = Vec::new();
        let mut turned = 0;

        let mut shaped = 0;

        for (mesh, _, instances) in scene.batches() {
            let Some(gpu) = self.meshes.get(mesh.0) else {
                continue;
            };

            // a mesh whose box is not its faces has nothing to say here, and
            // what it says instead is loud: see `MeshData::is_box`
            if !gpu.boxy {
                shaped += instances.len();
                continue;
            }

            for instance in instances {
                let model = instance.matrix();

                match coplanar::world_box(model, gpu.bounds) {
                    Some(world) => boxes.push(world),
                    None => turned += 1,
                }
            }
        }

        coplanar::Fights::of(boxes, turned, shaped, &self.looking())
    }

    /// What this camera's depth buffer can tell apart, per spec 0048.
    pub fn looking(&self) -> crate::renderer::depth::Depth {
        crate::renderer::depth::Depth::of(&self.camera)
    }

    /// Asks for one coplanar check on the next frame drawn, per spec 0048.
    ///
    /// One frame and not every frame, because it compares every drawn instance
    /// with every other. The answer goes in the log and stays in
    /// [`Renderer::fights`], so a game can outline the patches the frame after
    /// and see where the speckle is rather than read coordinates.
    ///
    /// Give it a `body` and it also fills out from the camera to find where
    /// that body could be, per spec 0049, and throws out every pair nobody can
    /// get in front of. Without one the report is every pair, which in the
    /// arcade is 221 and mostly the tops of ceilings.
    ///
    /// The fill goes over the frame's own drawn boxes and not over the game's
    /// colliders, which is worth saying because the colliders look like the
    /// better list. A ceiling you cannot walk into does not need a collider and
    /// the arcade's hall has none, so a fill over colliders climbs out through
    /// the ceiling and the filter keeps the very faces it was meant to drop.
    /// What is drawn has every ceiling in it, because a room with no ceiling
    /// drawn is a room you can see the sky through.
    pub fn check_fighting_faces(&mut self, body: Option<crate::reach::Body>) {
        self.asked_fights = true;
        self.body = body;
    }

    /// What the last check found, per spec 0048. Empty until one is asked for.
    pub fn fights(&self) -> &coplanar::Fights {
        &self.fights
    }

    /// Runs an owed check and says what it found.
    fn answer_fights(&mut self, scene: &Scene) {
        if !self.asked_fights {
            return;
        }

        self.asked_fights = false;
        let found = self.fighting_faces(scene);
        let every = found.fights.len();

        self.fights = match self.body.take() {
            Some(body) => {
                let mut world = crate::collision::Aabb::empty();

                for box_ in &found.boxes {
                    world = world.union(box_);
                }

                // pulled in around the camera until it is a fill that can be
                // built. Handed the whole drawn world it refused and came
                // back empty, and an empty reach keeps every pair rather than
                // none: the narrowing this body is for became a no-op without
                // saying so. See `reach::fillable`.
                let within = crate::reach::Reach::fillable(
                    world.expanded(glam::Vec3::splat(1.0)),
                    self.camera.position,
                    crate::reach::STEP,
                );
                let reach = crate::reach::Reach::found(
                    self.camera.position,
                    body,
                    &found.boxes,
                    within,
                    crate::reach::STEP,
                );
                let kept = found.only_seen_from(&reach);

                log::info!(
                    "a body can be in {} cells of {:.2} over {:.0} by {:.0} by {:.0}, and {} of {} fighting pairs are in front of one",
                    reach.cells(),
                    reach.step(),
                    within.size().x,
                    within.size().y,
                    within.size().z,
                    kept.fights.len(),
                    every
                );
                kept
            }
            None => found,
        };

        let report = &self.fights;

        if report.fights.is_empty() {
            log::info!(
                "no two drawn faces share a plane in the open, over {} boxes, with {} turned off the axes and {} not box shaped, neither measured",
                report.boxes.len(),
                report.turned,
                report.shaped
            );
            return;
        }

        log::warn!(
            "{} pairs of drawn faces share a plane in the open, over {} boxes, with {} turned off the axes and {} not box shaped, neither measured",
            report.fights.len(),
            report.boxes.len(),
            report.turned,
            report.shaped
        );

        // the biggest handful and not all of them. The list is sorted by how
        // much patch there is, a building gives a couple of hundred, and a
        // report nobody reads is a check nobody ran. The rest are outlined in
        // the game, where the ones you cannot get to never show.
        for fight in report.fights.iter().take(SAID) {
            log::warn!("  {}", fight);
        }

        if report.fights.len() > SAID {
            log::warn!(
                "  and {} smaller, which are outlined rather than written out",
                report.fights.len() - SAID
            );
        }
    }

    /// What the shadow map covers. A world bigger than this gets no shadows
    /// outside it, per spec 0015; smaller bounds give sharper shadows.
    pub fn set_scene_bounds(&mut self, bounds: Aabb) {
        self.scene_bounds = bounds;
    }

    /// Whether the contact march runs. On by default; a game with nothing
    /// resting on anything can save the work. See spec 0029.
    pub fn set_contact_shadows(&mut self, on: bool) {
        self.contact_shadows = on;
    }

    /// Whether each pass leaves out what it cannot see. Spec 0057.
    pub fn set_culling(&mut self, on: bool) {
        self.culling = on;
    }

    /// Whether the picture leaves out kept runs that a wall hides. Spec
    /// 0061.
    ///
    /// Off by default: it costs a pass over the boxes and a buffer read, and
    /// saves nothing at all in a scene where nothing stands in front of
    /// anything. The game knows which it has and the engine cannot.
    pub fn set_occluding(&mut self, on: bool) {
        self.occluding = on;

        if on && self.asking.is_none() {
            self.asking = Some(hidden::Asking::new(&self.device));
        }

        if on && self.box_mesh.is_none() {
            self.box_mesh = Some(self.add_mesh(&crate::mesh::MeshData::cube()));
        }
    }

    pub fn occluding(&self) -> bool {
        self.occluding
    }

    pub fn culling(&self) -> bool {
        self.culling
    }

    pub fn contact_shadows(&self) -> bool {
        self.contact_shadows
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

        // a renderer with no window has no cursor to lock either, per spec
        // 0062: a picture test asks for nothing of the sort, and a game
        // always has one
        let Some(window) = self.window.as_ref() else {
            return false;
        };
        let grabbed = match window.set_cursor_grab(mode) {
            Ok(()) => true,
            Err(e) if locked => {
                // some platforms only confine the cursor, which is close enough
                match window.set_cursor_grab(winit::window::CursorGrabMode::Confined) {
                    Ok(()) => true,
                    Err(_) => {
                        log::warn!("could not lock the cursor: {}", e);
                        false
                    }
                }
            }
            Err(_) => false,
        };

        window.set_cursor_visible(!locked || !grabbed);
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
        log::info!("drawing {} by {}", new_size.width, new_size.height);
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        if let Some(surface) = &self.surface {
            surface.configure(&self.device, &self.config);
        }
        self.queue.write_buffer(
            &self.screen_buffer,
            0,
            bytemuck::cast_slice(&screen_size(&self.config)),
        );
        self.depth = depth::DepthTexture::new(
            &self.device,
            self.config.width,
            self.config.height,
            self.depth.samples,
        );
        self.colour = multisampled(&self.device, &self.config, self.drawn, self.depth.samples);
        // the old view died with the old texture, and the march reads through
        // this one
        self.scene_depth_bind_group =
            scene_depth_group(&self.device, &self.scene_depth_layout, &self.depth);
        self.camera.set_viewport(self.width(), self.height());
        self.text_brush
            .resize_view(self.width(), self.height(), &self.queue);
    }

    pub fn render(&mut self, scene: &Scene, geometry: &Geometry, text_renderer: &TextRenderer) {
        // before anything is asked of the surface, so a frame that turns out
        // not to draw still answers a check that was owed
        self.answer_fights(scene);

        // **Timed, because this is where a frame waits.** Asking for the
        // next surface texture blocks until the display hands one back, so a
        // frame measured around this call reports the display's pace and not
        // the engine's: every scene this machine can draw inside a frame
        // comes out at exactly one frame, however little it cost. Spec 0056.
        let asking = Instant::now();
        // a renderer with no window draws through `picture` instead, per
        // spec 0062
        let Some(surface) = self.surface.as_ref() else {
            return;
        };
        let got = surface.get_current_texture();

        self.waited = asking.elapsed().as_secs_f32();

        let asked = Asked::at(&got);

        if asked.reconfigure {
            if let Some(surface) = &self.surface {
                surface.configure(&self.device, &self.config);
            }
        }
        if asked.complain {
            log::error!("Failed to acquire surface texture: {:?}", got);
        }

        let frame = match got {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            _ => {
                // Nothing will be drawn this frame, and the frame has already
                // staged its uploads: every `write_buffer` since the last
                // submit is sitting in wgpu's staging, and staging only comes
                // back when something is submitted. Returning without
                // submitting keeps all of it, for ever.
                //
                // Which is what a window behind another window did. Nothing
                // presents, so nothing waits on a vsync either and the loop
                // runs as fast as the machine will go: the arcade staged three
                // water surfaces a frame at some thousands of frames a second
                // and grew by the better part of half a gigabyte a second. It
                // reached a hundred and sixty-six gigabytes and took the
                // machine down with it, four times over six days, because it
                // only ever happened while the window was not being looked at.
                //
                // An empty submit is the whole of the fix. It costs nothing and
                // it hands the staging back.
                if asked.must_flush() {
                    self.queue.submit([]);
                }

                return;
            }
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.drawn),
            ..Default::default()
        });

        self.draw_into(&view, scene, geometry, text_renderer);
        self.queue.present(frame);
    }

    /// Draws one frame and hands back its pixels, for a picture test. Spec
    /// 0062.
    ///
    /// Rgba8, row by row, with the padding a copy to a buffer needs taken
    /// out again, so what comes back is what a person would see.
    /// Writes every kept run into a buffer of its own, and only when the
    /// runs have changed. Spec 0063.
    ///
    /// **This is the whole point of the spec.** A world that stands still
    /// is written once and drawn out of what was written, however many
    /// frames go by and wherever the eye goes. What makes that possible is
    /// that a kept instance is stored relative to its run's middle, so
    /// nothing in this buffer depends on where anybody is standing.
    ///
    /// The whole buffer is written again when anything changes rather than
    /// the one run that did. A run is kept and dropped far less often than
    /// it is drawn, and a free list that packs runs into the gaps left by
    /// dropped ones is a thing to write when something needs it.
    fn lay_out_what_is_kept(&mut self, holding: &[&crate::renderer::scene::Keeping]) {
        let built: Vec<(u64, u64)> = holding
            .iter()
            .map(|keeping| (keeping.id, keeping.version))
            .collect();

        if built == self.kept_built {
            return;
        }

        let mut instances: Vec<Instance> = Vec::new();
        // **What a buffer on this device holds, which is where a scene
        // stops.** Spec 0069.
        let fits = (self.device.limits().max_buffer_size / Instance::SIZE) as usize;
        let lengths: Vec<usize> = holding
            .iter()
            .map(|keeping| keeping.instances.len())
            .collect();
        let (drawn, turned) = what_fits(&lengths, fits);

        self.kept_homes.clear();

        for (n, keeping) in holding.iter().enumerate().take(drawn) {
            self.kept_homes.insert(
                keeping.id,
                culling::Home {
                    base: instances.len() as u32,
                    // nought is the entry that adds nothing, so a run's own
                    // entry is one past its place in this list
                    standing: n as u32 + 1,
                },
            );
            instances.extend_from_slice(&keeping.instances);
        }

        self.turned_away = turned;

        // once, with the number, the way spec 0020 says what the lamp cap
        // refused. The renderer says it rather than `Scene`, which is
        // where every other cap's line lives, because the renderer is what
        // knows how big a buffer this device gives and it holds the scene
        // by shared reference.
        if turned > 0 && !self.said_what_did_not_fit {
            self.said_what_did_not_fit = true;

            log::warn!(
                "a scene kept {} more instances than the {} a buffer holds on this device, so \
                 that many are not being drawn. The runs are left out whole and in the order \
                 they were kept; see spec 0069",
                turned,
                fits
            );
        }

        let wanted = instances.len() as u64 * Instance::SIZE;
        let most = self.device.limits().max_buffer_size;

        if let Some(size) = instance_room(self.kept_buffer.size(), wanted, most) {
            self.kept_buffer = create_buffer(&self.device, size, wgpu::BufferUsages::VERTEX);
        }

        if !instances.is_empty() {
            self.queue
                .write_buffer(&self.kept_buffer, 0, bytemuck::cast_slice(&instances));
        }

        self.kept_built = built;

        log::info!(
            "{} kept instances in {} runs, written again",
            instances.len(),
            holding.len()
        );
    }

    /// Writes where each kept run stands this frame, which is its middle
    /// with the eye taken off. Spec 0063.
    ///
    /// This is all the CPU sends for a world that is standing still: four
    /// numbers a run rather than sixty eight bytes an instance.
    fn say_where_they_stand(
        &mut self,
        holding: &[&crate::renderer::scene::Keeping],
        eye: glam::Vec3,
    ) {
        let stride = self.standing_stride;
        let wanted = stride * (holding.len() as u64 + 1);

        if wanted > self.standing_buffer.size() {
            self.standing_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Standing"),
                size: wanted,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.camera_bind_group = standing_bind_group(
                &self.device,
                &self.camera_bind_group_layout,
                &self.camera_buffer,
                &self.standing_buffer,
            );
        }

        // one write rather than one a run, since a dynamic offset wants
        // them a stride apart and the gaps between are never read
        let mut table = vec![0u8; wanted as usize];

        for (n, keeping) in holding.iter().enumerate() {
            let at = keeping.round.at - eye;
            let entry = [at.x, at.y, at.z, 0.0];
            let at = (n as u64 + 1) * stride;

            table[at as usize..at as usize + STANDING as usize]
                .copy_from_slice(bytemuck::cast_slice(&entry));
        }

        self.queue.write_buffer(&self.standing_buffer, 0, &table);
    }

    pub fn picture(
        &mut self,
        scene: &Scene,
        geometry: &Geometry,
        text_renderer: &TextRenderer,
    ) -> Picture {
        let (width, height) = (self.config.width, self.config.height);
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Picture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[self.drawn],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.drawn),
            ..Default::default()
        });

        self.draw_into(&view, scene, geometry, text_renderer);

        // a copy to a buffer wants its rows on 256 bytes, which is not what
        // a picture is, so the padding comes off on the way out
        let row = width * 4;
        let padded =
            row.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let read = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Picture Read"),
            size: (padded * height) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Picture Encoder"),
            });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &read,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(std::iter::once(encoder.finish()));

        let landed = Arc::new(AtomicBool::new(false));
        let told = landed.clone();

        read.slice(..).map_async(wgpu::MapMode::Read, move |got| {
            told.store(got.is_ok(), Ordering::Release);
        });

        // waited for rather than polled around: a picture test has nothing
        // else to do, and a frame that never lands is a test that hangs
        // rather than a test that passes
        let _ = self.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });

        let mut pixels = Vec::with_capacity((row * height) as usize);

        if landed.load(Ordering::Acquire) {
            if let Ok(got) = read.slice(..).get_mapped_range() {
                for line in 0..height {
                    let from = (line * padded) as usize;

                    pixels.extend_from_slice(&got[from..from + row as usize]);
                }
            }
        }

        read.unmap();

        Picture {
            width,
            height,
            pixels,
        }
    }

    /// Draws one frame into a view, which is a surface's texture for a game
    /// and a texture of its own for a picture test. Spec 0062.
    ///
    /// Everything between the two is the same code, because a picture of a
    /// frame drawn some other way would be a picture of something nobody
    /// plays.
    fn draw_into(
        &mut self,
        view: &wgpu::TextureView,
        scene: &Scene,
        geometry: &Geometry,
        text_renderer: &TextRenderer,
    ) {
        let mut counted = Counted::default();

        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[SceneUniform::new(
                &self.camera,
                &scene.light,
                scene.point_lights(),
                scene.spot_lights(),
                &self.scene_bounds,
                self.contact_shadows,
            )]),
        );
        self.queue.write_buffer(
            &self.point_faces_buffer,
            0,
            bytemuck::cast_slice(&point_faces(scene.point_lights(), self.camera.position)),
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

        // **The pair the last frame wrote, resolved in this frame's
        // encoder.** Spec 0065: resolving it in the same submission that
        // wrote it gave back an end tick the GPU had not written yet, so
        // index 1 held the frame before's and the pair came out backwards.
        // A backwards pair is thrown away, which is why the GPU column was
        // blank on every frame fast enough to matter.
        if let Some(timing) = self.timing.as_mut() {
            if timing.waiting != 0 && !timing.reading {
                encoder.resolve_query_set(&timing.set, 0..Timing::QUERIES, &timing.resolved, 0);
                encoder.copy_buffer_to_buffer(&timing.resolved, 0, &timing.read, 0, Timing::BYTES);
                timing.copied = timing.waiting;
                timing.waiting = 0;
            }
        }

        // the light sees translucent things as solid, because a shadow map
        // holds a depth and has nowhere to put an alpha. It does not see a
        // glowing thing at all, per spec 0039.
        let drawn = self.upload_instances(scene);
        let outlines = self.upload_outlines(scene);

        // what the light can see, first of all. Skipped when nothing is drawn in
        // 3D: the instance buffer is empty then, and slicing an empty buffer is
        // a panic in wgpu.
        if drawn.anything {
            // the sun first, then one pass for each spot, per spec 0021. Every
            // spot's map is cleared whether or not it is in use, so a spot that
            // was switched off this frame cannot leave last frame's shadow
            // behind on its layer.
            let spots = scene
                .spot_lights()
                .len()
                .min(crate::lighting::MAX_SPOT_LIGHTS);
            let sun = crate::lighting::MAX_SPOT_LIGHTS as u32;

            // each map with the slot of the run it draws, per spec 0057: a
            // map is filled with what that light can see and not with what
            // the camera can, because a thing behind you throws its shadow
            // into the picture.
            let mut maps: Vec<(u32, &wgpu::TextureView, usize)> =
                vec![(sun, &self.shadow_view, SUN)];
            for (index, layer) in self.spot_shadow_layers.iter().enumerate() {
                maps.push((index as u32, layer, FIRST_SPOT + index));
            }

            for (which, view, slot) in maps {
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
                    timestamp_writes: timing_writes(
                        &mut self.timing,
                        if which >= sun {
                            crate::cost::SUN
                        } else {
                            crate::cost::FIRST_SPOT + which as usize
                        },
                    ),
                    occlusion_query_set: None,
                    multiview_mask: None,
                });

                // a spot with nothing in it gets its clear and no drawing,
                // which costs an empty pass and keeps the layer honest
                if which < sun && which as usize >= spots {
                    continue;
                }

                shadow_pass.set_pipeline(&self.shadow_pipeline);
                shadow_pass.set_bind_group(0, &self.camera_bind_group, &[0]);
                shadow_pass.set_bind_group(1, &self.which_bind_groups[which as usize], &[]);

                for batch in drawn.at(slot).casting.iter() {
                    let (mesh_id, texture_id, first, count) =
                        (&batch.mesh, &batch.texture, &batch.first, &batch.count);
                    // a texture with holes casts no shadow here, for the
                    // reason the prepass skips it: this pass has no texture
                    // bound, so a line of lettering would throw the shadow
                    // of the rectangle it is written on. Spec 0047.
                    if self.holed.get(texture_id.0).copied().unwrap_or(false) {
                        continue;
                    }
                    let mesh = &self.meshes[mesh_id.0];
                    if mesh.index_count == 0 {
                        continue;
                    }
                    as_it_stands(
                        &mut shadow_pass,
                        batch,
                        &self.camera_bind_group,
                        self.standing_stride,
                        &self.instance_buffer,
                        &self.kept_buffer,
                    );
                    shadow_pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                    shadow_pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                    counted.saw(mesh.index_count, *count);
                    shadow_pass.draw_indexed(0..mesh.index_count, 0, *first..(*first + *count));
                }
            }

            // then six passes for each lamp that asked to cast, per spec 0022.
            // Every layer is cleared whether a lamp claimed it or not, for the
            // same reason the spots' are.
            let faces = point_faces(scene.point_lights(), self.camera.position);
            for (layer, view) in self.point_shadow_layers.iter().enumerate() {
                // six layers to a lamp, and a lamp's six faces all see what
                // the lamp reaches. Spec 0057.
                let slot = FIRST_LAMP + layer / 6;
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
                    timestamp_writes: timing_writes(
                        &mut self.timing,
                        crate::cost::FIRST_LAMP_FACE + layer,
                    ),
                    occlusion_query_set: None,
                    multiview_mask: None,
                });

                // no lamp claimed this layer, so it gets its clear and nothing
                // else
                if faces[layer].position_range[3] <= 0.0 {
                    continue;
                }

                shadow_pass.set_pipeline(&self.point_shadow_pipeline);
                shadow_pass.set_bind_group(0, &self.camera_bind_group, &[0]);
                shadow_pass.set_bind_group(1, &self.which_bind_groups[layer], &[]);
                shadow_pass.set_bind_group(2, &self.point_faces_bind_group, &[]);

                for batch in drawn.at(slot).casting.iter() {
                    let (mesh_id, texture_id, first, count) =
                        (&batch.mesh, &batch.texture, &batch.first, &batch.count);
                    // a texture with holes casts no shadow here, for the
                    // reason the prepass skips it: this pass has no texture
                    // bound, so a line of lettering would throw the shadow
                    // of the rectangle it is written on. Spec 0047.
                    if self.holed.get(texture_id.0).copied().unwrap_or(false) {
                        continue;
                    }
                    let mesh = &self.meshes[mesh_id.0];
                    if mesh.index_count == 0 {
                        continue;
                    }
                    as_it_stands(
                        &mut shadow_pass,
                        batch,
                        &self.camera_bind_group,
                        self.standing_stride,
                        &self.instance_buffer,
                        &self.kept_buffer,
                    );
                    shadow_pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                    shadow_pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                    counted.saw(mesh.index_count, *count);
                    shadow_pass.draw_indexed(0..mesh.index_count, 0, *first..(*first + *count));
                }
            }
        }

        // depth for the whole scene first. The mesh pass reads it while it is
        // testing against it, which a pass may only do when it is not also
        // writing it, so the writing happens here. See spec 0029.
        {
            let mut prepass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Depth Prepass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: timing_writes(&mut self.timing, crate::cost::PREPASS),
                occlusion_query_set: None,
                multiview_mask: None,
            });

            // only the solid geometry: see-through things do not write depth in
            // the mesh pass either, per spec 0018
            if !drawn.picture().opaque.is_empty() {
                prepass.set_pipeline(&self.prepass_pipeline);
                prepass.set_bind_group(0, &self.camera_bind_group, &[0]);

                for batch in drawn.picture().opaque.iter() {
                    let (mesh_id, texture_id, first, count) =
                        (&batch.mesh, &batch.texture, &batch.first, &batch.count);
                    let mesh = &self.meshes[mesh_id.0];
                    // a texture with holes in it cannot write depth with this
                    // pipeline: it has no texture bound and would fill in the
                    // holes. It goes in the second run below instead, which
                    // binds the surface and cuts them. Spec 0047.
                    if mesh.index_count == 0
                        || self.holed.get(texture_id.0).copied().unwrap_or(false)
                    {
                        continue;
                    }
                    as_it_stands(
                        &mut prepass,
                        batch,
                        &self.camera_bind_group,
                        self.standing_stride,
                        &self.instance_buffer,
                        &self.kept_buffer,
                    );
                    prepass.set_vertex_buffer(0, mesh.vertices.slice(..));
                    prepass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                    counted.saw(mesh.index_count, *count);
                    prepass.draw_indexed(0..mesh.index_count, 0, *first..(*first + *count));
                }

                // and the cut-out things, bound to their surfaces so the holes
                // are discarded before the depth lands. Second rather than
                // mixed in, because this changes pipeline and binds a group
                // per batch where the run above binds nothing at all.
                prepass.set_pipeline(&self.cutout_prepass_pipeline);

                for batch in drawn.picture().opaque.iter() {
                    let (mesh_id, texture_id, first, count) =
                        (&batch.mesh, &batch.texture, &batch.first, &batch.count);
                    let mesh = &self.meshes[mesh_id.0];
                    if mesh.index_count == 0
                        || !self.holed.get(texture_id.0).copied().unwrap_or(false)
                    {
                        continue;
                    }
                    prepass.set_bind_group(1, &self.textures[texture_id.0], &[]);
                    as_it_stands(
                        &mut prepass,
                        batch,
                        &self.camera_bind_group,
                        self.standing_stride,
                        &self.instance_buffer,
                        &self.kept_buffer,
                    );
                    prepass.set_vertex_buffer(0, mesh.vertices.slice(..));
                    prepass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                    counted.saw(mesh.index_count, *count);
                    prepass.draw_indexed(0..mesh.index_count, 0, *first..(*first + *count));
                }
            }
        }

        // and then, with the solid world's depth laid down, the boxes of
        // the kept runs: drawn tested and not written, each inside a query,
        // so the GPU can say which of them has nothing of itself in front
        // of what is already there. Spec 0061.
        if self.asked_count > 0 {
            let mut hiding = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("What Is Hidden"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: timing_writes(&mut self.timing, crate::cost::HIDDEN),
                occlusion_query_set: self.asking.as_ref().map(|asking| &asking.set),
                multiview_mask: None,
            });

            if let (Some(asking), Some(cube)) = (self.asking.as_ref(), self.box_mesh) {
                let mesh = &self.meshes[cube.0];

                hiding.set_pipeline(&self.hidden_pipeline);
                hiding.set_bind_group(0, &self.camera_bind_group, &[0]);
                hiding.set_vertex_buffer(0, mesh.vertices.slice(..));
                hiding.set_vertex_buffer(1, asking.boxes.slice(..));
                hiding.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);

                for n in 0..self.asked_count {
                    hiding.begin_occlusion_query(n);
                    hiding.draw_indexed(0..mesh.index_count, 0, n..(n + 1));
                    hiding.end_occlusion_query();
                }
            }
        }

        // the world next, depth tested, then the interface painted on top
        {
            // into the multisampled texture and resolved onto the surface at
            // the end of this pass, per spec 0052. The interface is painted on
            // afterwards, onto what came out, because text is already smoothed
            // by its own atlas and a quad has no edge worth four samples.
            let (into, onto) = match &self.colour {
                Some(many) => (many, Some(view)),
                None => (view, None),
            };
            let mut mesh_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Mesh Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: into,
                    depth_slice: None,
                    resolve_target: onto,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                // read only: the prepass settled it, and a pass that wrote it
                // could not also be sampling it
                // read only, and it has to stay that way: this pass samples
                // the same depth texture for spec 0029, and wgpu will not
                // have a texture written as an attachment and read as a
                // resource in one pass. That is why the cut-out things write
                // their depth back in the prepass instead. Spec 0047.
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth.view,
                    depth_ops: None,
                    stencil_ops: None,
                }),
                timestamp_writes: timing_writes(&mut self.timing, crate::cost::MESH),
                occlusion_query_set: None,
                multiview_mask: None,
            });

            // This pass clears the frame, so it runs even with nothing 3D in
            // it. Only the drawing waits on an instance buffer that exists.
            if drawn.anything {
                mesh_pass.set_bind_group(0, &self.camera_bind_group, &[0]);

                // solid first, so it is in the depth buffer before anything is
                // blended over it. Then the far side of everything see-through,
                // then the near side, per spec 0018.
                let runs = [
                    (&self.mesh_pipeline, &drawn.picture().opaque),
                    (&self.translucent_pipelines[0], &drawn.picture().translucent),
                    (&self.translucent_pipelines[1], &drawn.picture().translucent),
                ];

                for (pipeline, batches) in runs {
                    if batches.is_empty() {
                        continue;
                    }
                    mesh_pass.set_pipeline(pipeline);

                    for batch in batches.iter() {
                        let (mesh_id, texture_id, first, count) =
                            (&batch.mesh, &batch.texture, &batch.first, &batch.count);
                        let mesh = &self.meshes[mesh_id.0];
                        if mesh.index_count == 0 {
                            continue;
                        }
                        mesh_pass.set_bind_group(1, &self.textures[texture_id.0], &[]);
                        mesh_pass.set_bind_group(2, &self.shadow_bind_group, &[]);
                        mesh_pass.set_bind_group(3, &self.scene_depth_bind_group, &[]);
                        as_it_stands(
                            &mut mesh_pass,
                            batch,
                            &self.camera_bind_group,
                            self.standing_stride,
                            &self.instance_buffer,
                            &self.kept_buffer,
                        );
                        mesh_pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                        mesh_pass
                            .set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                        counted.saw(mesh.index_count, *count);
                        mesh_pass.draw_indexed(0..mesh.index_count, 0, *first..(*first + *count));
                    }
                }
            }

            // and the colliders over the top of it, per spec 0046. Last, so
            // they are over everything they are being compared against, and
            // tested, so one behind a wall is hidden.
            if outlines > 0 {
                mesh_pass.set_pipeline(&self.outline_pipeline);
                mesh_pass.set_bind_group(0, &self.camera_bind_group, &[0]);
                mesh_pass.set_vertex_buffer(0, self.outline_buffer.slice(..));
                counted.draws += 1;
                mesh_pass.draw(0..outlines, 0..1);
            }
        }

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: timing_writes(&mut self.timing, crate::cost::OVERLAY),
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
                counted.saw(num_indices, 1);
                render_pass.draw_indexed(0..num_indices, 0, 0..1);
            }

            self.text_brush.draw(&mut render_pass);
            counted.draws += 1;
        }

        // the queries into a buffer, and that buffer into one that can be
        // mapped. Both in this encoder, before it is submitted: a resolve
        // after the submit is a resolve of the next frame's answers. Spec
        // 0061.
        if self.asked_count > 0 {
            if let Some(asking) = self.asking.as_mut() {
                encoder.resolve_query_set(&asking.set, 0..self.asked_count, &asking.resolved, 0);
                encoder.copy_buffer_to_buffer(
                    &asking.resolved,
                    0,
                    &asking.read,
                    0,
                    self.asked_count as u64 * 8,
                );
                asking.about = (self.hidden.asked().to_vec(), self.frame);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        self.counted = counted;

        if let Some(timing) = self.timing.as_mut() {
            if timing.started {
                timing.started = false;
                timing.waiting = timing.wrote;
                timing.wrote = 0;
            }
        }

        self.read_the_clock();
        self.read_what_is_hidden();
        self.frame += 1;
    }

    /// Takes whatever the depth buffer has said about the boxes, and asks
    /// for the next lot. Spec 0061.
    ///
    /// One read in flight at a time, the way spec 0056 reads the clock: a
    /// second would be written over the one being read. A frame whose answer
    /// has not landed knows nothing new, and knowing nothing means drawing
    /// everything.
    fn read_what_is_hidden(&mut self) {
        let Some(asking) = self.asking.as_mut() else {
            return;
        };

        if !asking.reading {
            if asking.about.0.is_empty() {
                return;
            }

            let landed = asking.landed.clone();

            asking
                .read
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |got| {
                    landed.store(got.is_ok(), Ordering::Release);
                });
            asking.reading = true;

            return;
        }

        // the callback only runs while the device is polled
        let _ = self.device.poll(wgpu::PollType::Poll);

        let Some(asking) = self.asking.as_mut() else {
            return;
        };

        if asking.reading && asking.landed.swap(false, Ordering::AcqRel) {
            let answers: Option<(Vec<u64>, Vec<u64>, u64)> =
                asking.read.slice(..).get_mapped_range().ok().map(|got| {
                    (
                        asking.about.0.clone(),
                        bytemuck::cast_slice::<u8, u64>(&got).to_vec(),
                        asking.about.1,
                    )
                });

            if let Some((about, samples, frame)) = answers {
                self.hidden.told(&about, &samples, frame);
            }

            if let Some(asking) = self.asking.as_mut() {
                asking.read.unmap();
                asking.reading = false;
                asking.about = (Vec::new(), 0);
            }
        }
    }

    /// What the last frame's drawing came to. Spec 0056.
    pub fn counted(&self) -> Counted {
        self.counted
    }

    /// Told by the loop what the frame's own work came to, in seconds.
    ///
    /// **Not the time from one frame to the next, which is the display's.**
    /// On this machine a window is redrawn when the display says so, and
    /// asking the surface not to wait does not change that: a scene that
    /// costs a millisecond and one that costs seven both arrive 8.33
    /// milliseconds apart, and the gap only moves once the work overruns a
    /// frame, in steps of a whole frame. Measured between frames, this
    /// engine would look flat up to the point where it looks like a
    /// staircase, and neither is a reading of the engine.
    ///
    /// So the loop times itself: everything from the top of the frame to the
    /// end of the submit. That is what the engine costs to use, and it moves
    /// smoothly with the scene.
    pub fn worked(&mut self, seconds: f32) {
        self.worked = (seconds - self.waited).max(0.0);
    }

    /// What that came to, with the wait for the display taken out of it.
    pub fn work_seconds(&self) -> f32 {
        self.worked
    }

    /// Told by the loop how long it has been since the previous frame, with
    /// no cap on it. Spec 0056.
    pub fn frame_took(&mut self, seconds: f32) {
        self.took = seconds;
    }

    /// What that came to, which is the number a person actually feels and on
    /// a machine that paces redraws is the display's rather than the
    /// engine's until the work overruns a frame.
    pub fn frame_seconds(&self) -> f32 {
        self.took
    }

    /// And how much of the frame was that wait, which is the display's number
    /// rather than the engine's and is most of a frame on anything this
    /// machine can draw comfortably.
    pub fn wait_seconds(&self) -> f32 {
        self.waited
    }

    /// How many samples a pixel is drawn from, which dominates the fill and
    /// therefore most of what the GPU's own clock reports.
    pub fn samples(&self) -> u32 {
        self.depth.samples
    }

    /// And how big the surface is, for the same reason.
    pub fn surface_size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    /// What the GPU said its own last frame took, in seconds, where the
    /// device will say at all. Spec 0056.
    ///
    /// The first pass of the frame beginning to the last one ending. **Not
    /// the passes added up**, which spec 0067 found can come to more than
    /// the frame did: a pass's pair says when it began and ended, and at
    /// four million instances those came to 23 ms inside a 16 ms frame.
    /// [`Self::gpu_passes`] is for weighing passes against each other and
    /// not for adding up.
    ///
    /// Some frames behind what is on the screen, because a timestamp is
    /// written by the GPU and cannot be read until the work has run. What
    /// comes back is a real frame from a moment ago, which for a number
    /// averaged over a window of them is the same number.
    pub fn gpu_seconds(&self) -> Option<f32> {
        self.timing.as_ref().and_then(|timing| timing.last)
    }

    /// What each pass of that frame took, by name. Spec 0067.
    ///
    /// Only the passes that ran: a spot nobody claimed and the occlusion
    /// pass with the dial off are not in here at all, rather than in here
    /// as a nought or as somebody else's time.
    ///
    /// The lamps are six passes each and the spots one each, and they come
    /// back under one name apiece with their times added, because twelve
    /// columns that each read a fifth of a millisecond say less than one
    /// that reads two and a half.
    ///
    /// **These weigh passes against each other and do not add up to a
    /// frame.** See [`Self::gpu_seconds`] for why.
    pub fn gpu_passes(&self) -> Vec<(&'static str, f32)> {
        let Some(timing) = self.timing.as_ref() else {
            return Vec::new();
        };
        let mut out: Vec<(&'static str, f32)> = Vec::new();

        for (slot, seconds) in timing.passes.iter().enumerate() {
            let Some(seconds) = seconds else {
                continue;
            };
            let name = crate::cost::pass_name(slot);

            match out.iter_mut().find(|(called, _)| *called == name) {
                Some((_, already)) => *already += seconds,
                None => out.push((name, *seconds)),
            }
        }

        out
    }

    /// Whether this device agreed to time a frame at all, which is a
    /// different thing from whether a reading has landed yet. Spec 0056.
    pub fn times_frames(&self) -> bool {
        self.timing.is_some()
    }

    /// Ask for the pair of ticks, or pick them up if they have landed.
    ///
    /// One read in flight at a time. A second would be a second pair of
    /// timestamps written over the pair being read, and the answer would be
    /// two halves of different frames.
    fn read_the_clock(&mut self) {
        let Some(timing) = self.timing.as_mut() else {
            return;
        };

        if !timing.reading {
            if timing.copied == 0 {
                return;
            }

            let landed = timing.landed.clone();

            timing
                .read
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |got| {
                    landed.store(got.is_ok(), Ordering::Release);
                });
            timing.reading = true;
            timing.read_of = timing.copied;
            timing.copied = 0;
        }

        // the callback only runs while the device is polled, and nothing else
        // here polls it
        let _ = self.device.poll(wgpu::PollType::Poll);

        let Some(timing) = self.timing.as_mut() else {
            return;
        };

        if timing.reading && timing.landed.swap(false, Ordering::AcqRel) {
            if let Ok(got) = timing.read.slice(..).get_mapped_range() {
                let ticks: &[u64] = bytemuck::cast_slice(&got);
                let read_of = timing.read_of;
                let period = timing.period;
                // the frame is the first pass beginning to the last one
                // ending, which is what there is once each pass owns its
                // own pair. Spec 0067.
                let mut first = u64::MAX;
                let mut last = 0u64;

                for slot in 0..crate::cost::PASSES {
                    if read_of & (1 << slot) == 0 {
                        timing.passes[slot] = None;

                        continue;
                    }

                    let (began, ended) = (ticks[slot * 2], ticks[slot * 2 + 1]);

                    // a pair that went backwards is a pass the device gave
                    // up on rather than one that took no time
                    timing.passes[slot] =
                        (ended > began).then(|| (ended - began) as f32 * period * 1e-9);

                    if ended > began {
                        first = first.min(began);
                        last = last.max(ended);
                    }
                }

                // **The frame is the span, and the parts do not add up to
                // it.** A pass's pair is when that pass began and ended on
                // the device, which is not the same as how long it had the
                // device to itself: at four million instances the passes
                // here come to 23 ms inside a frame of 16, so they overlap
                // or they count waiting. What they are good for is weight,
                // which is the question this spec was written to answer,
                // and the span is what a frame cost.
                timing.last = (last > first).then(|| (last - first) as f32 * period * 1e-9);
                timing.span = timing.last;
            }

            timing.read.unmap();
            timing.reading = false;
        }
    }

    /// Whether the surface is waiting for the display, which is what a
    /// readout has to say before any number it prints means anything.
    pub fn waiting_for_the_display(&self) -> bool {
        self.config.present_mode == wgpu::PresentMode::Fifo
    }

    /// Packs every instance into one buffer and says where each mesh's run
    /// starts, so each mesh is one instanced draw. The opaque runs come back
    /// separately from the translucent ones, because they are drawn in
    /// different passes, per spec 0018, and the casting runs separately again,
    /// because a glowing thing is drawn but throws nothing, per spec 0039.
    ///
    /// A run is one of four kinds, so the instances of a kind lie together and
    /// a kind is a span rather than a list of indices.
    fn upload_instances(&mut self, scene: &Scene) -> Drawn {
        let views = self.views(scene);
        let batches = scene.batches();
        let mut pushed: Vec<culling::Pushed> = Vec::with_capacity(batches.len());

        for (mesh, texture, instances) in batches {
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

            pushed.push(culling::Pushed {
                mesh,
                texture,
                bounds: self.meshes[mesh.0].bounds,
                instances,
                quarters: None,
                hidden: false,
                round: None,
                home: None,
            });
        }

        // and what the game handed over once rather than this frame, per
        // spec 0058: laid out in the buffer first, so a run that nothing has
        // touched is already where it needs to be and this frame writes
        // nothing for it. Spec 0059.
        let holding: Vec<&crate::renderer::scene::Keeping> = scene
            .keepings()
            .filter(|keeping| {
                if keeping.mesh.0 >= self.meshes.len() {
                    log::warn!(
                        "a scene kept mesh {:?}, which was never added",
                        keeping.mesh
                    );
                    return false;
                }
                if keeping.texture.0 >= self.textures.len() {
                    log::warn!(
                        "a scene kept texture {:?}, which was never added",
                        keeping.texture
                    );
                    return false;
                }

                true
            })
            .collect();

        // what to ask the depth buffer about this frame, and the boxes to
        // ask with. Everything kept is asked, including what is already
        // known to be hidden, or a run that goes behind a wall once never
        // comes back. Spec 0061.
        self.asked_count = self.ask_what_is_hidden(&holding);

        let frame = self.frame;
        let eye = self.camera.position;

        // where the kept runs lie, written only when they change, and the
        // table that says where each of them stands this frame. Spec 0063.
        self.lay_out_what_is_kept(&holding);
        self.say_where_they_stand(&holding, eye);

        let kept: Vec<culling::Pushed> = holding
            .iter()
            .map(|keeping| {
                let mut kept = culling::of_a_keeping(
                    keeping,
                    self.meshes[keeping.mesh.0].bounds,
                    self.kept_homes.get(&keeping.id).copied(),
                );

                kept.hidden = self.occluding && self.hidden.is_hidden(keeping.id, frame);

                kept
            })
            .collect();

        let (instances, drawn) = culling::pick(&views, &pushed, &kept, eye);

        // **Written again every frame, kept or pushed.** What reaches the
        // GPU has the eye taken out of it, per spec 0051, and the eye moves.
        // Spec 0059 moved that subtraction into the vertex stage so a kept
        // run could be written once and left alone, and the sun's shadows
        // went with it; the subtraction is back on the CPU and so is this.
        let wanted = instances.len() as u64 * Instance::SIZE;

        let most = self.device.limits().max_buffer_size;

        if let Some(size) = instance_room(self.instance_buffer.size(), wanted, most) {
            if wanted > most {
                log::warn!(
                    "a scene wants {} bytes of instances and this device will give {}",
                    wanted,
                    most
                );
            }

            self.instance_buffer = create_buffer(&self.device, size, wgpu::BufferUsages::VERTEX);
        }

        if !instances.is_empty() {
            self.queue
                .write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances));
        }

        drawn
    }

    /// Lines up this frame's boxes for the depth buffer to answer about,
    /// and says how many there are. Spec 0061.
    ///
    /// A run's box is the sphere that holds it, drawn as the cube that holds
    /// that: bigger than the run, which is the right way round, since a box
    /// that is too big can only be found less hidden than the thing inside
    /// it.
    fn ask_what_is_hidden(&mut self, holding: &[&crate::renderer::scene::Keeping]) -> u32 {
        if !self.occluding {
            return 0;
        }

        // **A query that is never written answers nought, and nought means
        // hidden.** If the pass below cannot draw the boxes, because the
        // cube to draw them with or the query set itself is not there yet,
        // then nothing is asked at all: a resolve of queries nobody wrote
        // reads as a whole world behind a wall, which is what it did the
        // first time this ran. Spec 0061.
        if self.box_mesh.is_none() {
            return 0;
        }

        let Some(asking) = self.asking.as_mut() else {
            return 0;
        };

        // nothing new is asked while the last answer is still coming back,
        // for the reason spec 0056 reads the clock one frame at a time: the
        // answers being read would be written over by the frame reading
        // them
        if asking.reading {
            return 0;
        }

        let living: std::collections::HashSet<u64> =
            holding.iter().map(|keeping| keeping.id).collect();

        self.hidden.forget(&living);

        // **A run the eye is standing inside is not asked about.** Its box
        // has every part of itself either behind the near plane, where it is
        // clipped away, or behind the thing the box is for: nothing passes,
        // and nothing passing means hidden, so the wall you are looking at
        // is not drawn. Spec 0061, and it took a wall out of the picture
        // before it was written down.
        let eye = self.camera.position;
        let near = self.camera.near;
        let inside: Vec<u64> = holding
            .iter()
            .filter(|keeping| eye.distance(keeping.round.at) <= keeping.round.reach + near * 2.0)
            .map(|keeping| keeping.id)
            .collect();

        for id in inside.iter() {
            self.hidden.forget_one(*id);
        }

        let asking_about: Vec<&crate::renderer::scene::Keeping> = holding
            .iter()
            .filter(|keeping| !inside.contains(&keeping.id))
            .copied()
            .collect();

        let asked = self
            .hidden
            .ask(
                asking_about.iter().map(|keeping| keeping.id),
                hidden::Asking::ROOM,
            )
            .to_vec();
        let boxes: Vec<Instance> = asking_about
            .iter()
            .take(asked.len())
            .map(|keeping| {
                // the eye comes off here as it does for everything else that
                // reaches the buffer, per spec 0051
                Instance::new(
                    &crate::mesh::Transform::at(keeping.round.at)
                        .with_scale(glam::Vec3::splat(keeping.round.reach * 2.0)),
                    glam::Vec4::ONE,
                    Instance::DEFAULT_SHININESS,
                )
                .from_the_eye(eye)
            })
            .collect();

        if boxes.is_empty() {
            return 0;
        }

        self.queue
            .write_buffer(&asking.boxes, 0, bytemuck::cast_slice(&boxes));

        boxes.len() as u32
    }

    /// What each pass of this frame can see. Spec 0057.
    ///
    /// One for every slot, every frame, in the same order: the picture, the
    /// sun, the four spots, then the lamps that cast. A map no light claimed
    /// gets a view that keeps nothing, so the slots never shift and its pass
    /// is skipped anyway.
    ///
    /// With culling off there is one view holding everything and every pass
    /// reads it, which is one copy of the frame in the buffer and what the
    /// engine did before this.
    fn views(&self, scene: &Scene) -> Vec<Seen> {
        if !self.culling {
            return vec![Seen::All];
        }

        let mut views = vec![Seen::Nothing; VIEWS];

        views[PICTURE] = Seen::Through(self.camera.frustum());
        // the sun's map covers the bounds the game gave the renderer, so its
        // frustum is the one the shader is already using
        views[SUN] = Seen::Through(crate::camera::Frustum::of(shadow::light_view_projection(
            scene.light.direction,
            &self.scene_bounds,
        )));

        for (n, spot) in scene
            .spot_lights()
            .iter()
            .take(crate::lighting::MAX_SPOT_LIGHTS)
            .enumerate()
        {
            views[FIRST_SPOT + n] =
                Seen::Through(crate::camera::Frustum::of(spot.view_projection()));
        }

        // and a lamp sees as far as it reaches, which is all six faces of
        // its map at once
        let lamps = scene.point_lights();

        for (n, index) in crate::lighting::casting_lamps(lamps)
            .into_iter()
            .take(crate::lighting::MAX_SHADOWING_POINT_LIGHTS)
            .enumerate()
        {
            views[FIRST_LAMP + n] = Seen::Round {
                at: lamps[index].position,
                reach: lamps[index].range,
            };
        }

        views
    }

    /// Writes this frame's outlines into their buffer, per spec 0046, and
    /// says how many vertices went in.
    fn upload_outlines(&mut self, scene: &Scene) -> u32 {
        let mut vertices: Vec<outline::Vertex> = Vec::new();

        for one in scene.outlines() {
            vertices.extend_from_slice(&outline::vertices(one, self.camera.position));
        }

        if vertices.is_empty() {
            return 0;
        }

        let bytes: &[u8] = bytemuck::cast_slice(&vertices);
        if bytes.len() as u64 > self.outline_buffer.size() {
            self.outline_buffer =
                create_buffer_init(&self.device, bytes, wgpu::BufferUsages::VERTEX);
        } else {
            self.queue.write_buffer(&self.outline_buffer, 0, bytes);
        }

        vertices.len() as u32
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
    fn new(light: &crate::lighting::PointLight, eye: glam::Vec3) -> Self {
        Self {
            position_range: (light.position - eye)
                .extend(light.range.max(0.0))
                .to_array(),
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
    fn new(light: &crate::lighting::SpotLight, eye: glam::Vec3) -> Self {
        let (cos_outer, cos_inner) = light.cone_cosines();
        let direction = light.direction.normalize_or_zero();
        let direction = if direction.length_squared() < 0.5 {
            glam::Vec3::NEG_Y
        } else {
            direction
        };
        let range = light.range.max(0.0);

        Self {
            position_range: (light.position - eye).extend(range).to_array(),
            direction_cos_outer: direction.extend(cos_outer).to_array(),
            color_intensity: light.color.extend(light.intensity).to_array(),
            cos_inner: [cos_inner, 0.0, 0.0, 0.0],
            // worked out in the world and then told where the world went, per
            // spec 0051: a matrix that took a world position takes an
            // eye-relative one by letting the eye back in first. The world
            // one is the light's own, so the map this fills and the frustum
            // spec 0057 culls it against are the same shape.
            view_projection: (light.view_projection() * glam::Mat4::from_translation(eye))
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
fn point_faces(
    lamps: &[crate::lighting::PointLight],
    eye: glam::Vec3,
) -> [GpuPointFace; POINT_SHADOW_LAYERS] {
    let mut faces = [GpuPointFace::default(); POINT_SHADOW_LAYERS];

    for (slot, index) in crate::lighting::casting_lamps(lamps)
        .into_iter()
        .enumerate()
    {
        let lamp = &lamps[index];
        let range = lamp.range.max(0.0);
        for face in 0..6 {
            faces[slot * 6 + face] = GpuPointFace {
                view_projection: (shadow::face_view_projection(lamp.position, face, range)
                    * glam::Mat4::from_translation(eye))
                .to_cols_array(),
                position_range: (lamp.position - eye).extend(range).to_array(),
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
    /// x is one when the contact march runs and zero when it does not, y and z
    /// are the camera's near and far planes, and w is how far the march goes.
    /// The planes are here because the depth buffer holds clip depth, and
    /// nothing can be compared in world units until that is undone. See spec
    /// 0029.
    contact: [f32; 4],
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
        contact_shadows: bool,
    ) -> Self {
        // everything with a position in it is handed over relative to this,
        // per spec 0051, and every matrix that used to take a world position
        // is told where the world went by letting the eye back in first
        let eye = camera.position;

        // the scene caps this already, per spec 0020, but the array is fixed
        // and reading past it would be a different kind of bug
        let used = lamps.len().min(crate::lighting::MAX_POINT_LIGHTS);
        let mut point_lights = [GpuPointLight::default(); crate::lighting::MAX_POINT_LIGHTS];
        for (slot, lamp) in point_lights.iter_mut().zip(&lamps[..used]) {
            *slot = GpuPointLight::new(lamp, eye);
        }

        let lit = spots.len().min(crate::lighting::MAX_SPOT_LIGHTS);
        let mut spot_lights = [GpuSpotLight::default(); crate::lighting::MAX_SPOT_LIGHTS];
        for (slot, spot) in spot_lights.iter_mut().zip(&spots[..lit]) {
            *slot = GpuSpotLight::new(spot, eye);
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
            view_projection: camera.view_projection_from_the_eye().to_cols_array(),
            // the eye is the origin now, so this is where it is. Kept rather
            // than taken out because the shader asks it for the direction to
            // the viewer and nought is the honest answer. Spec 0051.
            camera_position: glam::Vec4::ZERO.to_array(),
            light_direction: light.direction.normalize_or_zero().extend(0.0).to_array(),
            light_color: light.color.extend(light.intensity).to_array(),
            ambient: light.ambient.extend(0.0).to_array(),
            light_view_projection: (shadow::light_view_projection(light.direction, bounds)
                * glam::Mat4::from_translation(eye))
            .to_cols_array(),
            point_light_count: [used as u32, 0, 0, 0],
            point_lights,
            spot_light_count: [lit as u32, 0, 0, 0],
            spot_lights,
            point_shadow,
            contact: [
                if contact_shadows { 1.0 } else { 0.0 },
                camera.near,
                camera.far,
                crate::contact::CONTACT_REACH,
            ],
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

/// The outlines, per spec 0046: lines, unlit, over the finished scene.
fn create_outline_pipeline(
    device: &wgpu::Device,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
    samples: u32,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("../../res/shaders/outline.wgsl"));

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Outline Pipeline Layout"),
        bind_group_layouts: &[Some(camera_bind_group_layout)],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Outline Pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Some(outline::Vertex::DESC)],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        // lines, and no culling: a line has no side to be the back of
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::LineList,
            ..Default::default()
        },
        depth_stencil: Some(depth::outline_state()),
        multisample: wgpu::MultisampleState {
            count: samples,
            ..Default::default()
        },
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

/// The scene's depth, as something a shader can read. Rebuilt whenever the
/// texture behind it is, which is every resize.
fn scene_depth_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    depth: &depth::DepthTexture,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Scene Depth Bind Group"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&depth.view),
        }],
    })
}

/// Depth and nothing else, so the mesh pass has the whole scene to read. The
/// same vertex stage as the mesh pipeline, so the depths it writes are the
/// depths that pass will test against. See spec 0029.
fn create_prepass_pipeline(
    device: &wgpu::Device,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    samples: u32,
) -> wgpu::RenderPipeline {
    let shader = mesh_shader(device, samples);

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Depth Prepass Pipeline Layout"),
        bind_group_layouts: &[Some(camera_bind_group_layout)],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Depth Prepass Pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Some(crate::mesh::Vertex::DESC), Some(Instance::DESC)],
            compilation_options: Default::default(),
        },
        fragment: None,
        primitive: depth::mesh_primitive_state(),
        depth_stencil: Some(depth::state()),
        multisample: wgpu::MultisampleState {
            count: samples,
            ..Default::default()
        },
        multiview_mask: None,
        cache: None,
    })
}

/// The pipeline that asks whether a box is hidden. Spec 0061.
///
/// The mesh shader's own vertex stage, which is what the prepass uses too,
/// with no fragment stage and no depth writing: what comes out of this is
/// not a picture but a count of samples that passed the depth test, which
/// the pass collects in an occlusion query. Writing depth would let a box
/// hide the box behind it and answer a question nobody asked.
fn create_hidden_pipeline(
    device: &wgpu::Device,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    samples: u32,
) -> wgpu::RenderPipeline {
    let shader = mesh_shader(device, samples);

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Hidden Pipeline Layout"),
        bind_group_layouts: &[Some(camera_bind_group_layout)],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Hidden Pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Some(crate::mesh::Vertex::DESC), Some(Instance::DESC)],
            compilation_options: Default::default(),
        },
        fragment: None,
        primitive: wgpu::PrimitiveState {
            // both faces, because a box the camera is inside has only its
            // far faces toward you and a box nobody can see from outside is
            // not hidden
            cull_mode: None,
            ..depth::mesh_primitive_state()
        },
        depth_stencil: Some(depth::outline_state()),
        multisample: wgpu::MultisampleState {
            count: samples,
            ..Default::default()
        },
        multiview_mask: None,
        cache: None,
    })
}

/// The prepass again, with the surface bound so the holes can be cut out of
/// the depth it writes. Spec 0047.
///
/// It exists because the plain prepass binds no texture and so would lay down
/// depth over a cut-out surface's holes, and because the mesh pass cannot
/// write depth at all: it samples the depth texture for spec 0029, and wgpu
/// refuses a texture written as an attachment and read as a resource in one
/// pass. So the one place a hole can be cut before the depth is written is
/// here, in a pass that reads no depth.
///
/// Without it, a surface with holes wrote no depth anywhere. It was tested
/// against everything and invisible to every test, so whatever was drawn
/// after it painted over it however far away that was: Saturn's rings were
/// cut away wherever a column stood behind them, a word on a sign was rubbed
/// out by the wall behind the sign, and the arcade's labels disappeared
/// outright at any distance with a star wall behind them.
///
/// No colour targets. This writes depth and nothing else.
fn create_cutout_prepass_pipeline(
    device: &wgpu::Device,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    texture_bind_group_layout: &wgpu::BindGroupLayout,
    samples: u32,
) -> wgpu::RenderPipeline {
    let shader = mesh_shader(device, samples);

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Cutout Prepass Pipeline Layout"),
        bind_group_layouts: &[
            Some(camera_bind_group_layout),
            Some(texture_bind_group_layout),
        ],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Cutout Prepass Pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Some(crate::mesh::Vertex::DESC), Some(Instance::DESC)],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_cutout"),
            targets: &[],
            compilation_options: Default::default(),
        }),
        primitive: depth::mesh_primitive_state(),
        depth_stencil: Some(depth::state()),
        multisample: wgpu::MultisampleState {
            count: samples,
            ..Default::default()
        },
        multiview_mask: None,
        cache: None,
    })
}

/// What a mesh pipeline binds, in the order the shader declares them: the
/// camera, the surface, the shadow maps, and spec 0029's scene depth.
struct MeshLayouts<'a> {
    camera: &'a wgpu::BindGroupLayout,
    texture: &'a wgpu::BindGroupLayout,
    shadow: &'a wgpu::BindGroupLayout,
    scene_depth: &'a wgpu::BindGroupLayout,
}

fn create_mesh_pipeline(
    device: &wgpu::Device,
    color_format: wgpu::TextureFormat,
    layouts: MeshLayouts,
    primitive: wgpu::PrimitiveState,
    depth_stencil: wgpu::DepthStencilState,
    samples: u32,
) -> wgpu::RenderPipeline {
    let shader = mesh_shader(device, samples);

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Mesh Pipeline Layout"),
        bind_group_layouts: &[
            Some(layouts.camera),
            Some(layouts.texture),
            Some(layouts.shadow),
            Some(layouts.scene_depth),
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
        multisample: wgpu::MultisampleState {
            count: samples,
            ..Default::default()
        },
        multiview_mask: None,
        cache: None,
    })
}

/// A vertex or index buffer for a mesh. Empty contents get four zero bytes
/// instead, because `create_buffer_init` panics on nothing at all and an empty
/// mesh is something a game is allowed to hand over.
/// What a mesh's two buffers are for. `COPY_DST` is what lets spec 0042 write
/// new vertices into a buffer already there rather than making another.
const VERTICES: wgpu::BufferUsages = wgpu::BufferUsages::VERTEX.union(wgpu::BufferUsages::COPY_DST);
const INDICES: wgpu::BufferUsages = wgpu::BufferUsages::INDEX.union(wgpu::BufferUsages::COPY_DST);

/// How big a buffer must become to take `want` bytes when it holds `have` now,
/// or `None` when the one already there will do.
///
/// Never shrinks. A surface stepped every frame is the same size every frame,
/// so the first upload is the only allocation; a mesh that grows and shrinks
/// keeps the largest buffer it ever needed, which is a little memory against
/// never allocating twice for the same shape.
fn room(have: u64, want: u64) -> Option<u64> {
    (want > have).then_some(want)
}

/// The same question for the instance buffer, which differs in two ways: it
/// asks for twice what it needs, and there is a ceiling.
///
/// **Twice**, so a scene that grows by a little every frame is not allocated
/// again every frame. **A ceiling**, because doubling is what killed the
/// frame at a million and a half instances: the buffer asked for more than
/// the 256 MB a device usually gives and the allocation failed. `most` is
/// what this device said it would give.
///
/// A scene that wants more than `most` on its own is asked for anyway. The
/// device refuses it and says so, which is the honest end of it; quietly
/// handing back a smaller buffer would draw part of a scene and call it a
/// frame. Spec 0060.
/// How many of a scene's runs fit in a buffer that holds `fits`
/// instances, and how many instances the rest come to. Spec 0069.
///
/// **A run is left out whole or not at all.** A run cut in half is a
/// building with one wall, and nothing downstream can tell that from a
/// building. In the order they were kept, which is the order a game handed
/// them over: leaving out the furthest, or the smallest, or the least
/// recently seen is a policy, and a policy the engine invents is one a
/// game cannot predict.
///
/// Everything after the first run that does not fit is left out too, even
/// a small one that would have. Packing the gaps would make what is drawn
/// depend on the sizes of what is not, which is the kind of rule a person
/// cannot hold in their head while looking at a hole in their world.
fn what_fits(lengths: &[usize], fits: usize) -> (usize, usize) {
    let mut taken = 0;
    let mut drawn = 0;

    for length in lengths {
        if taken + length > fits {
            break;
        }

        taken += length;
        drawn += 1;
    }

    (drawn, lengths[drawn..].iter().sum())
}

fn instance_room(have: u64, want: u64, most: u64) -> Option<u64> {
    room(have, want).map(|want| want.max(have * 2).min(most))
}

/// Puts `contents` in `buffer`, replacing it when it is too small.
fn refill(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &mut wgpu::Buffer,
    label: &str,
    contents: &[u8],
    usage: wgpu::BufferUsages,
) {
    match room(buffer.size(), contents.len() as u64) {
        // nothing to write is not nothing to do: the mesh keeps its buffer and
        // the caller drops its index count to zero, which draws nothing
        None if contents.is_empty() => (),
        None => queue.write_buffer(buffer, 0, contents),
        Some(_) => *buffer = mesh_buffer(device, label, contents, usage),
    }
}

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
/// Binds what one batch needs that its pass has not already bound: the
/// buffer its instances lie in, and the entry of the standing table that
/// puts them back in the world. Spec 0063.
///
/// A free function rather than a method, because every pass does this and
/// each of them is already holding a borrow of the renderer.
fn as_it_stands(
    pass: &mut wgpu::RenderPass<'_>,
    batch: &crate::renderer::scene::Batch,
    camera: &wgpu::BindGroup,
    standing_stride: u64,
    frame: &wgpu::Buffer,
    kept: &wgpu::Buffer,
) {
    use crate::renderer::scene::Where;

    pass.set_bind_group(
        0,
        camera,
        &[(batch.standing as u64 * standing_stride) as u32],
    );
    pass.set_vertex_buffer(
        1,
        match batch.from {
            Where::Frame => frame,
            Where::Kept => kept,
        }
        .slice(..),
    );
}

/// How big one entry of the standing table is: a `vec4` of which three are
/// the run's middle with the eye taken off. Spec 0063.
const STANDING: u64 = 16;

/// The scene's uniform and the standing table together, which is one bind
/// group because the second is read at an offset rather than rebound.
fn standing_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    camera: &wgpu::Buffer,
    standing: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Camera Bind Group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: standing,
                    offset: 0,
                    size: wgpu::BufferSize::new(STANDING),
                }),
            },
        ],
    })
}

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

/// The mesh shader, built for however many samples the device gave.
///
/// **The depth binding's type is part of the shader**, and it has to agree
/// with the bind group layout or the pipeline will not build at all. Written
/// multisampled and nothing else, as it was first, this engine crashed on
/// startup on any device that would not give four samples: the fallback that
/// spec 0052 promised drew as it always did could not even open a window.
///
/// It was not caught because the one run that exercised it was read wrong. The
/// window never appeared, and that was taken for a screenshot that missed
/// rather than a game that had died.
///
/// `textureLoad`'s third argument means the mip level on a plain texture and
/// the sample on a multisampled one, and nought is right either way, so the
/// type is the whole of the difference.
fn mesh_shader(device: &wgpu::Device, samples: u32) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Mesh Shader"),
        source: wgpu::ShaderSource::Wgsl(mesh_source(samples)),
    })
}

/// The mesh shader's source, for a given sample count.
///
/// Its own function so the substitution can be checked without a device, which
/// is the whole of what went wrong: the only thing that exercised the fallback
/// was a run of the game, and a run of the game that died looked the same as a
/// screenshot that missed.
pub fn mesh_source(samples: u32) -> std::borrow::Cow<'static, str> {
    const SOURCE: &str = include_str!("../../res/shaders/mesh.wgsl");

    if samples > 1 {
        std::borrow::Cow::Borrowed(SOURCE)
    } else {
        std::borrow::Cow::Owned(SOURCE.replace("texture_depth_multisampled_2d", "texture_depth_2d"))
    }
}

/// What the surface is configured as, and what everything is drawn through.
///
/// **What is drawn into is not always what is presented.**
///
/// The shader works in linear light and the surface is what turns that into
/// what a screen wants, so an sRGB one is not a preference: a plain one shows
/// linear values as though they were already encoded, and every mid tone
/// comes out dark. In the arcade that was a lit hall with its lamps and its
/// neon still bright and everything else nearly black, which reads as a
/// building with the power off. Jake, clicking play on the site: it got dark.
///
/// A browser cannot be given one. WebGPU lets a canvas be configured as
/// `rgba8unorm`, `bgra8unorm` or `rgba16float` and nothing else, so wgpu's
/// web backend advertises exactly those three and a search for an sRGB format
/// among them finds nothing. What WebGPU does allow is an sRGB *view* of that
/// canvas, declared up front as a view format, and drawing through that view
/// encodes on the way out. Spec 0053.
///
/// So there are two formats. The first is what the surface is configured as
/// and is all the surface ever hears about; the second is what every
/// pipeline, the multisampled target and the frame's own view are made with.
/// On a native surface they are the same value and none of this does
/// anything, which is why it went unnoticed for as long as it did.
fn formats(offered: &[wgpu::TextureFormat]) -> (wgpu::TextureFormat, wgpu::TextureFormat) {
    let format = offered
        .iter()
        .copied()
        .find(|format| format.is_srgb())
        .unwrap_or_else(|| offered.first().copied().unwrap_or(crate::texture::FORMAT));

    (format, format.add_srgb_suffix())
}

/// The texture the world is drawn into when the device multisamples.
///
/// One of these and not several: the surface's own texture cannot be
/// multisampled, because it is what gets presented, so the samples live here
/// and are resolved onto it. Nothing samples this, so it needs no binding.
fn multisampled(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    format: wgpu::TextureFormat,
    samples: u32,
) -> Option<wgpu::TextureView> {
    if samples <= 1 {
        return None;
    }

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Multisampled Colour"),
        size: wgpu::Extent3d {
            width: config.width.max(1),
            height: config.height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: samples,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });

    Some(texture.create_view(&wgpu::TextureViewDescriptor::default()))
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

/// Whether the surface waits for the display. Spec 0056.
///
/// **A frame rate with this on is a reading of the display.** Everything in
/// this engine has always waited, which is right for a game: it is the
/// cheapest way to have no tearing and no frames nobody sees. It also means
/// that a scene costing four milliseconds and one costing fifteen both report
/// sixty a second, so there is no way to tell from the picture whether
/// something got slower until it gets slow enough to miss a frame, which is a
/// cliff rather than a measurement.
///
/// So a game can ask for the other one. The only one that does is the one
/// whose whole job is to say what a frame costs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Presenting {
    /// Wait for the display, which is what every game wants.
    #[default]
    WaitForTheDisplay,
    /// Draw as fast as the device will, tearing and all.
    AsFastAsItWill,
}

/// What a pass writes into the query set, if anything is timing at all.
///
/// A free function rather than a method, because every call site has a render
/// pass descriptor half built out of `self` and a method would be a second
/// borrow of it.
fn timing_writes(
    timing: &mut Option<Timing>,
    slot: usize,
) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
    timing.as_mut().and_then(|timing| timing.writes(slot))
}

impl Counted {
    /// One instanced draw of a mesh with this many indices.
    fn saw(&mut self, indices: u32, instances: u32) {
        self.draws += 1;
        self.triangles += indices / 3 * instances;
    }
}

/// The GPU's own time for a frame, where the device will give it. Spec 0056.
///
/// **A frame's cost on the CPU is mostly waiting.** The loop measures the
/// wall clock between one frame and the next, which on a scene the GPU is
/// struggling with is the GPU's time arriving late rather than the work the
/// CPU did. So the device is asked directly, and what comes back is a pair of
/// ticks written either side of the frame's passes.
///
/// Two things make this fiddly rather than one line. The timestamps are
/// written by the GPU, so they are only readable once the frame has actually
/// run, which is some frames after it was submitted: this keeps one read in
/// flight and picks it up whenever it lands, so what the readout shows is a
/// real frame a moment ago rather than nothing. And on this machine the
/// device gives `TIMESTAMP_QUERY` and neither of the two that allow a
/// timestamp anywhere other than the edge of a render pass, which is why the
/// beginning is written by whichever pass happens to run first and the end by
/// the one that always runs last.
struct Timing {
    set: wgpu::QuerySet,
    resolved: wgpu::Buffer,
    read: wgpu::Buffer,
    /// Nanoseconds a tick, from the queue.
    period: f32,
    /// Whether a read is in flight, and whether it has landed.
    reading: bool,
    landed: Arc<AtomicBool>,
    /// Whether this frame has written its first timestamp yet.
    started: bool,
    /// Which slots this frame has written into, one bit each. Spec 0067:
    /// the query set is never cleared, so a slot nobody wrote holds the
    /// last frame that did, and this is what says which are worth reading.
    wrote: u32,
    /// The same, for the pair sitting in the query set unresolved, and for
    /// the one copied into the buffer that can be mapped, and for the one
    /// being read. Each travels with its data because they are three
    /// different frames.
    waiting: u32,
    copied: u32,
    read_of: u32,
    /// What each pass of the last reported frame took. Spec 0067.
    passes: [Option<f32>; crate::cost::PASSES],
    /// The last frame the GPU reported: its first pass beginning to its
    /// last one ending.
    last: Option<f32>,
    /// The same number under its other name, which the passes are held
    /// against.
    span: Option<f32>,
}

impl Timing {
    /// How many queries there are: a pair for each pass. Spec 0067.
    const QUERIES: u32 = crate::cost::PASSES as u32 * 2;
    /// And how many bytes that resolves to, which is a `u64` each.
    const BYTES: u64 = Self::QUERIES as u64 * 8;

    fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        Self {
            set: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("Frame Timestamps"),
                ty: wgpu::QueryType::Timestamp,
                count: Self::QUERIES,
            }),
            resolved: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Timestamps Resolved"),
                size: Self::BYTES,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            read: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Timestamps Read"),
                size: Self::BYTES,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            period: queue.get_timestamp_period(),
            reading: false,
            landed: Arc::new(AtomicBool::new(false)),
            started: false,
            wrote: 0,
            waiting: 0,
            copied: 0,
            read_of: 0,
            passes: [None; crate::cost::PASSES],
            last: None,
            span: None,
        }
    }

    /// The pair of queries one pass writes, which is its own two and
    /// nobody else's. Spec 0067.
    ///
    /// **A slot a pass, so there is no first and last to get wrong.** This
    /// used to take a `last` flag and hand the beginning to whichever pass
    /// asked first and the end to the one that always runs last. A
    /// diagnostic left in by mistake made every pass but the last return
    /// nothing here, and the engine reported the 2D pass as the frame for a
    /// day without anything noticing. A pass that names its own slot cannot
    /// be read as another one.
    fn writes(&mut self, slot: usize) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        // nothing is written while a read is in flight, or the pairs being
        // read would be overwritten by the frame that is reading them
        if self.reading || slot >= crate::cost::PASSES {
            return None;
        }

        self.started = true;
        self.wrote |= 1 << slot;

        Some(wgpu::RenderPassTimestampWrites {
            query_set: &self.set,
            beginning_of_pass_write_index: Some(slot as u32 * 2),
            end_of_pass_write_index: Some(slot as u32 * 2 + 1),
        })
    }
}

impl Presenting {
    /// Which of the surface's own modes that is.
    ///
    /// Asked of the surface rather than assumed, like the sample count in
    /// spec 0052 and the sRGB view in 0053. `Immediate` is the one that never
    /// waits and not every device has it; `Mailbox` still waits for the
    /// display to swap but never blocks the game, which is the next best
    /// thing; `Fifo` is the one every device has.
    pub fn out_of(self, offered: &[wgpu::PresentMode]) -> wgpu::PresentMode {
        let wanted: &[wgpu::PresentMode] = match self {
            Presenting::WaitForTheDisplay => &[wgpu::PresentMode::Fifo],
            Presenting::AsFastAsItWill => &[
                wgpu::PresentMode::Immediate,
                wgpu::PresentMode::Mailbox,
                wgpu::PresentMode::Fifo,
            ],
        };

        wanted
            .iter()
            .copied()
            .find(|mode| offered.contains(mode))
            .unwrap_or(wgpu::PresentMode::Fifo)
    }
}

/// A frame, as pixels. Spec 0062.
///
/// Rgba8 with no padding, top row first, which is what a picture test
/// compares and what it writes out when the comparison fails.
#[derive(Debug, Clone)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// What one frame of drawing came to, which travels with its time in
/// [`crate::cost::Frame`]. Spec 0056.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counted {
    /// Every draw call the frame made, in every pass: the shadow maps and the
    /// depth prepass as well as the picture, because they are what the frame
    /// cost and leaving them out would make a scene with shadows look free.
    pub draws: u32,
    /// And the triangles those calls asked for, instances counted.
    pub triangles: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spec 0056: a game that waits for the display always gets Fifo, and one
    /// that does not gets the best the surface will give it.
    ///
    /// Asked of the surface rather than assumed, like the sample count in
    /// spec 0052. `Immediate` is the one that never waits and not every
    /// device has it.
    #[test]
    fn the_present_mode_is_what_the_surface_will_give() {
        use wgpu::PresentMode::*;

        // every game but the one that measures, on any surface at all
        for offered in [
            vec![Fifo],
            vec![Fifo, Immediate, Mailbox],
            vec![Mailbox, Fifo],
            vec![],
        ] {
            assert_eq!(
                Presenting::WaitForTheDisplay.out_of(&offered),
                Fifo,
                "offered {:?}",
                offered
            );
        }

        // and the one that does not wait takes the best on offer
        assert_eq!(
            Presenting::AsFastAsItWill.out_of(&[Fifo, Mailbox, Immediate]),
            Immediate,
            "Immediate is the one that never waits"
        );
        assert_eq!(
            Presenting::AsFastAsItWill.out_of(&[Fifo, Mailbox]),
            Mailbox,
            "and Mailbox is the next best"
        );
        assert_eq!(
            Presenting::AsFastAsItWill.out_of(&[Fifo]),
            Fifo,
            "a surface with neither still has to draw"
        );
        assert_eq!(
            Presenting::AsFastAsItWill.out_of(&[]),
            Fifo,
            "and so does one that offers nothing"
        );
    }

    /// Spec 0056: what a frame drew is counted as it is drawn, instances and
    /// all, so a number that moves can be attributed.
    #[test]
    fn a_draw_is_counted_with_its_instances() {
        let mut counted = Counted::default();

        // one cube, which is twelve triangles
        counted.saw(36, 1);
        assert_eq!(counted.draws, 1);
        assert_eq!(counted.triangles, 12);

        // and a thousand of them in one instanced call, which is one draw
        // and a thousand times the triangles
        counted.saw(36, 1000);
        assert_eq!(counted.draws, 2, "instancing is one draw");
        assert_eq!(counted.triangles, 12 + 12_000);

        // a mesh with no indices is not a draw at all
        let mut empty = Counted::default();
        empty.saw(0, 10);
        assert_eq!(empty.triangles, 0);
    }

    /// Spec 0053: what is drawn through is sRGB whatever the surface offers.
    ///
    /// The browser's list is the case this exists for. A canvas can be
    /// configured as `rgba8unorm`, `bgra8unorm` or `rgba16float` and nothing
    /// else, so there is no sRGB format in it to find, and drawing linear
    /// light into one of those without the encode is what took the arcade's
    /// lights out on the site.
    #[test]
    fn what_is_drawn_through_is_always_srgb() {
        use wgpu::TextureFormat::*;

        // a browser, which offers nothing sRGB at all
        let (surface, drawn) = formats(&[Bgra8Unorm, Rgba8Unorm, Rgba16Float]);
        assert_eq!(surface, Bgra8Unorm, "the canvas takes what it can be given");
        assert_eq!(drawn, Bgra8UnormSrgb, "and is drawn through sRGB");
        assert!(drawn.is_srgb());

        // a native surface, which offers one, and where the two are one thing
        let (surface, drawn) = formats(&[Bgra8Unorm, Bgra8UnormSrgb]);
        assert_eq!(surface, Bgra8UnormSrgb, "an sRGB surface is taken as it is");
        assert_eq!(drawn, surface, "and nothing is laid over it");

        // and whatever is offered, what is drawn through encodes
        for offered in [
            vec![Rgba8Unorm],
            vec![Bgra8Unorm],
            vec![Rgba8UnormSrgb, Rgba8Unorm],
            vec![Bgra8Unorm, Rgba8Unorm],
        ] {
            let (surface, drawn) = formats(&offered);

            assert!(
                drawn.is_srgb(),
                "offered {:?}, drawn through {:?}",
                offered,
                drawn
            );
            assert!(
                offered.contains(&surface),
                "the surface was configured as {:?}, which was not offered",
                surface
            );
            assert_eq!(
                drawn.remove_srgb_suffix(),
                surface.remove_srgb_suffix(),
                "the view and the surface are the same format underneath"
            );
        }
    }

    /// Spec 0042: a buffer that can take the new bytes is written into rather
    /// than replaced, which is what makes a surface stepped every frame cost
    /// no allocation at all after the first.
    #[test]
    fn a_buffer_that_fits_is_kept() {
        assert_eq!(
            room(4096, 4096),
            None,
            "an exact fit asked for a new buffer"
        );
        assert_eq!(room(4096, 2048), None);
    }

    /// Spec 0042: and one too small is replaced with one big enough.
    #[test]
    fn a_buffer_too_small_is_replaced() {
        assert_eq!(room(2048, 4096), Some(4096));
        assert_eq!(room(0, 4), Some(4));
    }

    /// Spec 0042: a buffer is never shrunk.
    ///
    /// A mesh that grows and shrinks would otherwise allocate twice for every
    /// shape it has already had, which is the common case for anything built
    /// from a count that moves.
    #[test]
    fn a_buffer_is_never_shrunk() {
        assert_eq!(
            room(1 << 20, 16),
            None,
            "a big buffer was given up for a small write"
        );
    }

    /// Spec 0060: the instance buffer asks for twice what it needs, and never
    /// for more than this device said it would give.
    ///
    /// Doubling is what made four million cubes draw without an allocation a
    /// frame. Doubling with no ceiling is what killed the frame at a million
    /// and a half: 256 MB is the room a device usually gives, and the ask
    /// went past it.
    #[test]
    fn a_buffer_asks_for_no_more_than_the_device_will_give() {
        const MOST: u64 = 256 << 20;

        assert_eq!(
            instance_room(1 << 20, 1 << 10, MOST),
            None,
            "a buffer that already holds it was made again"
        );
        assert_eq!(
            instance_room(1 << 20, (1 << 20) + 1, MOST),
            Some(2 << 20),
            "a buffer that grew by a byte did not double"
        );
        assert_eq!(
            instance_room(200 << 20, 210 << 20, MOST),
            Some(MOST),
            "doubling past what the device gives was asked for anyway"
        );
    }

    /// Spec 0069: a run that will not fit is left out whole, and the ones
    /// before it are still drawn.
    #[test]
    fn what_does_not_fit_is_left_out_whole() {
        // four runs of a thousand into a buffer that holds two and a half
        // thousand: two fit, and the third is left out rather than the
        // first five hundred of it being drawn
        assert_eq!(what_fits(&[1000, 1000, 1000, 1000], 2500), (2, 2000));
        assert_eq!(
            what_fits(&[1000, 1000], 2500),
            (2, 0),
            "a scene that fits had something turned away"
        );
        assert_eq!(
            what_fits(&[4000], 2500),
            (0, 4000),
            "one run too big for any buffer was drawn anyway"
        );
        assert_eq!(what_fits(&[], 2500), (0, 0));
        // and a small run after a large one goes too, rather than being
        // packed into the gap the large one left
        assert_eq!(what_fits(&[2000, 1000, 10], 2500), (1, 1010));
    }

    /// Spec 0069: a buffer is never asked for more than the device gives.
    ///
    /// **Spec 0060 decided the other way and was wrong.** It asked for the
    /// whole of what a scene wanted even after the device had said no, on
    /// the grounds that the device would refuse it out loud rather than
    /// two thirds of a scene being drawn and called a frame. The out loud
    /// is a fatal validation error from inside wgpu, which is not this
    /// engine saying anything, and it is the only cap here that kills the
    /// game instead of saying what it turned away.
    #[test]
    fn a_buffer_never_asks_for_more_than_the_device_gives() {
        const MOST: u64 = 256 << 20;

        assert_eq!(
            instance_room(1 << 20, 300 << 20, MOST),
            Some(MOST),
            "a buffer was asked for more than this device will give"
        );
        assert_eq!(
            instance_room(1 << 20, MOST, MOST),
            Some(MOST),
            "a scene that exactly fits was not given the room"
        );
    }

    /// Spec 0042: and nothing to write asks for no buffer.
    ///
    /// An empty mesh is legal to upload, so it is legal to update to. wgpu
    /// refuses a buffer with nothing in it, so the mesh keeps the placeholder
    /// it was built with and draws nothing by its index count.
    #[test]
    fn empty_data_asks_for_nothing() {
        assert_eq!(room(4, 0), None);
        assert_eq!(MeshData::new(Vec::new(), Vec::new()).indices.len(), 0);
    }

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
            64 + 16 * 4 + 64 + 16 + 32 * MAX_POINT_LIGHTS + 16 + 128 * MAX_SPOT_LIGHTS + 16 + 16
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

        // everything past the lamps moves when there is room for more of them,
        // so these follow the count rather than restating it. Written out as
        // numbers they said the block was laid out for eight lamps and nothing
        // else, which is a test that fails the day the answer changes rather
        // than the day it goes wrong.
        let lamps = 32 * MAX_POINT_LIGHTS;
        let spot_count = 208 + lamps;
        let spots = spot_count + 16;
        let shadow = spots + 128 * MAX_SPOT_LIGHTS;
        let contact = shadow + 16;

        assert_eq!(
            std::mem::offset_of!(SceneUniform, spot_light_count),
            spot_count
        );
        assert_eq!(std::mem::offset_of!(SceneUniform, spot_lights), spots);
        assert_eq!(std::mem::offset_of!(SceneUniform, point_shadow), shadow);
        assert_eq!(std::mem::offset_of!(SceneUniform, contact), contact);

        for offset in [0, 64, 80, 96, 112, 128, 192, 208, spot_count, spots, shadow] {
            assert_eq!(offset % 16, 0, "{} is not on sixteen bytes", offset);
        }
    }

    #[test]
    fn the_uniform_carries_the_light() {
        let mut light = Light::new();
        light.intensity = 0.5;
        let uniform = SceneUniform::new(
            &Camera::new(),
            &light,
            &[],
            &[],
            &shadow::default_bounds(),
            true,
        );

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
            true,
        );

        assert_eq!(uniform.point_light_count[0], 2);
        // from the eye, per spec 0051, and the default camera stands at
        // (0, 1, 5), so (1, 2, 3) is two in front of it and one above
        assert_eq!(
            uniform.point_lights[0].position_range,
            [1.0, 1.0, -2.0, 5.0]
        );
        assert_eq!(
            uniform.point_lights[0].color_intensity,
            [1.0, 0.0, 0.0, 2.0]
        );
        assert_eq!(
            uniform.point_lights[1].position_range,
            [-4.0, -1.0, -5.0, 9.0]
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
            true,
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
            true,
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
            true,
        );

        assert_eq!(uniform.spot_light_count[0], 1);
        // from the eye, per spec 0051, with the default camera at (0, 1, 5)
        assert_eq!(uniform.spot_lights[0].position_range, [0.0, 2.0, -5.0, 9.0]);
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
            true,
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
            true,
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
            true,
        );

        assert_eq!(uniform.point_shadow, [0; 4]);
    }

    #[test]
    fn a_casting_lamp_gets_six_layers_and_the_rest_get_none() {
        let lamps =
            [PointLight::new(glam::vec3(0.0, 3.0, 0.0), glam::Vec3::ONE, 1.0, 8.0).casting()];
        let faces = point_faces(&lamps, glam::Vec3::ZERO);

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
        let camera = Camera::new();
        let uniform = SceneUniform::new(
            &camera,
            &Light::new(),
            &lamps,
            &[],
            &shadow::default_bounds(),
            true,
        );
        // and both sides have to have moved the world the same way, per spec
        // 0051, or the slot points at a lamp standing somewhere else
        let faces = point_faces(&lamps, camera.position);

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

#[cfg(test)]
mod asked {
    use super::*;

    /// Every answer that is not a frame to draw into.
    fn no_frame() -> [wgpu::CurrentSurfaceTexture; 4] {
        [
            wgpu::CurrentSurfaceTexture::Outdated,
            wgpu::CurrentSurfaceTexture::Lost,
            wgpu::CurrentSurfaceTexture::Timeout,
            wgpu::CurrentSurfaceTexture::Occluded,
        ]
    }

    /// A frame that draws nothing still hands its staging back.
    ///
    /// The one that matters, and the one that was wrong. `write_buffer` stages
    /// into memory that only comes back on a submit, so a frame that uploads
    /// and then returns without submitting keeps what it uploaded for ever.
    /// A window behind another window never presents, so nothing waits on a
    /// vsync either and the loop runs flat out: the arcade grew by the better
    /// part of half a gigabyte a second that way, reached a hundred and
    /// sixty-six gigabytes, and took the machine down with it four times over
    /// six days before anybody caught it.
    ///
    /// It only happened while nobody was looking at the window, which is why a
    /// test is the only thing that can hold it: there is no screenshot of this.
    #[test]
    fn a_frame_that_draws_nothing_still_flushes() {
        for got in no_frame() {
            let asked = Asked::at(&got);

            assert!(!asked.draws, "{:?} is not a frame to draw into", got);
            assert!(
                asked.must_flush(),
                "{:?} gives up without handing its staging back",
                got
            );
        }
    }

    /// Being occluded or timing out is not a fault and is not logged.
    ///
    /// At a thousand frames a second an occluded window writes a thousand lines
    /// a second, which is its own way of filling a disk.
    #[test]
    fn a_hidden_window_is_not_an_error() {
        for got in [
            wgpu::CurrentSurfaceTexture::Timeout,
            wgpu::CurrentSurfaceTexture::Occluded,
        ] {
            assert!(!Asked::at(&got).complain, "{:?} is logged as a fault", got);
        }
        for got in [
            wgpu::CurrentSurfaceTexture::Outdated,
            wgpu::CurrentSurfaceTexture::Lost,
        ] {
            assert!(
                !Asked::at(&got).complain && Asked::at(&got).reconfigure,
                "{:?} should be set up again rather than complained about",
                got
            );
        }
    }
}
