use std::path::Path;
use std::sync::Arc;

use glam::{Mat4, Vec3};
use image::{ImageBuffer, Rgba, RgbaImage};
use wgpu::util::DeviceExt;

use crate::mesh::Mesh;

const MIN_STRIPES: f32 = 8.0;
const MAX_STRIPES: f32 = 24.0;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct StereoParams {
    tile_color: f32,
    tile_height: f32,
    tile_scale_x: f32,
    tile_scale_y: f32,
    show_uv: f32,
    main_stripe: f32,
    depth_factor: f32,
    invert: f32,
    heightmap_scale_x: f32,
    heightmap_scale_y: f32,
    stripes_count: f32,
    show_heightmap: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SceneParams {
    mvp: [[f32; 4]; 4],
}

pub struct ViewModel {
    pub eye: [f32; 3],
    pub near: f32,
    pub far: f32,
}

pub struct FrameInput<'a> {
    pub width: u32,
    pub height: u32,
    pub heightmap: &'a RgbaImage,
    pub tile: &'a RgbaImage,
    pub tile_padding: u32,
    pub colored_tile: bool,
    pub show_uv: bool,
    pub show_heightmap: bool,
    pub invert: bool,
    pub depth: f32,
    pub main_stripe_centered: bool,
    pub adaptive_stripes: bool,
    pub stripe_width: f32,
    pub fixed_stripes: f32,
    pub moving: bool,
    pub mesh: Option<&'a Mesh>,
    pub view: ViewModel,
    pub elapsed_ms: f32,
}

pub struct Gpu {
    // The window surface was created from this instance, so it has to stay alive.
    #[allow(dead_code)]
    instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter: wgpu::Adapter,
    blit_shader: wgpu::ShaderModule,
    sampler: wgpu::Sampler,
    stereo_pipeline: wgpu::RenderPipeline,
    scene_pipeline: wgpu::RenderPipeline,
    blit_pipeline: wgpu::RenderPipeline,
    stereo_layout: wgpu::BindGroupLayout,
    scene_layout: wgpu::BindGroupLayout,
    blit_layout: wgpu::BindGroupLayout,
    stereo_buffer: wgpu::Buffer,
    scene_buffer: wgpu::Buffer,
    heightmap: GpuImage,
    tile: GpuImage,
    color: Option<GpuTarget>,
    depth_color: Option<GpuImage>,
    depth: Option<wgpu::TextureView>,
    depth_size: (u32, u32),
    mesh_buffer: Option<(wgpu::Buffer, u32)>,
    mesh_id: u64,
}

struct GpuImage {
    // The view does not keep the texture alive.
    #[allow(dead_code)]
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    #[allow(dead_code)]
    size: (u32, u32),
    key: u64,
}

struct GpuTarget {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: (u32, u32),
}

