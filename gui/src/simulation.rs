use crate::{
    gpu::pipelines::planet::{camera::Camera, pipeline::Pipeline, satellite::SatelliteRenderMode},
    model::system::{EARTH_RADIUS_KM, System as CoreSystem},
};
use chrono::Utc;
use iced::{Rectangle, mouse, wgpu, widget::shader};
use log::{debug, info};
use nalgebra::{Isometry3, Point3, Point4, Rotation3, Translation3, Vector3};

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

#[derive(Debug)]
pub struct Simulation {
    pub system: CoreSystem,
    pub camera: Camera,
    pub satellite_mode: SatelliteRenderMode,
    pub frame_mode: FrameMode,
    pub ecef_reference_earth_angle: f32,
    pub paused: bool,
    pub time_scale: f32,
    pub pick_radius_scale: f32,
}

impl Simulation {
    pub fn earth_rotation_phase(&self) -> f32 {
        self.system.earth_rotation() as f32
    }

    pub fn elapsed_time(&self) -> f32 {
        self.system.elapsed_seconds()
    }

    pub fn set_time_scale(&mut self, new_scale: f32) {
        let clamped = new_scale.clamp(0.1, 50_000.0);
        self.system.simulation_speed = clamped.max(1.0).round() as i32;
        self.time_scale = clamped;
    }

    pub fn toggle_pause(&mut self) {
        if self.paused {
            self.system.last_tick_time = Utc::now();
            self.paused = false;
        } else {
            self.paused = true;
        }
    }

    pub fn reset_time(&mut self) {
        let now = Utc::now();
        self.system.simulation_time = now;
        self.system.start_time = now;
        self.system.last_tick_time = now;
        self.ecef_reference_earth_angle = 0.0;
        self.paused = false;
    }

    pub fn tick(&mut self) {
        let phase_before = self.earth_rotation_phase();
        self.system.tick();
        let phase_after = self.earth_rotation_phase();

        if self.frame_mode == FrameMode::Ecef {
            let world_delta = (phase_after - phase_before).rem_euclid(std::f32::consts::TAU);
            let camera_delta = -world_delta;
            if camera_delta.abs() > f32::EPSILON {
                let rot = Rotation3::from_axis_angle(&Vector3::z_axis(), camera_delta);
                let iso = Isometry3::from_parts(Translation3::identity(), rot.into());
                self.camera.transform(&iso);
            }

            self.ecef_reference_earth_angle = phase_after;
        }
    }

    fn frame_adjusted_camera(&self, width: f32, height: f32, _earth_phase: f32) -> Camera {
        let mut camera = self.camera.clone();
        camera.change_aspect(width, height);
        camera
    }

    /// Project a world space point into screen pixel coordinates.
    ///
    /// - `world_pos` is in the same coordinate frame as the current program frame mode.
    /// - `viewport_size` is in pixels (width, height).
    ///
    /// Returns `Some((x_px, y_px))` when the point can be projected and is in front of camera projection,
    /// or `None` when the point is behind the camera or the viewport is invalid.
    pub fn world_to_screen(
        &self,
        world_pos: Point3<f32>,
        viewport_size: (f32, f32),
    ) -> Option<(f32, f32)> {
        let (width, height) = viewport_size;
        if width <= 0.0 || height <= 0.0 {
            return None;
        }

        let camera = self.frame_adjusted_camera(width, height, self.earth_rotation_phase());
        let view_proj = camera.build_view_projection_matrix();

        let world_point = Point4::new(world_pos.x, world_pos.y, world_pos.z, 1.0);
        let clip = view_proj * world_point;
        if clip.w.abs() < f32::EPSILON {
            return None;
        }

        let ndc_x = clip.x / clip.w;
        let ndc_y = clip.y / clip.w;
        let ndc_z = clip.z / clip.w;

        // Keep points in front of the near plane and inside [-1,1] clip range.
        if ndc_z < -1.0 || ndc_z > 1.0 {
            return None;
        }

        let pixel_x = (ndc_x * 0.5 + 0.5) * width;
        let pixel_y = (1.0 - (ndc_y * 0.5 + 0.5)) * height;

        Some((pixel_x, pixel_y))
    }

