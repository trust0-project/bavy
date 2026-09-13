//! Native wgpu HDL presenter (Metal on macOS).
//!
//! Instanced 2D, same packing/shaders as the site WebGPU executor. The winit
//! window owns the surface; VM execution stays on another thread.
//!
//! `SurfaceError::Lost` / `Outdated`: recreate/reconfigure the surface and
//! redraw the last good HDL. Device probe (`try_device`) must succeed before
//! a window is opened — `adapter` + `request_device`, not just Instance.

use std::sync::Arc;

use winit::window::Window;

use crate::hdl_frame;
use crate::hdl_pack;
use crate::hdl_raster::{self, Packed, Rect, INSTANCE_BYTES};

const SHADER: &str = include_str!("hdl_instanced.wgsl");
const MIN_UNIFORM: u64 = 256;

/// Adapter + device that succeeded without a surface.
#[derive(Clone)]
pub struct GpuDevice {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub backend: String,
}

/// Probe GPU. `navigator.gpu` analogue: adapter **and** device must succeed.
pub fn try_device() -> Result<GpuDevice, String> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .ok_or_else(|| "wgpu request_adapter returned None".to_string())?;
    let info = adapter.get_info();
    let backend = format!("{} ({:?})", info.name, info.backend);
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("havy-hdl"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
        },
        None,
    ))
    .map_err(|e| format!("wgpu request_device failed: {e}"))?;
    Ok(GpuDevice {
        instance,
        adapter,
        device,
        queue,
        backend,
    })
}

pub struct HdlWgpu {
    window: Arc<Window>,
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    instance_buf: wgpu::Buffer,
    bind0: wgpu::BindGroup,
    bind1: wgpu::BindGroup,
    _textures: Vec<wgpu::Texture>,
}