impl Gpu {
    pub fn headless() -> Result<Self, String> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        Self::open(instance, None)
    }

    pub fn with_window(
        window: Arc<winit::window::Window>,
    ) -> Result<(Self, wgpu::Surface<'static>, wgpu::SurfaceConfiguration), String> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = instance
            .create_surface(window.clone())
            .map_err(|error| format!("No pude crear la superficie: {error}"))?;
        let mut gpu = Self::open(instance, Some(&surface))?;
        let caps = surface.get_capabilities(&gpu.adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .unwrap_or(caps.formats[0]);
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        gpu.set_present_format(format);
        surface.configure(&gpu.device, &config);
        Ok((gpu, surface, config))
    }

    fn open(instance: wgpu::Instance, surface: Option<&wgpu::Surface<'_>>) -> Result<Self, String> {
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: surface,
            force_fallback_adapter: false,
        }))
        .ok_or_else(|| "No hay un adaptador de GPU".to_string())?;
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("estereograma"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .map_err(|error| format!("No pude abrir el dispositivo de GPU: {error}"))?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("stereogram"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/stereogram.wgsl").into()),
        });
        let scene_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/scene.wgsl").into()),
        });
        let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blit"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/blit.wgsl").into()),
        });

        let stereo_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("stereo-layout"),
            entries: &[
                uniform_entry(0),
                texture_entry(1),
                sampler_entry(2),
                texture_entry(3),
                sampler_entry(4),
            ],
        });
        let scene_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene-layout"),
            entries: &[uniform_entry(0)],
        });
        let blit_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("blit-layout"),
            entries: &[texture_entry(0), sampler_entry(1)],
        });

        let stereo_pipeline = pipeline(
            &device,
            &shader,
            "vs_fullscreen",
            "fs_stereogram",
            &stereo_layout,
            false,
            wgpu::TextureFormat::Rgba8Unorm,
        );
        let scene_pipeline = mesh_pipeline(&device, &scene_shader, &scene_layout);
        let blit_pipeline = pipeline(
            &device,
            &blit_shader,
            "vs_blit",
            "fs_blit",
            &blit_layout,
            false,
            wgpu::TextureFormat::Rgba8Unorm,
        );

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("linear-clamp"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let placeholder = RgbaImage::from_pixel(1, 1, Rgba([0, 0, 0, 255]));
        let heightmap = upload_image(&device, &queue, &placeholder, "heightmap");
        let tile = upload_image(&device, &queue, &placeholder, "tile");
        let stereo_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("stereo-params"),
            size: 256,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let scene_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("scene-params"),
            contents: bytemuck::bytes_of(&SceneParams {
                mvp: Mat4::IDENTITY.to_cols_array_2d(),
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        Ok(Self {
            instance,
            device,
            queue,
            adapter,
            blit_shader,
            sampler,
            stereo_pipeline,
            scene_pipeline,
            blit_pipeline,
            stereo_layout,
            scene_layout,
            blit_layout,
            stereo_buffer,
            scene_buffer,
            heightmap,
            tile,
            color: None,
            depth_color: None,
            depth: None,
            depth_size: (0, 0),
            mesh_buffer: None,
            mesh_id: 0,
        })
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn set_present_format(&mut self, format: wgpu::TextureFormat) {
        let fragment = if format.is_srgb() { "fs_blit_srgb" } else { "fs_blit" };
        self.blit_pipeline = pipeline(
            &self.device,
            &self.blit_shader,
            "vs_blit",
            fragment,
            &self.blit_layout,
            false,
            format,
        );
    }

    pub fn render_image(&mut self, input: &FrameInput<'_>) -> Result<RgbaImage, String> {
        self.sync_sources(input);
        let view = self.color_view(input.width.max(1), input.height.max(1));
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });
        self.draw_scene(input, &mut encoder);
        self.draw_stereogram(input, &view, &mut encoder);
        self.queue.submit(Some(encoder.finish()));
        self.read_color()
    }

    pub fn render_to_view(
        &mut self,
        input: &FrameInput<'_>,
        destination: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        viewport: (f32, f32, f32, f32),
    ) {
        self.sync_sources(input);
        let view = self.color_view(input.width.max(1), input.height.max(1));
        self.draw_scene(input, encoder);
        self.draw_stereogram(input, &view, encoder);
        self.blit(&view, destination, encoder, viewport);
    }

    fn sync_sources(&mut self, input: &FrameInput<'_>) {
        if input.moving {
            self.ensure_depth(input.width.max(1), input.height.max(1));
        } else if self.heightmap.key != image_key(input.heightmap) {
            self.heightmap = upload_image(&self.device, &self.queue, input.heightmap, "heightmap");
        }
        if self.tile.key != image_key(input.tile) {
            self.tile = upload_image(&self.device, &self.queue, input.tile, "tile");
        }
    }

    fn draw_scene(&mut self, input: &FrameInput<'_>, encoder: &mut wgpu::CommandEncoder) {
        if !input.moving {
            return;
        }
        let width = input.width.max(1);
        let height = input.height.max(1);
        self.ensure_depth(width, height);
        let Some(mesh) = input.mesh else {
            return;
        };
        if self.mesh_id == 0 {
            self.upload_mesh(1, mesh);
        }
        let Some((buffer, vertex_count)) = &self.mesh_buffer else {
            return;
        };

        let aspect = width as f32 / height as f32;
        let model = Mat4::from_axis_angle(Vec3::Z, 0.0005 * input.elapsed_ms);
        let view = Mat4::look_at_rh(Vec3::from(input.view.eye), Vec3::ZERO, Vec3::Z);
        let projection =
            // gl-matrix receives 45 as radians, not as degrees.
            Mat4::perspective_rh_gl(45.0, aspect, input.view.near, input.view.far);
        let mvp = projection * view * model;
        self.queue.write_buffer(
            &self.scene_buffer,
            0,
            bytemuck::bytes_of(&SceneParams {
                mvp: mvp.to_cols_array_2d(),
            }),
        );
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene-bind"),
            layout: &self.scene_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.scene_buffer.as_entire_binding(),
            }],
        });
        let color_view = self.depth_color.as_ref().unwrap().view.clone();
        let depth_view = self.depth.as_ref().unwrap().clone();
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("depth-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.scene_pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.set_vertex_buffer(0, buffer.slice(..));
        pass.draw(0..*vertex_count, 0..1);
    }

    fn draw_stereogram(
        &mut self,
        input: &FrameInput<'_>,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let stripes = stripe_count(input);
        let params = stereo_params(input, stripes);
        self.queue
            .write_buffer(&self.stereo_buffer, 0, bytemuck::bytes_of(&params));

        let heightmap_view = if input.moving {
            self.depth_color.as_ref().unwrap().view.clone()
        } else {
            self.heightmap.view.clone()
        };
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("stereo-bind"),
            layout: &self.stereo_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.stereo_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&heightmap_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&self.tile.view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("stereo-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.stereo_pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.draw(0..3, 0..1);
    }

    fn blit(
        &self,
        source: &wgpu::TextureView,
        destination: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        viewport: (f32, f32, f32, f32),
    ) {
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blit-bind"),
            layout: &self.blit_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("blit-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: destination,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.blit_pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.set_viewport(
            viewport.0,
            viewport.1,
            viewport.2.max(1.0),
            viewport.3.max(1.0),
            0.0,
            1.0,
        );
        pass.draw(0..3, 0..1);
    }

    fn color_view(&mut self, width: u32, height: u32) -> wgpu::TextureView {
        self.ensure_color(width, height);
        self.color.as_ref().unwrap().view.clone()
    }

    fn read_color(&self) -> Result<RgbaImage, String> {
        let target = self.color.as_ref().ok_or("No hay imagen para leer.")?;
        read_texture(
            &self.device,
            &self.queue,
            &target.texture,
            target.size.0,
            target.size.1,
        )
    }

    fn ensure_color(&mut self, width: u32, height: u32) {
        let size = (width, height);
        let needs_new = self
            .color
            .as_ref()
            .map(|target| target.size != size)
            .unwrap_or(true);
        if needs_new {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("color"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.color = Some(GpuTarget {
                texture,
                view,
                size,
            });
        }
    }

    fn ensure_depth(&mut self, width: u32, height: u32) {
        if self.depth_size == (width, height) && self.depth_color.is_some() {
            return;
        }
        let color = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth-color"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let color_view = color.create_view(&wgpu::TextureViewDescriptor::default());
        let depth = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        self.depth = Some(depth.create_view(&wgpu::TextureViewDescriptor::default()));
        self.depth_color = Some(GpuImage {
            texture: color,
            view: color_view,
            size: (width, height),
            key: 0,
        });
        self.depth_size = (width, height);
    }

    fn upload_mesh(&mut self, id: u64, mesh: &Mesh) {
        let buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("mesh"),
                contents: bytemuck::cast_slice(&mesh.positions),
                usage: wgpu::BufferUsages::VERTEX,
            });
        self.mesh_buffer = Some((buffer, mesh.triangle_count() * 3));
        self.mesh_id = id;
    }

    pub fn set_mesh(&mut self, id: u64, mesh: &Mesh) {
        if self.mesh_id != id {
            self.upload_mesh(id, mesh);
        }
    }
}

