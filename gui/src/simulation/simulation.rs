use std::time::Instant;

use log::debug;
use winit::event::WindowEvent;

use crate::{astro::constants::EARTH_RADIUS, gpu::Gpu, simulation::camera::CameraUniform};

use super::{
    camera::{Camera, CameraController},
    frame::FramePipeline,
    planet_pipeline::PlanetPipeline,
};

pub enum SimulationUpdateResult {
    RequiresRedraw,
    Done,
}

pub struct Simulation {
    camera_controller: CameraController,
    camera_uniform: CameraUniform,
    earth: PlanetPipeline,
    frame: FramePipeline,
    // square: SquarePipeline,
    simulation_speed: f64,
    simulation_time: Instant,
    last_tick: Instant,
}

impl Simulation {
    pub fn new(gpu: &Gpu, simulation_speed: f64) -> Self {
        debug!(
            "width: {}, height: {}",
            gpu.surface_config.width, gpu.surface_config.height
        );
        /***************
         * Camera
         ***************/
        let camera = Camera::new(
            gpu.surface_config.width as f32,
            gpu.surface_config.height as f32,
        );
        let camera_uniform = CameraUniform::new(&camera);
        let camera_controller = CameraController::new(camera, 2.0, 1e5);

        /***************
         * Objects
         ***************/
        let earth = PlanetPipeline::new(EARTH_RADIUS, &gpu, &camera_uniform);
        let frame = FramePipeline::new(&gpu, &camera_uniform);

        Self {
            camera_uniform: camera_uniform,
            camera_controller: camera_controller,
            earth: earth,
            frame: frame,
            // square: square,
            simulation_speed: simulation_speed,
            simulation_time: Instant::now(),
            last_tick: Instant::now(),
        }
    }

    pub fn tick(&mut self) {
        let current_tick = Instant::now();
        let elapsed_real_time = current_tick - self.last_tick;
        let elpased_simulation_time = elapsed_real_time.mul_f64(self.simulation_speed);

        self.simulation_time = self.simulation_time + elpased_simulation_time;
        self.last_tick = current_tick;
    }

    // Used by the application to update the simulation
    pub fn update(&mut self, event: &WindowEvent, gpu: &Gpu) -> SimulationUpdateResult {
        debug!("Update simulation");
        match self.camera_controller.update_camera(event) {
            SimulationUpdateResult::RequiresRedraw => {
                self.camera_uniform
                    .update_view_proj(&self.camera_controller.camera);
                self.earth.update_camera(gpu, &self.camera_uniform);
                self.frame.update_camera(gpu, &self.camera_uniform);
                return SimulationUpdateResult::RequiresRedraw;
            }
            _ => SimulationUpdateResult::Done,
        }
    }

    // Used by the application to re-render the simulation
    pub fn render(&self, gpu: &Gpu) -> Result<(), wgpu::SurfaceError> {
        debug!("Render simulation");
        let output = gpu.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            self.earth.render(&mut render_pass);
            self.frame.render(&mut render_pass);
        }
        gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}

#[cfg(test)]
mod simulation_tests {

    #[test]
    fn test_tick() {
        // /* Setup */
        // let mut simulation = Simulation::new(Universe::new(), 5.0);
        // let simulation_time = simulation.simulation_time;
        // let last_tick = simulation.last_tick;

        // /* Run */
        // simulation.tick();

        // /* Test */
        // assert_that!(simulation.simulation_time.duration_since(simulation_time))
        //     .is_greater_than(simulation.last_tick.duration_since(last_tick).mul_f64(4.0));
    }
}