impl HdlWgpu {
    pub fn new(window: Arc<Window>, gpu: GpuDevice) -> Result<Self, String> {
        let surface = gpu
            .instance
            .create_surface(window.clone())
            .map_err(|e| format!("create_surface: {e}"))?;
        if !gpu.adapter.is_surface_supported(&surface) {
            return Err("adapter does not support the window surface".into());
        }
        let size = window.inner_size();
        let config = surface_config(&surface, &gpu.adapter, size.width.max(1), size.height.max(1))?;
        surface.configure(&gpu.device, &config);

        let shader = gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("hdl-instanced"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let pipeline = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("hdl-instanced"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: INSTANCE_BYTES as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[
                        wgpu::VertexAttribute {
                            shader_location: 0,
                            offset: 0,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                        wgpu::VertexAttribute {
                            shader_location: 1,
                            offset: 16,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                        wgpu::VertexAttribute {
                            shader_location: 2,
                            offset: 32,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                        wgpu::VertexAttribute {
                            shader_location: 3,
                            offset: 48,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let uniform = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hdl-viewport"),
            size: MIN_UNIFORM,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let instance_buf = grow_instance_buffer(&gpu.device, None, 256);

        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("hdl-nearest"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        let (textures, views) = upload_pack(&gpu.device, &gpu.queue);
        let bind0 = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("hdl-bg0"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        let bind1 = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("hdl-bg1"),
            layout: &pipeline.get_bind_group_layout(1),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&views[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&views[1]),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&views[2]),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&views[3]),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&views[4]),
                },
            ],
        });

        Ok(Self {
            window,
            instance: gpu.instance,
            adapter: gpu.adapter,
            device: gpu.device,
            queue: gpu.queue,
            surface,
            config,
            pipeline,
            uniform,
            instance_buf,
            bind0,
            bind1,
            _textures: textures,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        if self.config.width == width && self.config.height == height {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Decode HDL bytes and present. Invalid frames are errors (caller keeps last good).
    pub fn present_bytes(&mut self, bytes: &[u8]) -> Result<(), String> {
        let frame = hdl_frame::decode(bytes).map_err(|e| e.as_str().to_string())?;
        let packed = hdl_raster::pack(&frame);
        self.present_packed(&packed)
    }

    pub fn present_clear(&mut self, argb: u32) -> Result<(), String> {
        let packed = Packed {
            width: 1024,
            height: 768,
            clear_argb: argb,
            instance_count: 0,
            instances: Vec::new(),
            batches: Vec::new(),
        };
        self.present_packed(&packed)
    }

    pub fn present_packed(&mut self, packed: &Packed) -> Result<(), String> {
        match self.submit(packed) {
            Ok(()) => Ok(()),
            Err(wgpu::SurfaceError::Timeout) => Ok(()),
            Err(wgpu::SurfaceError::OutOfMemory) => Err("wgpu surface out of memory".into()),
            Err(e @ (wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Other)) => {
                self.recover_surface(&e)?;
                self.submit(packed).map_err(|e| format!("wgpu present after recover: {e}"))
            }
        }
    }

    fn recover_surface(&mut self, why: &wgpu::SurfaceError) -> Result<(), String> {
        match why {
            wgpu::SurfaceError::Lost | wgpu::SurfaceError::Other => self.recreate_surface(),
            wgpu::SurfaceError::Outdated => {
                let size = self.window.inner_size();
                self.resize(size.width.max(1), size.height.max(1));
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn recreate_surface(&mut self) -> Result<(), String> {
        let surface = self
            .instance
            .create_surface(self.window.clone())
            .map_err(|e| format!("recreate surface: {e}"))?;
        if !self.adapter.is_surface_supported(&surface) {
            return Err("adapter no longer supports the window surface".into());
        }
        let size = self.window.inner_size();
        let config = surface_config(&surface, &self.adapter, size.width.max(1), size.height.max(1))?;
        // Format change would need a new pipeline; keep the previous format if still listed.
        let format = if surface
            .get_capabilities(&self.adapter)
            .formats
            .contains(&self.config.format)
        {
            self.config.format
        } else {
            config.format
        };
        self.config = wgpu::SurfaceConfiguration {
            format,
            ..config
        };
        surface.configure(&self.device, &self.config);
        self.surface = surface;
        Ok(())
    }

    fn submit(&mut self, packed: &Packed) -> Result<(), wgpu::SurfaceError> {
        let logical_w = packed.width.max(1) as f32;
        let logical_h = packed.height.max(1) as f32;
        let fb_w = self.config.width.max(1);
        let fb_h = self.config.height.max(1);
        let uniforms = [logical_w, logical_h, fb_w as f32, fb_h as f32];
        self.queue
            .write_buffer(&self.uniform, 0, f32_bytes(&uniforms));

        if packed.bytes_uploaded() > 0 {
            self.instance_buf =
                grow_instance_buffer(&self.device, Some(&self.instance_buf), packed.bytes_uploaded());
            self.queue
                .write_buffer(&self.instance_buf, 0, f32_bytes(&packed.instances));
        }

        let frame = self.surface.get_current_texture()?;
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let (r, g, b, a) = hdl_raster::clear_rgba(packed.clear_argb);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("hdl-present"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("hdl-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind0, &[]);
            pass.set_bind_group(1, &self.bind1, &[]);
            if packed.instance_count > 0 {
                let sx = fb_w as f32 / logical_w;
                let sy = fb_h as f32 / logical_h;
                for batch in &packed.batches {
                    let Some((x, y, w, h)) = clamp_scissor(scale_rect(batch.scissor, sx, sy), fb_w, fb_h)
                    else {
                        continue;
                    };
                    if batch.instance_count == 0 {
                        continue;
                    }
                    pass.set_scissor_rect(x, y, w, h);
                    let off = batch.first_instance as u64 * INSTANCE_BYTES as u64;
                    let size = batch.instance_count as u64 * INSTANCE_BYTES as u64;
                    pass.set_vertex_buffer(0, self.instance_buf.slice(off..off + size));
                    pass.draw(0..6, 0..batch.instance_count);
                }
            }
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }
}

fn surface_config(
    surface: &wgpu::Surface<'_>,
    adapter: &wgpu::Adapter,
    width: u32,
    height: u32,
) -> Result<wgpu::SurfaceConfiguration, String> {
    let caps = surface.get_capabilities(adapter);
    let format = caps
        .formats
        .iter()
        .copied()
        .find(|f| !f.is_srgb())
        .or_else(|| caps.formats.first().copied())
        .ok_or_else(|| "no surface formats".to_string())?;
    let present_mode = if caps.present_modes.contains(&wgpu::PresentMode::Fifo) {
        wgpu::PresentMode::Fifo
    } else {
        *caps
            .present_modes
            .first()
            .ok_or_else(|| "no present modes".to_string())?
    };
    let alpha_mode = if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
        wgpu::CompositeAlphaMode::Opaque
    } else {
        caps.alpha_modes
            .first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto)
    };
    Ok(wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width,
        height,
        present_mode,
        alpha_mode,
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    })
}

fn grow_instance_buffer(
    device: &wgpu::Device,
    prev: Option<&wgpu::Buffer>,
    bytes: usize,
) -> wgpu::Buffer {
    let size = (bytes.max(64) as u64).next_power_of_two().max(256);
    if let Some(prev) = prev {
        if prev.size() >= size {
            return prev.clone();
        }
    }
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("hdl-instances"),
        size,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn upload_pack(device: &wgpu::Device, queue: &wgpu::Queue) -> (Vec<wgpu::Texture>, Vec<wgpu::TextureView>) {
    let pack = hdl_pack::load();
    let mut textures = Vec::with_capacity(5);
    let mut views = Vec::with_capacity(5);
    let mut add = |w: u32, h: u32, rgba: &[u8]| {
        let tex = upload_rgba(device, queue, w.max(1), h.max(1), rgba);
        views.push(tex.create_view(&wgpu::TextureViewDescriptor::default()));
        textures.push(tex);
    };
    for atlas in &pack.atlases {
        let rgba = hdl_pack::atlas_to_rgba(atlas);
        add(atlas.atlas_width, atlas.atlas_height, &rgba);
    }
    for id in 0..3u16 {
        if let Some(t) = hdl_pack::texture_by_id(id) {
            add(t.width, t.height, &t.rgba);
        } else {
            add(1, 1, &[0, 0, 0, 0]);
        }
    }
    (textures, views)
}

fn upload_rgba(device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32, rgba: &[u8]) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("hdl-pack"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let (padded, bpr) = pad_rows(width, height, rgba);
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &padded,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(bpr),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    texture
}

fn pad_rows(width: u32, height: u32, rgba: &[u8]) -> (Vec<u8>, u32) {
    let unpadded = width * 4;
    let padded = unpadded.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    if padded == unpadded && rgba.len() >= (unpadded * height) as usize {
        return (rgba.to_vec(), unpadded);
    }
    let mut out = vec![0u8; (padded * height) as usize];
    let row = (unpadded as usize).min(rgba.len());
    for y in 0..height as usize {
        let src = y * unpadded as usize;
        let dst = y * padded as usize;
        if src >= rgba.len() {
            break;
        }
        let n = row.min(rgba.len() - src);
        out[dst..dst + n].copy_from_slice(&rgba[src..src + n]);
    }
    (out, padded)
}

fn scale_rect(r: Rect, sx: f32, sy: f32) -> Rect {
    Rect {
        x: r.x * sx,
        y: r.y * sy,
        w: r.w * sx,
        h: r.h * sy,
    }
}

fn clamp_scissor(r: Rect, fb_w: u32, fb_h: u32) -> Option<(u32, u32, u32, u32)> {
    let x0 = r.x.floor().max(0.0).min(fb_w as f32) as u32;
    let y0 = r.y.floor().max(0.0).min(fb_h as f32) as u32;
    let x1 = (r.x + r.w).ceil().max(0.0).min(fb_w as f32) as u32;
    let y1 = (r.y + r.h).ceil().max(0.0).min(fb_h as f32) as u32;
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    Some((x0, y0, x1 - x0, y1 - y0))
}

fn f32_bytes(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 4) }
}