pub fn save_png(path: &Path, image: &RgbaImage) -> Result<(), String> {
    image
        .save(path)
        .map_err(|error| format!("No pude guardar la imagen: {error}"))
}

fn stripe_count(input: &FrameInput<'_>) -> f32 {
    if input.adaptive_stripes {
        let ideal = (input.width as f32 / input.stripe_width.max(1.0)).round();
        ideal.clamp(MIN_STRIPES, MAX_STRIPES)
    } else {
        input.fixed_stripes.clamp(MIN_STRIPES, MAX_STRIPES)
    }
}

fn stereo_params(input: &FrameInput<'_>, stripes: f32) -> StereoParams {
    let (heightmap_width, heightmap_height) = if input.moving {
        (input.width.max(1), input.height.max(1))
    } else {
        (input.heightmap.width(), input.heightmap.height())
    };
    let useful = stripes / (stripes + 1.0);
    let heightmap_h = if input.show_heightmap { useful } else { 1.0 };
    let canvas_aspect = (input.width as f32 / input.height.max(1) as f32) * useful;
    let map_aspect = heightmap_width as f32 / heightmap_height.max(1) as f32;
    let (scale_x, scale_y) = if canvas_aspect > map_aspect {
        (canvas_aspect / map_aspect / heightmap_h, -1.0)
    } else {
        (1.0 / heightmap_h, -map_aspect / canvas_aspect)
    };

    let useful_w = input
        .tile
        .width()
        .saturating_sub(2 * input.tile_padding)
        .max(1) as f32;
    let useful_h = input
        .tile
        .height()
        .saturating_sub(2 * input.tile_padding)
        .max(1) as f32;
    let tile_width_px = input.width as f32 / (stripes + 1.0);
    let tile_height_px = tile_width_px / (useful_w / useful_h);
    let tile_height = tile_height_px / input.height.max(1) as f32;
    let main_stripe = if input.main_stripe_centered {
        (stripes * 0.5).floor()
    } else {
        0.0
    };

    StereoParams {
        tile_color: if input.colored_tile { 1.0 } else { 0.0 },
        tile_height,
        tile_scale_x: useful_w / input.tile.width().max(1) as f32,
        tile_scale_y: -(useful_h / input.tile.height().max(1) as f32),
        show_uv: if input.show_uv { 1.0 } else { 0.0 },
        main_stripe,
        depth_factor: input.depth,
        invert: if input.invert { 1.0 } else { 0.0 },
        heightmap_scale_x: scale_x,
        heightmap_scale_y: scale_y,
        stripes_count: stripes,
        show_heightmap: if input.show_heightmap { 1.0 } else { 0.0 },
    }
}

