use gui::{
    gpu::pipelines::planet::{camera::Camera, pipeline::Pipeline, satellite::SatelliteRenderMode},
    model::simulation::Simulation,
};
use iced::{Rectangle, mouse, wgpu, widget::shader};
use nalgebra::{Point3, Point4, Vector3};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub enum SelectedObject {
    Earth,
    Satellite(String),
    GroundStation(String),
    None,
}

pub struct Program {
    pub model: Arc<Simulation>,
    pub camera: Camera,
    pub start_time: std::time::Instant,
    pub satellite_mode: SatelliteRenderMode,
    pub paused: bool,
    pub paused_elapsed: f32,
    pub pick_radius_scale: f32,
}

impl Program {
    pub fn elapsed_time(&self) -> f32 {
        if self.paused {
            self.paused_elapsed
        } else {
            self.paused_elapsed + self.start_time.elapsed().as_secs_f32()
        }
    }

    pub fn toggle_pause(&mut self) {
        if self.paused {
            self.start_time = std::time::Instant::now();
            self.paused = false;
        } else {
            self.paused_elapsed = self.elapsed_time();
            self.paused = true;
        }
    }

    pub fn reset_time(&mut self) {
        self.start_time = std::time::Instant::now();
        self.paused_elapsed = 0.0;
        self.paused = false;
    }

    pub fn world_ray_from_cursor(
        &self,
        cursor: (f32, f32),
        viewport_size: (f32, f32),
    ) -> Option<(Point3<f32>, Vector3<f32>)> {
        let (width, height) = viewport_size;
        if width <= 0.0 || height <= 0.0 {
            return None;
        }

        let ndc_x = ((cursor.0 + 0.5) / width) * 2.0 - 1.0;
        let ndc_y = 1.0 - ((cursor.1 + 0.5) / height) * 2.0;

        let mut camera = self.camera.clone();
        camera.change_aspect(width, height);
        let view_proj = camera.build_view_projection_matrix();
        let inv = view_proj.try_inverse()?;

        let near_clip = Point4::new(ndc_x, ndc_y, -1.0, 1.0);
        let far_clip = Point4::new(ndc_x, ndc_y, 1.0, 1.0);

        let world_near = inv * near_clip;
        let world_far = inv * far_clip;

        if world_near.w.abs() < f32::EPSILON || world_far.w.abs() < f32::EPSILON {
            return None;
        }

        let near_point = Point3::new(
            world_near.x / world_near.w,
            world_near.y / world_near.w,
            world_near.z / world_near.w,
        );

        let far_point = Point3::new(
            world_far.x / world_far.w,
            world_far.y / world_far.w,
            world_far.z / world_far.w,
        );

        let direction = (far_point - near_point).normalize();
        Some((near_point, direction))
    }

    pub fn pick_object(
        &self,
        origin: Point3<f32>,
        direction: Vector3<f32>,
    ) -> (SelectedObject, Option<f32>) {
        let pick_radius_scale = self.pick_radius_scale;
        let mut best_hit: (SelectedObject, f32, f32) =
            (SelectedObject::None, f32::INFINITY, f32::INFINITY);

        let camera_pos = origin;

        let mut consider_object = |obj: SelectedObject, center: Point3<f32>, radius: f32| {
            let to_center = center - camera_pos;
            let depth = to_center.norm();
            if depth <= 0.0 {
                return;
            }

            let camera_to_object = to_center / depth;
            let angle = (direction.dot(&camera_to_object).clamp(-1.0, 1.0).acos()).to_degrees();
            let angular_radius = (radius / depth).asin().to_degrees() * pick_radius_scale;

            if angle <= angular_radius {
                if angle < best_hit.1 || (angle == best_hit.1 && depth < best_hit.2) {
                    best_hit = (obj, angle, depth);
                }
            }
        };

        consider_object(SelectedObject::Earth, Point3::origin(), 1.0);

        let satellite_radius = 0.08;
        let station_radius = 0.1;

        for (orbit_index, orbit) in self.model.orbits.iter().enumerate() {
            for sat in orbit.satellites.iter() {
                let pos = orbit.position(self.elapsed_time(), sat);
                let center = Point3::new(pos[0], pos[1], pos[2]);
                consider_object(
                    SelectedObject::Satellite(format!("{}:{}", orbit_index, sat.name)),
                    center,
                    satellite_radius,
                );
            }
        }

        for station in &self.model.ground_stations {
            let cart = station.cartesian();
            let center = Point3::new(cart[0], cart[1], cart[2]);
            consider_object(
                SelectedObject::GroundStation(station.name.clone()),
                center,
                station_radius,
            );
        }

        if matches!(best_hit.0, SelectedObject::None) {
            (SelectedObject::None, None)
        } else {
            (best_hit.0, Some(best_hit.2))
        }
    }
}

impl<Message> shader::Program<Message> for Program {
    type State = String;

    type Primitive = Primitive;

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        bounds: iced::Rectangle,
    ) -> Self::Primitive {
        let mut camera = self.camera.clone();
        camera.change_aspect(bounds.width, bounds.height);

        Primitive {
            model: Arc::clone(&self.model),
            camera,
            elapsed: self.elapsed_time(),
            satellite_mode: self.satellite_mode,
        }
    }
}

#[derive(Debug)]
pub struct Primitive {
    model: Arc<Simulation>,
    camera: Camera,
    elapsed: f32,
    satellite_mode: SatelliteRenderMode,
}

impl shader::Primitive for Primitive {
    type Pipeline = Pipeline;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &iced::wgpu::Device,
        queue: &iced::wgpu::Queue,
        bounds: &iced::Rectangle,
        viewport: &shader::Viewport,
    ) {
        pipeline.prepare(
            device,
            queue,
            bounds,
            viewport,
            &self.model,
            &self.camera,
            self.elapsed,
            self.satellite_mode,
        );
    }

    fn render(
        &self,
        pipeline: &Pipeline,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        pipeline.render(encoder, target, clip_bounds);
    }
}
