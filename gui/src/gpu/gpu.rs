use iced::wgpu;
use std::sync::Arc;
use wgpu::ExperimentalFeatures;
use winit::window::Window;

pub struct Gpu<'window> {
    pub surface: wgpu::Surface<'window>,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_format: wgpu::TextureFormat,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub depth_texture: wgpu::Texture,
}

impl<'window> Gpu<'window> {
    pub async fn new_async(
        // window: impl Into<wgpu::SurfaceTarget<'window>>,
        // window: &'window Window,
        window: Arc<Window>,
    ) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to request adapter!");

        let (device, queue) = {
            log::info!("WGPU Adapter Features: {:#?}", adapter.features());
            adapter
                .request_device(&wgpu::DeviceDescriptor {
                    label: Some("WGPU Device"),
                    required_features: wgpu::Features::default()
                        | wgpu::Features::DEPTH_CLIP_CONTROL,
                    required_limits: wgpu::Limits {
                        max_texture_dimension_2d: 4096, // Allow higher resolutions on native
                        ..wgpu::Limits::downlevel_defaults()
                    },
                    memory_hints: wgpu::MemoryHints::default(),
                    experimental_features: ExperimentalFeatures::disabled(),
                    trace: wgpu::Trace::Off,
                })
                .await
                .expect("Failed to request a device!")
        };

        let surface_capabilities = surface.get_capabilities(&adapter);

        // This assumes an sRGB surface texture
        let surface_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb()) // egui wants a non-srgb surface texture
            .unwrap_or(surface_capabilities.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_capabilities.present_modes[0],
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &surface_config);

        let depth_texture = Self::create_depth_texture(&device, &surface_config);

        Self {
            surface,
            adapter,
            device,
            queue,
            surface_format,
            surface_config,
            depth_texture,
        }
    }

    fn create_depth_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
            self.depth_texture = Self::create_depth_texture(&self.device, &self.surface_config);
        }
    }

    #[allow(dead_code)]
    pub fn debug_render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