fn image_key(image: &RgbaImage) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    image.width().hash(&mut hasher);
    image.height().hash(&mut hasher);
    image.as_raw().hash(&mut hasher);
    hasher.finish()
}

fn upload_image(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    image: &RgbaImage,
    label: &str,
) -> GpuImage {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: image.width(),
            height: image.height(),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        image.as_raw(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * image.width()),
            rows_per_image: Some(image.height()),
        },
        wgpu::Extent3d {
            width: image.width(),
            height: image.height(),
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    GpuImage {
        texture,
        view,
        size: (image.width(), image.height()),
        key: image_key(image),
    }
}

fn read_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Result<RgbaImage, String> {
    let bytes_per_row = (width * 4 + 255) & !255;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (bytes_per_row * height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("readback"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));
    let slice = buffer.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device.poll(wgpu::Maintain::Wait);
    receiver
        .recv()
        .map_err(|_| "La GPU no devolvió la imagen.".to_string())?
        .map_err(|error| format!("No pude leer la imagen de la GPU: {error}"))?;
    let data = slice.get_mapped_range();
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        let start = (y * bytes_per_row) as usize;
        pixels.extend_from_slice(&data[start..start + (width * 4) as usize]);
    }
    drop(data);
    buffer.unmap();
    ImageBuffer::from_raw(width, height, pixels)
        .ok_or_else(|| "La imagen leída no tiene el tamaño esperado.".to_string())
}

fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn sampler_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    vertex: &str,
    fragment: &str,
    layout: &wgpu::BindGroupLayout,
    depth: bool,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(fragment),
        bind_group_layouts: &[layout],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(fragment),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(vertex),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fragment),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: depth.then(|| wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24Plus,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

fn mesh_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("mesh"),
        bind_group_layouts: &[layout],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("mesh"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_mesh"),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 12,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                }],
            }],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_depth"),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24Plus,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}