    pub fn world_ray_from_cursor(
        &self,
        cursor: (f32, f32),
        viewport_size: (f32, f32),
    ) -> Option<(Point3<f32>, Vector3<f32>, (f32, f32))> {
        let (width, height) = viewport_size;
        if width <= 0.0 || height <= 0.0 {
            return None;
        }

        let ndc_x = ((cursor.0 + 0.5) / width) * 2.0 - 1.0;
        let ndc_y = 1.0 - ((cursor.1 + 0.5) / height) * 2.0;

        let camera = self.frame_adjusted_camera(width, height, self.earth_rotation_phase());
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
        Some((near_point, direction, (ndc_x, ndc_y)))
    }

    pub fn pick_object(
        &self,
        origin: Point3<f32>,
        direction: Vector3<f32>,
        _cursor_ndc: (f32, f32),
        _viewport_size: (f32, f32),
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

            let screen_position = self.world_to_screen(center, _viewport_size);
            debug!(
                "pick info resource={:?} cursor_ndc={:?} viewport={:?} world_center={:?} screen_pos={:?}",
                obj, _cursor_ndc, _viewport_size, center, screen_position,
            );

            let camera_to_object = to_center / depth;
            let angle = (direction.dot(&camera_to_object).clamp(-1.0, 1.0).acos()).to_degrees();
            let angular_radius = (radius / depth).asin().to_degrees() * pick_radius_scale;
            info!(
                "resource {:?} angle {} radius {}",
                obj, angle, angular_radius
            );

            if angle <= angular_radius {
                if angle < best_hit.1 || (angle == best_hit.1 && depth < best_hit.2) {
                    best_hit = (obj, angle, depth);
                }
            }
        };

        consider_object(SelectedObject::Earth, Point3::origin(), EARTH_RADIUS_KM);

        let elapsed = self.elapsed_time();
        let earth_rotation_angle = self.system.earth_rotation() as f32;
        // The WGSL station_shader uses column-major earth_rotation(θ) which evaluates to
        // x'=cθ·x+sθ·y, y'=-sθ·x+cθ·y — that is Rz(-θ). Negate here to match.
        let ecef_to_eci = Rotation3::from_axis_angle(&Vector3::z_axis(), -earth_rotation_angle);

        let dot_radius = EARTH_RADIUS_KM * CoreSystem::SATELLITE_SCALE_FACTOR;
        let satellite_radius = match self.satellite_mode {
            SatelliteRenderMode::Dot => dot_radius,
            SatelliteRenderMode::Cube => dot_radius * 0.25,
        };

        for (orbit_index, orbit) in self.system.orbits.iter().enumerate() {
            for sat in orbit.satellites.iter() {
                let pos_eci = orbit.position(elapsed, sat);
                let center = Point3::new(pos_eci[0], pos_eci[1], pos_eci[2]);
                consider_object(
                    SelectedObject::Satellite(format!("{}:{}", orbit_index, sat.name)),
                    center,
                    satellite_radius,
                );
            }
        }

        for station in &self.system.ground_stations {
            let cart: [f32; 3] = station.cartesian();
            let center_ecef = Vector3::new(cart[0], cart[1], cart[2]);
            let center_eci = ecef_to_eci * center_ecef;
            let center = Point3::new(center_eci.x, center_eci.y, center_eci.z);

            // Model cube vertices are in [-0.1, 0.1], so bounding-sphere radius scales by 0.1*sqrt(3).
            let station_radius = (0.173_205_08_f32 * station.cube_size).max(25.0_f32);
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

impl<Message> shader::Program<Message> for Simulation {
    type State = String;

    type Primitive = Primitive;

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: mouse::Cursor,
        bounds: iced::Rectangle,
    ) -> Self::Primitive {
        // TODO compute next frame date here from simulation then update camera
        // if necessary and pass this date to render pipeline
        let elapsed = self.elapsed_time();
        let earth_phase = self.earth_rotation_phase();

        let camera = self.frame_adjusted_camera(bounds.width, bounds.height, earth_phase);

        Primitive {
            system: self.system.clone(),
            camera,
            elapsed,
            earth_rotation_angle: earth_phase,
            satellite_mode: self.satellite_mode,
        }
    }
}

#[derive(Debug)]
pub struct Primitive {
    system: CoreSystem,
    camera: Camera,
    elapsed: f32,
    earth_rotation_angle: f32,
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
            &self.system,
            &self.camera,
            self.elapsed,
            self.earth_rotation_angle,
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
