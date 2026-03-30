use gui::{
    astro::Astral,
    gpu::pipelines::planet::{camera::Camera, pipeline::Pipeline, satellite::SatelliteRenderMode},
    model::simulation::Simulation,
};
use iced::{Rectangle, mouse, wgpu, widget::shader};
use nalgebra::{Point3, Point4, Rotation3, Vector3};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub enum SelectedObject {
    Earth,
    Satellite(String),
    GroundStation(String),
    None,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FrameMode {
    Eci,
    Ecef,
}

pub struct Program {
    pub model: Arc<Simulation>,
    pub camera: Camera,
    pub start_time: std::time::Instant,
    pub satellite_mode: SatelliteRenderMode,
    pub frame_mode: FrameMode,
    pub paused: bool,
    pub paused_elapsed: f32,
    pub time_scale: f32,
    pub pick_radius_scale: f32,
}

impl Program {
    pub fn elapsed_time(&self) -> f32 {
        if self.paused {
            self.paused_elapsed
        } else {
            self.paused_elapsed + self.start_time.elapsed().as_secs_f32() * self.time_scale
        }
    }

    pub fn set_time_scale(&mut self, new_scale: f32) {
        let clamped = new_scale.clamp(0.1, 50_000.0);
        let elapsed = self.elapsed_time();
        self.paused_elapsed = elapsed;
        self.start_time = std::time::Instant::now();
        self.time_scale = clamped;
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

    fn earth_rotation_angle(&self) -> f32 {
        let elapsed_secs = self.elapsed_time() as f64;
        let day_of_year = 172 + ((elapsed_secs / 86400.0) as u32 % 365);
        let hour = (elapsed_secs / 3600.0) % 24.0;
        Astral::earth_rotation_angle(day_of_year, hour) as f32
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

        consider_object(
            SelectedObject::Earth,
            Point3::origin(),
            gui::model::simulation::EARTH_RADIUS_KM,
        );

        let elapsed = self.elapsed_time();
        let earth_rotation_angle = self.earth_rotation_angle();
        let ecef_to_eci = Rotation3::from_axis_angle(&Vector3::z_axis(), earth_rotation_angle);
        let eci_to_ecef = Rotation3::from_axis_angle(&Vector3::z_axis(), -earth_rotation_angle);

        let dot_radius = gui::model::simulation::EARTH_RADIUS_KM
            * gui::model::simulation::Simulation::SATELLITE_SCALE_FACTOR;
        let satellite_radius = match self.satellite_mode {
            SatelliteRenderMode::Dot => dot_radius,
            SatelliteRenderMode::Cube => dot_radius * 0.25,
        };

        for (orbit_index, orbit) in self.model.orbits.iter().enumerate() {
            for sat in orbit.satellites.iter() {
                let pos_eci = orbit.position(elapsed, sat);
                let center_eci = Vector3::new(pos_eci[0], pos_eci[1], pos_eci[2]);
                let center = match self.frame_mode {
                    FrameMode::Eci => Point3::new(center_eci.x, center_eci.y, center_eci.z),
                    FrameMode::Ecef => {
                        let v = eci_to_ecef * center_eci;
                        Point3::new(v.x, v.y, v.z)
                    }
                };
                consider_object(
                    SelectedObject::Satellite(format!("{}:{}", orbit_index, sat.name)),
                    center,
                    satellite_radius,
                );
            }
        }

        for station in &self.model.ground_stations {
            let cart = station.cartesian();
            let center_ecef = Vector3::new(cart[0], cart[1], cart[2]);
            let center = match self.frame_mode {
                FrameMode::Eci => {
                    let v = ecef_to_eci * center_ecef;
                    Point3::new(v.x, v.y, v.z)
                }
                FrameMode::Ecef => Point3::new(center_ecef.x, center_ecef.y, center_ecef.z),
            };

            // Model cube vertices are in [-0.1, 0.1], so bounding-sphere radius scales by 0.1*sqrt(3).
            let station_radius = (0.173_205_08 * station.cube_size).max(25.0);
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
            frame_mode: self.frame_mode,
        }
    }
}

#[derive(Debug)]
pub struct Primitive {
    model: Arc<Simulation>,
    camera: Camera,
    elapsed: f32,
    satellite_mode: SatelliteRenderMode,
    frame_mode: FrameMode,
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
        let frame_mode_u32 = match self.frame_mode {
            FrameMode::Eci => 0,
            FrameMode::Ecef => 1,
        };

        pipeline.prepare(
            device,
            queue,
            bounds,
            viewport,
            &self.model,
            &self.camera,
            self.elapsed,
            self.satellite_mode,
            frame_mode_u32,
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
